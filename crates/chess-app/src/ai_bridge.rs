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

use bevy::prelude::*;
use chess_ai::{Ai, Difficulty, SearchInfo, SearchLimits, UciConfig};
use chess_core::Move;
use crossbeam_channel::{Receiver, TryRecvError};

use crate::app_state::{AiSettings, CoreGame, GameMode};
use crate::history_view::HistoryView;
use crate::moves::apply_local_move;
use crate::sound::{MoveSound, PendingSound};

/// Holds the in-flight AI computation, if any.
#[derive(Resource, Default)]
pub struct AiTask {
    pub rx: Option<Receiver<Option<Move>>>,
    pub info_rx: Option<Receiver<SearchInfo>>,
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

/// If it is the AI's turn and no task is running, start one.
pub fn request_ai_move(
    core: Res<CoreGame>,
    settings: Res<AiSettings>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
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
    let limits: SearchLimits = settings.difficulty.limits();
    let use_book = settings.difficulty != Difficulty::Easy;
    let engine_path = settings.engine_path.clone();
    let eval_file = settings.eval_file.clone();

    let (tx, rx) = crossbeam_channel::bounded(1);
    // Bounded channel for search info: GUI drains frequently, 4 slots prevent overflow.
    let (info_tx, info_rx) = crossbeam_channel::bounded(4);
    task.rx = Some(rx);
    task.info_rx = Some(info_rx);
    search_info.thinking = true;
    search_info.latest = None;

    let rt = runtime.0.clone();
    rt.spawn(async move {
        let mut ai = match engine_path {
            Some(path) => {
                let mut cfg = UciConfig::new(path);
                if let Some(ev) = eval_file {
                    cfg = cfg.with_option("EvalFile", ev.to_string_lossy().to_string());
                }
                Ai::pikafish(&cfg).await
            }
            None => Ai::builtin(),
        };
        let mv = ai
            .best_move_with_info(&board, &[], limits, use_book, Some(info_tx))
            .await;
        let _ = tx.send(mv);
    });
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
pub fn poll_ai_move(
    mut task: ResMut<AiTask>,
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
        Ok(Some(mv)) => {
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
        }
        Ok(None) => {
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
