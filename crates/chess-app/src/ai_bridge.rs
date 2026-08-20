//! Non-blocking AI move computation.
//!
//! When it becomes the AI's turn, [`request_ai_move`] launches a task on the
//! shared Tokio runtime and stores a receiver. [`poll_ai_move`] checks the
//! receiver each frame and applies the move when ready — all without ever
//! blocking the Bevy schedule.
//!
//! Additionally, real-time search information (depth, score, PV, nodes) is
//! streamed via [`SearchInfoResource`] so the GUI can display evaluation bars
//! and analysis details.
//!
//! With an external engine configured, a **persistent** UCI process is kept
//! alive for the whole session ([`EngineSession`]) instead of launching a new
//! process per move, so the engine's transposition table carries over between
//! moves. On top of that the bridge speaks the UCI *ponder* protocol: right
//! after the AI moves, the engine starts pondering the reply it predicts from
//! the human (`go ponder`), turning the human's own thinking time into extra
//! search depth at zero UX cost.

use bevy::prelude::*;
use chess_ai::{Ai, Difficulty, SearchInfo, SearchLimits, UciConfig, UciEngine};
use chess_core::{Color as ChessColor, Move};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::app_state::{AiSettings, CoreGame, GameMode};
use crate::history_view::HistoryView;
use crate::moves::apply_local_move;
use crate::sound::{MoveSound, PendingSound};

/// Result payload of one move computation: the best move plus the engine's
/// predicted reply (used to set up the next ponder search).
type MoveReply = (Option<Move>, Option<Move>);

/// Holds the in-flight AI computation, if any.
#[derive(Resource, Default)]
pub struct AiTask {
    pub rx: Option<Receiver<MoveReply>>,
    pub info_rx: Option<Receiver<SearchInfo>>,
    /// FEN the in-flight computation was issued for. If the board no longer
    /// matches when the result arrives (undo / restart / load mid-search),
    /// the move is stale and must be discarded.
    expected_fen: String,
    /// Active ponder search on the engine side, if any.
    ponder: Option<PonderTrack>,
}

/// Tracks an in-flight ponder search: the engine is searching the position
/// after the human reply it predicted.
struct PonderTrack {
    /// FEN of the position the human is thinking in (before the reply).
    base_fen: String,
    /// FEN after the predicted reply — a ponder hit when the board reaches it.
    predicted_fen: String,
}

/// Resource holding the latest search info for GUI display.
#[derive(Resource, Default)]
pub struct SearchInfoResource {
    /// Latest search info (updated during AI thinking).
    pub latest: Option<SearchInfo>,
    /// Whether AI is currently thinking.
    pub thinking: bool,
}

impl SearchInfoResource {
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.latest = None;
        self.thinking = false;
    }
}

/// Build the UCI engine config from the settings, or `None` to signal the
/// built-in fallback. Shared by the game search and the analysis-mode
/// background search.
///
/// Thread/hash budgets come from [`engine_tuning`]: modest for casual tiers,
/// the whole machine for [`Difficulty::Extreme`].
pub fn engine_config(settings: &AiSettings) -> Option<UciConfig> {
    let path = settings.engine_path.clone()?;
    let mut cfg = UciConfig::new(path);
    if let Some(ev) = &settings.eval_file {
        cfg = cfg.with_option("EvalFile", ev.to_string_lossy().to_string());
    }
    let (threads, hash) = engine_tuning(settings.difficulty);
    Some(
        cfg.with_option("Threads", threads.to_string())
            .with_option("Hash", hash.to_string()),
    )
}

/// Commands sent to the persistent engine task (processed strictly in order).
enum EngineCmd {
    /// Compute a move for the game described by `fen` (the game's *initial*
    /// position) plus `history`. Stops and discards any in-flight ponder.
    Compute {
        fen: String,
        history: Vec<Move>,
        stm: ChessColor,
        movetime: Duration,
        info_tx: Sender<SearchInfo>,
        reply: Sender<MoveReply>,
    },
    /// Start pondering the predicted human reply: `history` already ends
    /// with that predicted move. The engine searches until told otherwise.
    StartPonder {
        fen: String,
        history: Vec<Move>,
        movetime: Duration,
    },
    /// The human played the predicted reply: convert the ponder into a real
    /// search (`ponderhit`) and report the resulting move.
    PonderHit {
        stm: ChessColor,
        info_tx: Sender<SearchInfo>,
        reply: Sender<MoveReply>,
    },
    /// Stop any ponder and idle (board left the predicted track: undo,
    /// restart, game over, mode switch).
    Abort,
    /// Fresh game: stop everything and clear engine state (`ucinewgame`).
    NewGame,
    /// Relaunch the process with new tuning (difficulty switch; Threads/Hash
    /// are process-level options).
    Relaunch(Box<UciConfig>),
}

