//! `chess-core` — a complete Xiangqi (Chinese Chess) rules engine.
//!
//! Pure logic with no rendering, networking, or async dependencies, so it
//! builds and tests on every platform. It provides:
//!
//! * [`Board`] — placement + side to move, pseudo-legal & legal move
//!   generation, attack/check detection, FEN I/O.
//! * [`Game`] — move history, undo, and result adjudication (checkmate,
//!   stalemate-as-loss, threefold repetition, perpetual check).
//!
//! All competition movement rules are implemented: palace confinement for
//! kings/advisors, the river limit and "塞象眼" eye-block for elephants, the
//! "蹩马腿" leg-block for horses, the cannon screen capture, pawn promotion of
//! sideways movement after crossing the river, and the "flying general"
//! (白脸将) prohibition.

pub mod board;
pub mod fen;
pub mod game;
pub mod moves;
pub mod piece;
pub mod square;

pub use board::Board;
pub use fen::{FenError, START_FEN};
pub use game::{DrawReason, Game, GameResult, IllegalMove, WinReason};
pub use moves::{Move, UndoState};
pub use piece::{Color, Piece, PieceKind};
pub use square::Square;

#[cfg(test)]
mod tests;
