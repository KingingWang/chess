//! `chess-ai` — opponent intelligence for the Xiangqi game.
//!
//! Two backends are provided behind one async API:
//!
//! * [`UciEngine`] — drives an external **Pikafish** (MIT-licensed, top
//!   strength) process over the UCI protocol. This is the recommended path to
//!   reach the project's strength target (≥2600 ELO @ 3 s on an i7-12700K),
//!   which is met by Pikafish + its NNUE, not by the built-in engine.
//! * [`search`] — a self-contained alpha-beta + quiescence engine in pure Rust
//!   used as a **fallback** when no external engine is configured or it fails
//!   to launch. It is correct and club-strength, deliberately simple.
//!
//! All searches are CPU- or IO-bound and are kept off the render thread:
//! [`Ai::best_move`] runs the built-in search via `spawn_blocking` and the UCI
//! engine via async IO, so callers (e.g. a Bevy task pool) never block.

pub mod book;
pub mod eval;
pub mod search;
pub mod tt;
pub mod uci;

use std::time::Duration;

use chess_core::{Board, Move};

pub use search::{SearchLimits, SearchResult};
pub use uci::{UciConfig, UciEngine, UciError};

/// Difficulty presets mapping to think time and (for the built-in engine) a
/// depth cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Master,
}

impl Difficulty {
    pub fn limits(self) -> SearchLimits {
        match self {
            Difficulty::Easy => SearchLimits {
                movetime: Duration::from_millis(200),
                max_depth: 4,
            },
            Difficulty::Medium => SearchLimits {
                movetime: Duration::from_millis(800),
                max_depth: 8,
            },
            Difficulty::Hard => SearchLimits {
                movetime: Duration::from_millis(2000),
                max_depth: 16,
            },
            Difficulty::Master => SearchLimits {
                movetime: Duration::from_millis(3000),
                max_depth: 64,
            },
        }
    }

    /// Human-readable Chinese label for display in the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "简单",
            Difficulty::Medium => "中等",
            Difficulty::Hard => "困难",
            Difficulty::Master => "大师",
        }
    }
    /// Emoji icon for this difficulty.
    pub fn emoji(self) -> &'static str {
        match self {
            Difficulty::Easy => "易",
            Difficulty::Medium => "中",
            Difficulty::Hard => "难",
            Difficulty::Master => "极",
        }
    }
}

/// Unified opponent. Prefer [`Ai::pikafish`]; it transparently falls back to
/// the built-in engine if the external engine cannot be launched.
pub enum Ai {
    Builtin,
    Uci(Box<UciEngine>),
}

/// Global opening book (lazy-initialized).
static BOOK: std::sync::OnceLock<book::OpeningBook> = std::sync::OnceLock::new();

fn get_book() -> &'static book::OpeningBook {
    BOOK.get_or_init(book::OpeningBook::default_book)
}

impl Ai {
    /// Always use the built-in engine.
    pub fn builtin() -> Ai {
        Ai::Builtin
    }

    /// Try to launch Pikafish (or any UCI engine); on failure, log and fall
    /// back to the built-in engine so the game is always playable.
    pub async fn pikafish(config: &UciConfig) -> Ai {
        match UciEngine::launch(config).await {
            Ok(engine) => {
                tracing::info!(path = %config.path.display(), "UCI engine ready");
                Ai::Uci(Box::new(engine))
            }
            Err(e) => {
                tracing::warn!(error = %e, "UCI engine unavailable; using built-in fallback");
                Ai::Builtin
            }
        }
    }

    /// Compute a move for the side to move in `board`. `history` lists the moves
    /// already played from `board` (used by the UCI backend; the built-in
    /// engine searches the position directly).
    pub async fn best_move(
        &mut self,
        board: &Board,
        history: &[Move],
        limits: SearchLimits,
        use_book: bool,
    ) -> Option<Move> {
        match self {
            Ai::Builtin => {
                // Try opening book first (skip for Easy difficulty).
                if use_book {
                    if let Some(book_mv) = get_book().lookup(board) {
                        tracing::info!(mv = %book_mv.to_iccs(), "book move");
                        return Some(book_mv);
                    }
                }
                let board = board.clone();
                // Keep the CPU-bound search off the async/render thread.
                tokio::task::spawn_blocking(move || search::search(&board, limits).best_move)
                    .await
                    .ok()
                    .flatten()
            }
            Ai::Uci(engine) => match engine.best_move(board, history, limits.movetime).await {
                Ok(mv) => Some(mv),
                Err(e) => {
                    tracing::error!(error = %e, "UCI move failed; falling back to built-in");
                    let board = board.clone();
                    tokio::task::spawn_blocking(move || search::search(&board, limits).best_move)
                        .await
                        .ok()
                        .flatten()
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::{Color, Piece, PieceKind, Square};

    fn sq(f: u8, r: u8) -> Square {
        Square::new(f, r).unwrap()
    }

    #[test]
    fn builtin_finds_mate_in_one() {
        // Red to move; Rc on e6 -> e1 mates the boxed-in Black king (see core
        // tests for the identical position).
        let mut b = Board::empty();
        b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
        b.set_piece(sq(3, 0), Some(Piece::new(Color::Red, PieceKind::Chariot)));
        b.set_piece(sq(5, 0), Some(Piece::new(Color::Red, PieceKind::Chariot)));
        b.set_piece(sq(4, 5), Some(Piece::new(Color::Red, PieceKind::Chariot)));
        b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
        b.set_side_to_move(Color::Red);

        let res = search::search(
            &b,
            SearchLimits {
                movetime: Duration::from_secs(2),
                max_depth: 4,
            },
        );
        let mv = res.best_move.expect("a move");
        // Apply and check it is mate.
        let mut bb = b.clone();
        bb.make_move(mv);
        assert!(bb.legal_moves().is_empty() && bb.is_in_check(Color::Black));
    }

    #[test]
    fn builtin_prefers_winning_material() {
        // Red chariot can capture an undefended black chariot for free.
        let mut b = Board::empty();
        b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
        b.set_piece(sq(8, 9), Some(Piece::new(Color::Black, PieceKind::King)));
        b.set_piece(sq(0, 4), Some(Piece::new(Color::Red, PieceKind::Chariot)));
        b.set_piece(sq(4, 4), Some(Piece::new(Color::Black, PieceKind::Chariot)));
        b.set_side_to_move(Color::Red);

        let res = search::search(
            &b,
            SearchLimits {
                movetime: Duration::from_millis(500),
                max_depth: 6,
            },
        );
        let mv = res.best_move.expect("a move");
        assert_eq!(
            mv,
            Move::new(sq(0, 4), sq(4, 4)),
            "should grab the free chariot"
        );
    }

    #[tokio::test]
    async fn ai_builtin_returns_move_async() {
        let mut ai = Ai::builtin();
        let b = Board::start_position();
        let mv = ai
            .best_move(
                &b,
                &[],
                SearchLimits {
                    movetime: Duration::from_millis(300),
                    max_depth: 4,
                },
                true,
            )
            .await;
        assert!(mv.is_some());
    }
}
