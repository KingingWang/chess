//! Board geometry for Xiangqi.
//!
//! The board is 9 files (columns, 0..=8) by 10 ranks (rows, 0..=9).
//! Rank 0 is Red's back rank (bottom); rank 9 is Black's back rank (top).
//! A [`Square`] is the linear index `rank * 9 + file` in `0..90`.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const FILES: u8 = 9;
pub const RANKS: u8 = 10;
pub const NUM_SQUARES: usize = 90;

/// The river divides ranks 0..=4 (Red half) from ranks 5..=9 (Black half).
pub const RIVER_RED_MAX_RANK: u8 = 4;
pub const RIVER_BLACK_MIN_RANK: u8 = 5;

/// A square on the board, stored as a compact index `0..90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Square(u8);

impl Square {
    /// Construct from a raw index, panicking if out of range (debug builds).
    #[inline]
    pub const fn from_index(index: u8) -> Square {
        debug_assert!((index as usize) < NUM_SQUARES);
        Square(index)
    }

    /// Construct from `(file, rank)`, returning `None` if off-board.
    #[inline]
    pub const fn new(file: u8, rank: u8) -> Option<Square> {
        if file < FILES && rank < RANKS {
            Some(Square(rank * FILES + file))
        } else {
            None
        }
    }

    /// Same as [`Square::new`] but for signed inputs (move-gen convenience).
    #[inline]
    pub const fn try_new(file: i8, rank: i8) -> Option<Square> {
        if file >= 0 && file < FILES as i8 && rank >= 0 && rank < RANKS as i8 {
            Some(Square(rank as u8 * FILES + file as u8))
        } else {
            None
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 % FILES
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 / FILES
    }

    /// Iterate over all 90 squares.
    pub fn all() -> impl Iterator<Item = Square> {
        (0..NUM_SQUARES as u8).map(Square)
    }
}

/// Is this square inside the given color's palace (九宫: files 3..=5)?
#[inline]
pub fn in_palace(sq: Square, color: crate::Color) -> bool {
    let f = sq.file();
    let r = sq.rank();
    if !(3..=5).contains(&f) {
        return false;
    }
    match color {
        crate::Color::Red => r <= 2,
        crate::Color::Black => r >= 7,
    }
}

/// Has this square crossed the river from the color's home half?
#[inline]
pub fn crossed_river(sq: Square, color: crate::Color) -> bool {
    match color {
        crate::Color::Red => sq.rank() >= RIVER_BLACK_MIN_RANK,
        crate::Color::Black => sq.rank() <= RIVER_RED_MAX_RANK,
    }
}
