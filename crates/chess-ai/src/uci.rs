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

        let mut line = String::new();
        loop {
            line.clear();
            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                return Err(UciError::Closed);
            }
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("bestmove ") {
                let token = rest.split_whitespace().next().unwrap_or("");
                if token == "(none)" || token.is_empty() {
                    return Err(UciError::BadMove(token.to_string()));
                }
                return Move::from_iccs(token).ok_or_else(|| UciError::BadMove(token.to_string()));
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
