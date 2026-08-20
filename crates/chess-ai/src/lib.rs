//! `chess-ai` — opponent intelligence for the Xiangqi game.
//!
//! Two backends are provided behind one async API:
//!
//! * [`UciEngine`] — drives an external **Pikafish** (GPL-3.0, top
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
pub mod rng;
pub mod search;
pub mod tt;
pub mod uci;

use std::time::Duration;

use chess_core::{Board, Move};

pub use search::{SearchInfo, SearchInfoSink, SearchLimits, SearchResult};
pub use uci::{UciConfig, UciEngine, UciError};

/// Difficulty presets mapping to think time and (for the built-in engine) a
/// depth cap plus a root-move variety window so identical positions do not
/// always produce identical replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Master,
    /// Absolute top tier: long thinks, full search depth, zero randomness,
    /// and no opening book — every move is computed by the engine itself.
    Extreme,
}

impl Difficulty {
    pub fn limits(self) -> SearchLimits {
        match self {
            Difficulty::Easy => SearchLimits {
                movetime: Duration::from_millis(200),
                max_depth: 4,
                // ~1 pawn: noticeably playful, still never drops pieces outright.
                variety_window: 80,
            },
            Difficulty::Medium => SearchLimits {
                movetime: Duration::from_millis(800),
                max_depth: 8,
                variety_window: 40,
            },
            Difficulty::Hard => SearchLimits {
                movetime: Duration::from_millis(2000),
                max_depth: 16,
                variety_window: 20,
            },
            Difficulty::Master => SearchLimits {
                movetime: Duration::from_millis(3000),
                max_depth: 64,
                // Full strength: always the best move (opening variety still
                // comes from the randomised book).
                variety_window: 0,
            },
            Difficulty::Extreme => SearchLimits {
                movetime: Duration::from_millis(10000),
                max_depth: 64,
                variety_window: 0,
            },
        }
    }

    /// Whether the opening book is consulted at this difficulty.
    ///
    /// Easy skips the book (hand-tuned playfulness from move one). Extreme
    /// skips it too, for the opposite reason: at the top tier every move —
    /// including the opening — should come from the engine's own search,
    /// not from a weighted-random book line.
    pub fn uses_book(self) -> bool {
        matches!(self, Self::Medium | Self::Hard | Self::Master)
    }

    /// Human-readable Chinese label for display in the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "简单",
            Difficulty::Medium => "中等",
            Difficulty::Hard => "困难",
            Difficulty::Master => "大师",
            Difficulty::Extreme => "巅峰",
        }
    }
    /// Emoji icon for this difficulty.
    pub fn emoji(self) -> &'static str {
        match self {
            Difficulty::Easy => "易",
            Difficulty::Medium => "中",
            Difficulty::Hard => "难",
            Difficulty::Master => "极",
            Difficulty::Extreme => "巅",
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

/// Look up an opening-book move directly, without going through [`Ai`].
///
/// The app's persistent-engine bridge uses this so book moves never touch
/// the long-lived engine process.
pub fn book_move(board: &Board) -> Option<Move> {
    get_book().lookup(board)
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
            Ai::Uci(engine) => {
                // Consult the opening book first: a raw engine always finds
                // the same "best" opening, so the weighted-random book is
                // what gives VsAi games varied openings (skipped on Easy).
                if use_book {
                    if let Some(book_mv) = get_book().lookup(board) {
                        tracing::info!(mv = %book_mv.to_iccs(), "book move");
                        return Some(book_mv);
                    }
                }
                match engine.best_move(board, history, limits.movetime).await {
                    Ok(mv) => Some(mv),
                    Err(e) => {
                        tracing::error!(error = %e, "UCI move failed; falling back to built-in");
                        let board = board.clone();
                        tokio::task::spawn_blocking(move || {
                            search::search(&board, limits).best_move
                        })
                        .await
                        .ok()
                        .flatten()
                    }
                }
            }
        }
    }

    /// Compute a move with real-time search info streaming to the GUI.
    /// The `info_sink` receives [`SearchInfo`] updates during search.
    pub async fn best_move_with_info(
        &mut self,
        board: &Board,
        history: &[Move],
        limits: SearchLimits,
        use_book: bool,
        info_sink: SearchInfoSink,
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
                tokio::task::spawn_blocking(move || {
                    search::search_with_info(&board, limits, info_sink).best_move
                })
                .await
                .ok()
                .flatten()
            }
            Ai::Uci(engine) => {
                // See `best_move`: book first, for opening variety.
                if use_book {
                    if let Some(book_mv) = get_book().lookup(board) {
                        tracing::info!(mv = %book_mv.to_iccs(), "book move");
                        return Some(book_mv);
                    }
                }
                // UCI engines stream real `info` lines (depth/score/PV)
                // through the sink; a final event carries the chosen move.
                match engine
                    .best_move_with_info(board, history, limits.movetime, info_sink.clone())
                    .await
                {
                    Ok((mv, _)) => Some(mv),
                    Err(e) => {
                        tracing::error!(error = %e, "UCI move failed; falling back to built-in");
                        let board = board.clone();
                        tokio::task::spawn_blocking(move || {
                            search::search_with_info(&board, limits, info_sink).best_move
                        })
                        .await
                        .ok()
                        .flatten()
                    }
                }
            }
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
                ..Default::default()
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
                ..Default::default()
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
                    ..Default::default()
                },
                true,
            )
            .await;
        assert!(mv.is_some());
    }
}
pub mod endgame;
