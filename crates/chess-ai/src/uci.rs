//! Integration with an external UCI engine (Pikafish is the recommended,
//! MIT-licensed, top-strength Xiangqi engine).
//!
//! Communication runs over the child process's stdin/stdout using Tokio so the
//! Bevy main thread is never blocked. The protocol used is standard UCI:
//!
//! ```text
//! > uci
//! < ... id / option lines ...
//! < uciok
//! > isready
//! < readyok
//! > position fen <FEN> [moves <m1> <m2> ...]
//! > go movetime <ms>
//! < info ...
//! < bestmove <iccs>
//! ```
//!
//! Pikafish additionally needs its NNUE file; pass it via [`UciConfig::options`]
//! as `("EvalFile", "/path/to/pikafish.nnue")`.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use chess_core::{Board, Move};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

/// How to launch and configure the external engine.
#[derive(Debug, Clone)]
pub struct UciConfig {
    /// Path to the engine executable (e.g. `./engines/pikafish`).
    pub path: PathBuf,
    /// `setoption name <k> value <v>` pairs sent after `uci`.
    pub options: Vec<(String, String)>,
    /// Handshake timeout.
    pub handshake_timeout: Duration,
}

impl UciConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        UciConfig {
            path: path.into(),
            options: Vec::new(),
            handshake_timeout: Duration::from_secs(10),
        }
    }

    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UciError {
    #[error("failed to spawn engine: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("engine i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("engine handshake timed out")]
    HandshakeTimeout,
    #[error("engine closed the connection unexpectedly")]
    Closed,
    #[error("engine returned an unparseable move: {0:?}")]
    BadMove(String),
}

/// Parse a UCI `info` line into [`SearchInfo`].
///
/// Handles the subset Pikafish/Stockfish emit:
/// `info depth 22 seldepth 30 score cp 35 nodes 12345 time 123 pv h2e2 h9g7`.
/// `score mate N` is encoded as a near-mate centipawn score compatible with
/// the GUI's mate display convention (`|score| >= 9900`). Returns `None` for
/// any non-info or malformed line.
fn parse_info_line(
    line: &str,
    stm: chess_core::Color,
    elapsed: Duration,
) -> Option<crate::search::SearchInfo> {
    let rest = line.strip_prefix("info ")?;
    let mut depth = 0u32;
    let mut score: Option<i32> = None;
    let mut nodes = 0u64;
    let mut pv: Vec<Move> = Vec::new();

    let mut it = rest.split_whitespace().peekable();
    while let Some(tok) = it.next() {
        match tok {
            "depth" => depth = it.next()?.parse().ok()?,
            "nodes" => nodes = it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "score" => {
                let kind = it.next()?;
                let value: i32 = it.next()?.parse().ok()?;
                score = Some(match kind {
                    "cp" => value,
                    // mate-in-N → ±(10000 - 2N), matching the GUI's display.
                    "mate" => {
                        let n = value.clamp(-500, 500);
                        if n >= 0 {
                            10000 - 2 * n.max(1)
                        } else {
                            -(10000 - 2 * (-n).max(1))
                        }
                    }
                    _ => return None,
                });
            }
            "pv" => {
                pv = it.by_ref().filter_map(Move::from_iccs).collect();
                break;
            }
            _ => {}
        }
    }
    // Lines like `info string …` carry no depth/score; ignore them.
    if depth == 0 && score.is_none() {
        return None;
    }
    Some(crate::search::SearchInfo {
        depth,
        score: score?,
        side_to_move: stm,
        pv,
        nodes,
        elapsed,
        is_final: false,
    })
}

/// Parse a `bestmove <mv> [ponder <mv>]` line. Returns `None` for
/// non-bestmove lines and unparsable/`(none)` moves (callers turn the latter
/// into [`UciError::BadMove`] as needed).
pub fn parse_bestmove_line(line: &str) -> Option<(Move, Option<Move>)> {
    let rest = line.strip_prefix("bestmove ")?;
    let mut it = rest.split_whitespace();
    let mv = Move::from_iccs(it.next()?)?;
    let ponder = match it.next() {
        Some("ponder") => it.next().and_then(Move::from_iccs),
        _ => None,
    };
    Some((mv, ponder))
}

