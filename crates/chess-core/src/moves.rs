//! Move representation.

use crate::piece::Piece;
use crate::square::Square;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A move from one square to another. Captures are implicit (whatever sits on
/// `to`). Xiangqi has no promotion or special moves, so this is sufficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Move {
    pub from: Square,
    pub to: Square,
}

impl Move {
    #[inline]
    pub const fn new(from: Square, to: Square) -> Move {
        Move { from, to }
    }

    /// ICCS coordinate string, e.g. `h2e2`. Files are letters a..i (file 0..8),
    /// ranks are digits 0..9 measured from Red's back rank.
    pub fn to_iccs(self) -> String {
        fn file_char(f: u8) -> char {
            (b'a' + f) as char
        }
        format!(
            "{}{}{}{}",
            file_char(self.from.file()),
            self.from.rank(),
            file_char(self.to.file()),
            self.to.rank(),
        )
    }

    /// Parse an ICCS coordinate string like `h2e2`.
    pub fn from_iccs(s: &str) -> Option<Move> {
        let b = s.as_bytes();
        if b.len() != 4 {
            return None;
        }
        let parse_file = |c: u8| -> Option<u8> {
            if (b'a'..=b'i').contains(&c) {
                Some(c - b'a')
            } else {
                None
            }
        };
        let parse_rank = |c: u8| -> Option<u8> {
            if c.is_ascii_digit() {
                Some(c - b'0')
            } else {
                None
            }
        };
        let from = Square::new(parse_file(b[0])?, parse_rank(b[1])?)?;
        let to = Square::new(parse_file(b[2])?, parse_rank(b[3])?)?;
        Some(Move::new(from, to))
    }
}

/// A move paired with the piece captured (if any), used to undo moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UndoState {
    pub mv: Move,
    pub captured: Option<Piece>,
}