/// Handle to the persistent engine task, if running.
#[derive(Resource, Default)]
pub struct EngineSession {
    cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<EngineCmd>>,
    /// Set when the engine process dies or fails to launch; the bridge then
    /// permanently falls back to the per-move/built-in path.
    failed: Arc<AtomicBool>,
    /// Difficulty the running process was launched with.
    tuned_for: Option<Difficulty>,
    /// Whether we have seen a non-empty move history; used to detect the
    /// transition back to move 0 (restart/rematch) and send `NewGame`.
    saw_moves: bool,
}

/// Long-lived task owning one UCI engine process.
///
/// While pondering, the task keeps draining stdout (discarding `info` lines —
/// they describe the predicted position, not the one on screen) so the pipe
/// never fills, and stays ready to convert or abort the ponder on command.
async fn persistent_engine_task(
    config: UciConfig,
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<EngineCmd>,
    failed: Arc<AtomicBool>,
) {
    let mut engine = match UciEngine::launch(&config).await {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "persistent engine failed to launch");
            failed.store(true, Ordering::Relaxed);
            return;
        }
    };
    tracing::info!("persistent UCI engine ready");

    let mut pondering = false;
    // A bestmove that arrived spontaneously during ponder (engines are not
    // supposed to stop a ponder search, but handle it just in case).
    let mut ponder_result: Option<MoveReply> = None;

    loop {
        tokio::select! {
            line = engine.next_line(), if pondering => {
                match line {
                    Ok(l) => {
                        if let Some((mv, pm)) = chess_ai::uci::parse_bestmove_line(&l) {
                            pondering = false;
                            ponder_result = Some((Some(mv), pm));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "engine died mid-ponder");
                        failed.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            }
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break }; // app shutting down
                match cmd {
                    EngineCmd::Compute { fen, history, stm, movetime, info_tx, reply } => {
                        if pondering {
                            pondering = false;
                            tracing::debug!("ponder miss: stopping ponder, computing fresh");
                            stop_and_drain(&mut engine).await;
                        }
                        ponder_result = None;
                        let result = run_compute(&mut engine, &fen, &history, stm, movetime, info_tx).await;
                        if result.is_err() {
                            failed.store(true, Ordering::Relaxed);
                            let _ = reply.send((None, None));
                            return;
                        }
                        let _ = reply.send(result.unwrap());
                    }
                    EngineCmd::StartPonder { fen, history, movetime } => {
                        if pondering {
                            stop_and_drain(&mut engine).await;
                        }
                        ponder_result = None;
                        pondering = false;
                        if engine.set_position(&fen, &history).await.is_ok()
                            && engine.go_movetime(movetime, true).await.is_ok()
                        {
                            pondering = true;
                            tracing::debug!("ponder started");
                        }
                    }
                    EngineCmd::PonderHit { stm, info_tx, reply } => {
                        tracing::debug!(cached = ponder_result.is_some(), "ponder hit");
                        let result = if let Some(r) = ponder_result.take() {
                            pondering = false;
                            r
                        } else if pondering {
                            pondering = false;
                            match engine.ponder_hit().await {
                                Ok(()) => match engine
                                    .wait_bestmove(stm, std::time::Instant::now(), Some(info_tx))
                                    .await
                                {
                                    Ok((mv, ponder, _)) => (Some(mv), ponder),
                                    Err(e) => {
                                        tracing::warn!(error = %e, "ponderhit conversion failed");
                                        (None, None)
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!(error = %e, "ponderhit failed");
                                    (None, None)
                                }
                            }
                        } else {
                            (None, None)
                        };
                        let _ = reply.send(result);
                    }
                    EngineCmd::Abort => {
                        if pondering {
                            pondering = false;
                            stop_and_drain(&mut engine).await;
                        }
                        ponder_result = None;
                    }
                    EngineCmd::NewGame => {
                        if pondering {
                            pondering = false;
                            stop_and_drain(&mut engine).await;
                        }
                        ponder_result = None;
                        if let Err(e) = engine.new_game().await {
                            tracing::warn!(error = %e, "ucinewgame failed");
                            failed.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                    EngineCmd::Relaunch(cfg) => {
                        if pondering {
                            pondering = false;
                            stop_and_drain(&mut engine).await;
                        }
                        ponder_result = None;
                        let _ = engine.quit().await;
                        match UciEngine::launch(&cfg).await {
                            Ok(e) => engine = e,
                            Err(e) => {
                                tracing::warn!(error = %e, "engine relaunch failed");
                                failed.store(true, Ordering::Relaxed);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = engine.quit().await;
}

/// Stop a ponder search and drain until its `bestmove` (discarded) so the
/// protocol stays in sync.
async fn stop_and_drain(engine: &mut UciEngine) {
    if engine.stop().await.is_err() {
        return;
    }
    let _ = engine
        .wait_bestmove(ChessColor::Red, std::time::Instant::now(), None)
        .await;
}

/// One full move computation on the persistent engine.
async fn run_compute(
    engine: &mut UciEngine,
    fen: &str,
    history: &[Move],
    stm: ChessColor,
    movetime: Duration,
    info_tx: Sender<SearchInfo>,
) -> Result<MoveReply, ()> {
    let result = async {
        engine.set_position(fen, history).await?;
        engine.go_movetime(movetime, false).await?;
        engine
            .wait_bestmove(stm, std::time::Instant::now(), Some(info_tx))
            .await
    }
    .await;
    match result {
        Ok((mv, ponder, _)) => Ok((Some(mv), ponder)),
        Err(e) => {
            tracing::warn!(error = %e, "engine compute failed");
            Err(())
        }
    }
}

/// Per-difficulty engine resources. Lower tiers stay modest (a few threads,
/// a small transposition table) so casual play stays responsive; the top
/// tier hands Pikafish the whole machine — every core plus a large TT —
/// which is where most of its extra strength comes from at long movetimes.
fn engine_tuning(d: Difficulty) -> (usize, u32) {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    match d {
        Difficulty::Extreme => (cores.clamp(1, 16), 256),
        _ => ((cores / 2).clamp(1, 4), 32),
    }
}

/// If it is the AI's turn and no task is running, start one.
pub fn request_ai_move(
    core: Res<CoreGame>,
    settings: Res<AiSettings>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
    mut session: ResMut<EngineSession>,
    mut task: ResMut<AiTask>,
    mut search_info: ResMut<SearchInfoResource>,
) {
    if core.mode != GameMode::VsAi || core.game.is_over() {
        return;
    }
    if core.local_to_move() || task.rx.is_some() {
        return; // human's turn, or a search is already running
    }

    let board = core.game.board().clone();
    let fen = board.to_fen();
    let limits: SearchLimits = settings.difficulty.limits();
    let use_book = settings.difficulty.uses_book();

    // Preferred path: the persistent engine process.
    if let Some(cfg) = engine_config(&settings) {
        if !session.failed.load(Ordering::Relaxed) {
            // Ponder hit: the human played exactly the reply the engine is
            // pondering — convert that search instead of starting cold.
            let ponder_hit = matches!(&task.ponder, Some(p) if p.predicted_fen == fen);

            // Book shortcut, only for cold computes: a ponder hit already has
            // the engine mid-search on this exact position.
            if !ponder_hit && use_book {
                if let Some(book_mv) = chess_ai::book_move(&board) {
                    tracing::info!(mv = %book_mv.to_iccs(), "book move");
                    let (tx, rx) = crossbeam_channel::bounded(1);
                    let _ = tx.send((Some(book_mv), None));
                    task.rx = Some(rx);
                    task.info_rx = None;
                    task.expected_fen = fen;
                    search_info.thinking = true;
                    search_info.latest = None;
                    return;
                }
            }

            ensure_engine_session(&runtime, &mut session, &cfg, settings.difficulty);
            if let Some(cmd_tx) = session.cmd_tx.clone() {
                let (reply_tx, reply_rx) = crossbeam_channel::bounded(1);
                // Bounded channel for search info: GUI drains frequently,
                // 4 slots prevent overflow.
                let (info_tx, info_rx) = crossbeam_channel::bounded(4);
                let stm = board.side_to_move();
                let cmd = if ponder_hit {
                    task.ponder = None;
                    EngineCmd::PonderHit {
                        stm,
                        info_tx,
                        reply: reply_tx,
                    }
                } else {
                    // Fresh compute (a ponder miss stops the ponder inside the
                    // task). Full history from the game's initial position so
                    // the engine sees repetitions and keeps its TT warm.
                    task.ponder = None;
                    let start_fen = core
                        .game
                        .board_at_ply(0)
                        .map(|b| b.to_fen())
                        .unwrap_or_else(|| fen.clone());
                    let history: Vec<Move> = core.game.played_moves().collect();
                    EngineCmd::Compute {
                        fen: start_fen,
                        history,
                        stm,
                        movetime: limits.movetime,
                        info_tx,
                        reply: reply_tx,
                    }
                };
                if cmd_tx.send(cmd).is_ok() {
                    task.rx = Some(reply_rx);
                    task.info_rx = Some(info_rx);
                    task.expected_fen = fen;
                    search_info.thinking = true;
                    search_info.latest = None;
                    return;
                }
                // Task is gone: mark the session failed and fall through to
                // the per-move fallback below.
                session.failed.store(true, Ordering::Relaxed);
                session.cmd_tx = None;
            }
        }
    }

    // Fallback path: per-move engine process, or the built-in engine.
    let engine_cfg = engine_config(&settings);
    let (tx, rx) = crossbeam_channel::bounded(1);
    let (info_tx, info_rx) = crossbeam_channel::bounded(4);
    task.rx = Some(rx);
    task.info_rx = Some(info_rx);
    task.expected_fen = fen;
    search_info.thinking = true;
    search_info.latest = None;

    let rt = runtime.0.clone();
    rt.spawn(async move {
        let mut ai = match engine_cfg {
            Some(cfg) => Ai::pikafish(&cfg).await,
            None => Ai::builtin(),
        };
        let mv = ai
            .best_move_with_info(&board, &[], limits, use_book, Some(info_tx))
            .await;
        let _ = tx.send((mv, None));
    });
}

/// Spawn the persistent engine task if needed.
fn ensure_engine_session(
    runtime: &crate::async_runtime::AsyncRuntime,
    session: &mut EngineSession,
    cfg: &UciConfig,
    difficulty: Difficulty,
) {
    if session.cmd_tx.is_some() {
        return;
    }
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let failed = session.failed.clone();
    let cfg = cfg.clone();
    runtime
        .0
        .spawn(async move { persistent_engine_task(cfg, cmd_rx, failed).await });
    session.cmd_tx = Some(cmd_tx);
    session.tuned_for = Some(difficulty);
}

/// Housekeeping for the persistent engine session:
///
/// * relaunches the process when the difficulty changed (Threads/Hash are
///   process-level options);
/// * sends `NewGame` when a restart/rematch resets the move history;
/// * aborts the ponder search when the board leaves the predicted track
///   (undo, game over, leaving VsAi).
pub fn engine_session_tick(
    core: Res<CoreGame>,
    settings: Res<AiSettings>,
    mut session: ResMut<EngineSession>,
    mut task: ResMut<AiTask>,
) {
    let Some(cmd_tx) = session.cmd_tx.clone() else {
        return;
    };

    if session.tuned_for != Some(settings.difficulty) {
        if let Some(cfg) = engine_config(&settings) {
            session.tuned_for = Some(settings.difficulty);
            task.ponder = None;
            let _ = cmd_tx.send(EngineCmd::Relaunch(Box::new(cfg)));
        }
    }

    // Fresh game detection: history went from non-empty back to zero.
    let empty = core.game.history_len() == 0;
    if empty && session.saw_moves {
        session.saw_moves = false;
        let _ = cmd_tx.send(EngineCmd::NewGame);
        task.ponder = None;
    } else if !empty {
        session.saw_moves = true;
    }

    // Ponder watchdog.
    if let Some(p) = &task.ponder {
        let fen = core.game.board().to_fen();
        let off_track = core.mode != GameMode::VsAi
            || core.game.is_over()
            || (fen != p.base_fen && fen != p.predicted_fen);
        if off_track {
            task.ponder = None;
            let _ = cmd_tx.send(EngineCmd::Abort);
        }
    }
}

/// Drain any pending search info updates into the resource.
pub fn poll_search_info(task: Res<AiTask>, mut search_info: ResMut<SearchInfoResource>) {
    if let Some(ref info_rx) = task.info_rx {
        // Drain all pending updates, keeping only the latest.
        while let Ok(info) = info_rx.try_recv() {
            search_info.latest = Some(info);
        }
    }
}

/// Apply the AI's move once the task finishes.
#[allow(clippy::too_many_arguments)]
pub fn poll_ai_move(
    mut task: ResMut<AiTask>,
    session: ResMut<EngineSession>,
    settings: Res<AiSettings>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut pending_sound: ResMut<PendingSound>,
    mut history_view: ResMut<HistoryView>,
    mut search_info: ResMut<SearchInfoResource>,
) {
    let Some(rx) = task.rx.as_ref() else {
        return;
    };
    match rx.try_recv() {
        Ok((Some(mv), ponder_mv)) => {
            // Discard stale results: the board moved on while computing
            // (undo / restart / load mid-search).
            if core.game.board().to_fen() != task.expected_fen {
                warn!("discarding stale AI move (board changed during search)");
                task.rx = None;
                task.info_rx = None;
                search_info.thinking = false;
                return;
            }

            // Detect capture before the move.
            let is_capture = core.game.board().piece_at(mv.to).is_some();

            apply_local_move(&mut core, mv);
            history_view.return_to_live();

            // Detect check after the move.
            let is_check = core.game.board().is_in_check(core.game.side_to_move());

            task.rx = None;
            task.info_rx = None;
            search_info.thinking = false;
            dirty.0 = true;

            let moved_piece = core.game.board().piece_at(mv.to).map(|p| p.kind);
            pending_sound.sound = Some(if is_check {
                MoveSound::Check
            } else if is_capture {
                MoveSound::Capture
            } else {
                MoveSound::Normal
            });
            pending_sound.piece = moved_piece;

            // Hand the engine's predicted reply to the persistent session so
            // it ponders while the human thinks.
            maybe_start_ponder(&mut task, &session, &settings, &core, ponder_mv);
        }
        Ok((None, _)) => {
            warn!("AI produced no move");
            task.rx = None;
            task.info_rx = None;
            search_info.thinking = false;
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            task.rx = None;
            task.info_rx = None;
            search_info.thinking = false;
        }
    }
}

/// After the AI moved, set the persistent engine pondering the human reply
/// it predicted (`go ponder` on the post-reply position).
fn maybe_start_ponder(
    task: &mut AiTask,
    session: &EngineSession,
    settings: &AiSettings,
    core: &CoreGame,
    ponder_mv: Option<Move>,
) {
    let Some(pm) = ponder_mv else { return };
    let Some(cmd_tx) = session.cmd_tx.clone() else {
        return;
    };
    if core.mode != GameMode::VsAi || core.game.is_over() {
        return;
    }
    // The predicted reply must still be legal on the current board.
    let mut after = core.game.board().clone();
    if !after.is_legal(pm) {
        return;
    }
    after.make_move(pm);
    let predicted_fen = after.to_fen();
    let base_fen = core.game.board().to_fen();
    let start_fen = core
        .game
        .board_at_ply(0)
        .map(|b| b.to_fen())
        .unwrap_or_else(|| base_fen.clone());
    let mut history: Vec<Move> = core.game.played_moves().collect();
    history.push(pm);
    let movetime = settings.difficulty.limits().movetime;
    if cmd_tx
        .send(EngineCmd::StartPonder {
            fen: start_fen,
            history,
            movetime,
        })
        .is_ok()
    {
        task.ponder = Some(PonderTrack {
            base_fen,
            predicted_fen,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AiSettings;

    #[test]
    fn engine_config_none_without_external_engine() {
        let settings = AiSettings {
            difficulty: chess_ai::Difficulty::Easy,
            engine_path: None,
            eval_file: None,
        };
        assert!(engine_config(&settings).is_none(), "no engine -> built-in");
    }

    #[test]
    fn engine_config_carries_eval_and_tuning_options() {
        let settings = AiSettings {
            difficulty: chess_ai::Difficulty::Hard,
            engine_path: Some("/tmp/fake-pikafish".into()),
            eval_file: Some("/tmp/fake.nnue".into()),
        };
        let cfg = engine_config(&settings).expect("engine configured");
        let keys: Vec<&str> = cfg.options.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"EvalFile"));
        assert!(keys.contains(&"Threads"));
        assert!(keys.contains(&"Hash"));
    }

    #[test]
    fn engine_tuning_gives_extreme_the_whole_machine() {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);

        let (threads, hash) = engine_tuning(Difficulty::Extreme);
        assert_eq!(threads, cores.clamp(1, 16));
        assert_eq!(hash, 256);

        let (t, h) = engine_tuning(Difficulty::Hard);
        assert_eq!(t, (cores / 2).clamp(1, 4));
        assert_eq!(h, 32);
    }
}