/// A live connection to a running UCI engine process.
pub struct UciEngine {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl UciEngine {
    /// Launch the engine and complete the `uci` / `isready` handshake.
    pub async fn launch(config: &UciConfig) -> Result<UciEngine, UciError> {
        let mut child = Command::new(&config.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(UciError::Spawn)?;

        let stdin = child.stdin.take().ok_or(UciError::Closed)?;
        let stdout = BufReader::new(child.stdout.take().ok_or(UciError::Closed)?);

        let mut engine = UciEngine {
            child,
            stdin,
            stdout,
        };

        timeout(config.handshake_timeout, engine.handshake(config))
            .await
            .map_err(|_| UciError::HandshakeTimeout)??;

        Ok(engine)
    }

    async fn handshake(&mut self, config: &UciConfig) -> Result<(), UciError> {
        self.send("uci").await?;
        self.read_until("uciok").await?;
        for (k, v) in &config.options {
            self.send(&format!("setoption name {k} value {v}")).await?;
        }
        self.send("isready").await?;
        self.read_until("readyok").await?;
        Ok(())
    }

    async fn send(&mut self, line: &str) -> Result<(), UciError> {
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Read lines until one starts with `marker`.
    async fn read_until(&mut self, marker: &str) -> Result<(), UciError> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                return Err(UciError::Closed);
            }
            if line.trim_start().starts_with(marker) {
                return Ok(());
            }
        }
    }

    /// Send `position fen <fen> [moves …]`. `history` is applied on top of
    /// `fen` — pass the game's *initial* FEN plus the full move list so the
    /// engine sees the real game trajectory (repetition detection and its
    /// transposition table both benefit).
    pub async fn set_position(&mut self, fen: &str, history: &[Move]) -> Result<(), UciError> {
        let mut pos = format!("position fen {fen}");
        if !history.is_empty() {
            pos.push_str(" moves");
            for m in history {
                pos.push(' ');
                pos.push_str(&m.to_iccs());
            }
        }
        self.send(&pos).await
    }

    /// `go movetime <ms>`. With `ponder = true` the engine searches in
    /// ponder mode: per the UCI protocol it must **not** emit `bestmove`
    /// (nor stop on the clock) until `ponderhit` or `stop` arrives. The
    /// movetime budget is counted from the `go`, so a ponderhit after the
    /// human thought longer than the budget yields an instant reply.
    pub async fn go_movetime(&mut self, movetime: Duration, ponder: bool) -> Result<(), UciError> {
        let cmd = if ponder {
            format!("go ponder movetime {}", movetime.as_millis())
        } else {
            format!("go movetime {}", movetime.as_millis())
        };
        self.send(&cmd).await
    }

    /// Convert a ponder search into a real one (`ponderhit`).
    pub async fn ponder_hit(&mut self) -> Result<(), UciError> {
        self.send("ponderhit").await
    }

    /// Interrupt the current search (`stop`); a `bestmove` line follows.
    pub async fn stop(&mut self) -> Result<(), UciError> {
        self.send("stop").await
    }

    /// `ucinewgame` + readiness handshake: clears the transposition table and
    /// game state for a fresh game.
    pub async fn new_game(&mut self) -> Result<(), UciError> {
        self.send("ucinewgame").await?;
        self.send("isready").await?;
        self.read_until("readyok").await
    }

