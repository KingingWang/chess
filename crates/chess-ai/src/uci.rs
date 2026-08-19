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
use std::time::Duration;

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
        let mut pos = format!("position fen {}", board.to_fen());
        if !history.is_empty() {
            pos.push_str(" moves");
            for m in history {
                pos.push(' ');
                pos.push_str(&m.to_iccs());
            }
        }
        self.send(&pos).await?;
        self.send(&format!("go movetime {}", movetime.as_millis()))
            .await?;

        let start = std::time::Instant::now();
        let stm = board.side_to_move();
        let mut last_info: Option<crate::search::SearchInfo> = None;
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                return Err(UciError::Closed);
            }
            let line = line.trim();
            if let Some(info) = parse_info_line(line, stm, start.elapsed()) {
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
            if let Some(rest) = line.strip_prefix("bestmove ") {
                let token = rest.split_whitespace().next().unwrap_or("");
                if token == "(none)" || token.is_empty() {
                    return Err(UciError::BadMove(token.to_string()));
                }
                let mv =
                    Move::from_iccs(token).ok_or_else(|| UciError::BadMove(token.to_string()))?;
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
                    elapsed: start.elapsed(),
                    is_final: true,
                };
                if let Some(ref tx) = sink {
                    let _ = tx.try_send(final_info.clone());
                }
                return Ok((mv, Some(final_info)));
            }
        }
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
}