    /// Read one stdout line (trimmed). [`UciError::Closed`] at EOF.
    pub async fn next_line(&mut self) -> Result<String, UciError> {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).await?;
        if n == 0 {
            return Err(UciError::Closed);
        }
        Ok(line.trim().to_string())
    }

    /// Read engine output until `bestmove`, streaming `info` lines to `sink`.
    /// Returns the best move, the engine's predicted reply (the `ponder`
    /// move, when reported), and a final info snapshot.
    pub async fn wait_bestmove(
        &mut self,
        stm: chess_core::Color,
        started: Instant,
        sink: crate::search::SearchInfoSink,
    ) -> Result<(Move, Option<Move>, Option<crate::search::SearchInfo>), UciError> {
        let mut last_info: Option<crate::search::SearchInfo> = None;
        loop {
            let line = self.next_line().await?;
            if let Some(info) = parse_info_line(&line, stm, started.elapsed()) {
                if let Some(ref tx) = sink {
                    // Channel is bounded: drop info when the GUI lags. If the
                    // receiver is gone entirely (e.g. analysis panel closed),
                    // stop early instead of burning CPU unwatched.
                    if let Err(crossbeam_channel::TrySendError::Disconnected(_)) =
                        tx.try_send(info.clone())
                    {
                        let _ = self.send("stop").await;
                    }
                }
                last_info = Some(info);
                continue;
            }
            if line.starts_with("bestmove ") {
                if line.contains("(none)") {
                    return Err(UciError::BadMove(line));
                }
                let (mv, ponder) =
                    parse_bestmove_line(&line).ok_or_else(|| UciError::BadMove(line.clone()))?;
                // Final event carrying at least the chosen move.
                let final_info = crate::search::SearchInfo {
                    depth: last_info.as_ref().map(|i| i.depth).unwrap_or(0),
                    score: last_info.as_ref().map(|i| i.score).unwrap_or(0),
                    side_to_move: stm,
                    pv: last_info
                        .as_ref()
                        .map(|i| i.pv.clone())
                        .filter(|pv| pv.first() == Some(&mv))
                        .unwrap_or_else(|| vec![mv]),
                    nodes: last_info.as_ref().map(|i| i.nodes).unwrap_or(0),
                    elapsed: started.elapsed(),
                    is_final: true,
                };
                if let Some(ref tx) = sink {
                    let _ = tx.try_send(final_info.clone());
                }
                return Ok((mv, ponder, Some(final_info)));
            }
        }
    }

    /// Ask the engine for the best move in `board`, thinking for `movetime`.
    /// `history` is the list of moves played from `board`'s position (usually
    /// empty when sending a full FEN).
    pub async fn best_move(
        &mut self,
        board: &Board,
        history: &[Move],
        movetime: Duration,
    ) -> Result<Move, UciError> {
        self.best_move_with_info(board, history, movetime, None)
            .await
            .map(|(mv, _)| mv)
    }

    /// Like [`Self::best_move`], but additionally parses the engine's
    /// `info depth … score … pv …` lines and streams them to `sink` as
    /// [`SearchInfo`] events (a final event is sent with the chosen move).
    ///
    /// Returns the best move plus the last info line seen, so callers can
    /// display an evaluation even if no intermediate info was streamed.
    pub async fn best_move_with_info(
        &mut self,
        board: &Board,
        history: &[Move],
        movetime: Duration,
        sink: crate::search::SearchInfoSink,
    ) -> Result<(Move, Option<crate::search::SearchInfo>), UciError> {
        self.set_position(&board.to_fen(), history).await?;
        self.go_movetime(movetime, false).await?;
        let (mv, _ponder, info) = self
            .wait_bestmove(board.side_to_move(), Instant::now(), sink)
            .await?;
        Ok((mv, info))
    }

    /// Politely shut the engine down.
    pub async fn quit(mut self) -> Result<(), UciError> {
        let _ = self.send("quit").await;
        let _ = self.child.wait().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::{Board, Color};

    fn parse(line: &str, stm: Color) -> Option<crate::search::SearchInfo> {
        parse_info_line(line, stm, Duration::from_millis(123))
    }

    #[test]
    fn parse_info_line_cp_with_pv() {
        let info = parse(
            "info depth 22 seldepth 30 multipv 1 score cp 35 nodes 1234567 time 1234 pv h2e2 h9g7 b0c2",
            Color::Red,
        )
        .expect("should parse");
        assert_eq!(info.depth, 22);
        assert_eq!(info.score, 35);
        assert_eq!(info.nodes, 1234567);
        assert_eq!(info.side_to_move, Color::Red);
        assert!(!info.is_final);
        assert_eq!(info.pv.len(), 3);
        assert_eq!(info.pv[0].to_iccs(), "h2e2");
    }

    #[test]
    fn parse_info_line_negative_score_and_black_stm() {
        let info = parse("info depth 10 score cp -88 nodes 42", Color::Black).unwrap();
        assert_eq!(info.score, -88);
        assert_eq!(info.side_to_move, Color::Black);
        // Red-centric helper flips for black-to-move positions.
        assert_eq!(info.red_score(), 88);
    }

    #[test]
    fn parse_info_line_mate_encoding() {
        let mate3 = parse("info depth 20 score mate 3 nodes 1", Color::Red).unwrap();
        assert!(mate3.score >= 9900, "mate must use the >=9900 convention");
        assert_eq!((10000 - mate3.score.abs()) / 2, 3);

        let mate10 = parse("info depth 20 score mate -10 nodes 1", Color::Red).unwrap();
        assert!(mate10.score <= -9900);
        assert_eq!((10000 - mate10.score.abs()) / 2, 10);
    }

    #[test]
    fn parse_info_line_rejects_non_info() {
        assert!(parse("bestmove h2e2", Color::Red).is_none());
        assert!(parse("uciok", Color::Red).is_none());
        assert!(parse("", Color::Red).is_none());
        assert!(parse("info string Pikafish is thinking", Color::Red).is_none());
        // depth without a score carries no eval -> ignored.
        assert!(parse("info depth 5 currmove h2e2", Color::Red).is_none());
    }

    #[test]
    fn parse_bestmove_with_and_without_ponder() {
        let (mv, ponder) = parse_bestmove_line("bestmove h2e2").unwrap();
        assert_eq!(mv.to_iccs(), "h2e2");
        assert!(ponder.is_none());

        let (mv, ponder) = parse_bestmove_line("bestmove h2e2 ponder h9g7").unwrap();
        assert_eq!(mv.to_iccs(), "h2e2");
        assert_eq!(ponder.unwrap().to_iccs(), "h9g7");

        // Not a bestmove line / no usable move.
        assert!(parse_bestmove_line("info depth 3").is_none());
        assert!(parse_bestmove_line("bestmove (none)").is_none());
        // "(none)" ponder move degrades to no-ponder instead of failing.
        let (_, ponder) = parse_bestmove_line("bestmove h2e2 ponder (none)").unwrap();
        assert!(ponder.is_none());
    }

    /// End-to-end against the real bundled Pikafish when present on this
    /// machine (engines/ is git-ignored, so CI machines skip it).
    #[tokio::test]
    async fn real_engine_streams_info_and_returns_legal_move() {
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let bin = manifest.join("../../engines/macos-arm64/pikafish");
        let nnue = manifest.join("../../engines/pikafish.nnue");
        if !(cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && bin.is_file()
            && nnue.is_file())
        {
            eprintln!("skipping: bundled Pikafish not present");
            return;
        }

        let cfg = UciConfig::new(bin)
            .with_option("EvalFile", nnue.to_string_lossy().to_string())
            .with_option("Threads", "2")
            .with_option("Hash", "16");
        let mut engine = UciEngine::launch(&cfg)
            .await
            .expect("handshake with bundled Pikafish");

        let (tx, rx) = crossbeam_channel::unbounded();
        let board = Board::start_position();
        let (mv, final_info) = engine
            .best_move_with_info(&board, &[], Duration::from_millis(800), Some(tx))
            .await
            .expect("best move");

        assert!(board.is_legal(mv), "engine move must be legal");
        let infos: Vec<_> = rx.try_iter().collect();
        assert!(
            infos.iter().any(|i| i.depth > 0),
            "engine should stream depth info"
        );
        let final_info = final_info.unwrap();
        assert!(final_info.is_final);
        assert_eq!(final_info.pv.first(), Some(&mv));
    }

    /// UCI ponder protocol against the real bundled Pikafish (skipped when
    /// the engine is not present, e.g. on CI). Covers both exits from a
    /// ponder search: `ponderhit` and `stop`.
    #[tokio::test]
    async fn real_engine_ponder_hit_and_stop() {
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let bin = manifest.join("../../engines/macos-arm64/pikafish");
        let nnue = manifest.join("../../engines/pikafish.nnue");
        if !(cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && bin.is_file()
            && nnue.is_file())
        {
            eprintln!("skipping: bundled Pikafish not present");
            return;
        }

        let cfg = UciConfig::new(bin)
            .with_option("EvalFile", nnue.to_string_lossy().to_string())
            .with_option("Threads", "2")
            .with_option("Hash", "16");
        let mut engine = UciEngine::launch(&cfg)
            .await
            .expect("handshake with bundled Pikafish");
        let board = Board::start_position();

        // 1) ponder -> ponderhit: the engine must hold its bestmove while
        // pondering, then finish within the budget counted from `go`.
        engine.set_position(&board.to_fen(), &[]).await.unwrap();
        engine
            .go_movetime(Duration::from_millis(1500), true)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        engine.ponder_hit().await.unwrap();
        let (mv, _ponder, _info) = timeout(
            Duration::from_secs(10),
            engine.wait_bestmove(board.side_to_move(), Instant::now(), None),
        )
        .await
        .expect("bestmove must follow ponderhit")
        .expect("valid bestmove");
        assert!(board.is_legal(mv), "ponderhit move must be legal");

        // 2) ponder -> stop: even with a huge movetime budget, `stop` ends
        // the ponder promptly (this is the ponder-miss path).
        engine.set_position(&board.to_fen(), &[]).await.unwrap();
        engine
            .go_movetime(Duration::from_secs(600), true)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        engine.stop().await.unwrap();
        let (mv, _, _) = timeout(
            Duration::from_secs(5),
            engine.wait_bestmove(board.side_to_move(), Instant::now(), None),
        )
        .await
        .expect("bestmove must follow stop")
        .expect("valid bestmove");
        assert!(board.is_legal(mv), "stop move must be legal");

        engine.quit().await.ok();
    }
}
