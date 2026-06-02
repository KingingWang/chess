//! FEN (Forsyth–Edwards Notation) for Xiangqi.
//!
//! Layout is written from rank 9 (Black's back rank, top) down to rank 0
//! (Red's back rank, bottom), files left-to-right. `w`/`r` = Red to move,
//! `b` = Black to move.

use std::str::FromStr;

use crate::board::Board;
use crate::piece::{Color, Piece};
use crate::square::{Square, FILES, RANKS};

/// Standard opening position.
pub const START_FEN: &str = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FenError {
    #[error("expected 10 ranks, found {0}")]
    WrongRankCount(usize),
    #[error("rank {rank} has {found} files, expected {expected}")]
    WrongFileCount {
        rank: usize,
        found: usize,
        expected: usize,
    },
    #[error("unrecognized piece character: {0}")]
    BadPiece(char),
    #[error("missing placement field")]
    MissingPlacement,
    #[error("invalid side-to-move field: {0}")]
    BadSide(String),
}

impl Board {
    /// Parse the placement + side fields of a FEN string.
    pub fn from_fen(fen: &str) -> Result<Board, FenError> {
        let mut fields = fen.split_whitespace();
        let placement = fields.next().ok_or(FenError::MissingPlacement)?;

        let ranks: Vec<&str> = placement.split('/').collect();
        if ranks.len() != RANKS as usize {
            return Err(FenError::WrongRankCount(ranks.len()));
        }

        let mut board = Board::empty();
        // First rank token corresponds to rank 9 (top), last to rank 0.
        for (i, rank_str) in ranks.iter().enumerate() {
            let rank = (RANKS as usize - 1 - i) as u8;
            let mut file: u8 = 0;
            for c in rank_str.chars() {
                if let Some(d) = c.to_digit(10) {
                    file += d as u8;
                } else {
                    let piece = Piece::from_fen_char(c).ok_or(FenError::BadPiece(c))?;
                    let sq = Square::new(file, rank).ok_or(FenError::WrongFileCount {
                        rank: rank as usize,
                        found: file as usize + 1,
                        expected: FILES as usize,
                    })?;
                    board.set_piece(sq, Some(piece));
                    file += 1;
                }
            }
            if file != FILES {
                return Err(FenError::WrongFileCount {
                    rank: rank as usize,
                    found: file as usize,
                    expected: FILES as usize,
                });
            }
        }

        let side = fields.next().unwrap_or("w");
        board.set_side_to_move(match side {
            "w" | "r" | "R" => Color::Red,
            "b" | "B" => Color::Black,
            other => return Err(FenError::BadSide(other.to_string())),
        });

        Ok(board)
    }

    /// Serialize placement + side fields (counters fixed at `- - 0 1`).
    pub fn to_fen(&self) -> String {
        let mut out = String::new();
        for r in (0..RANKS).rev() {
            let mut empties = 0u8;
            for f in 0..FILES {
                let sq = Square::new(f, r).unwrap();
                match self.piece_at(sq) {
                    Some(p) => {
                        if empties > 0 {
                            out.push(char::from(b'0' + empties));
                            empties = 0;
                        }
                        out.push(p.to_fen_char());
                    }
                    None => empties += 1,
                }
            }
            if empties > 0 {
                out.push(char::from(b'0' + empties));
            }
            if r != 0 {
                out.push('/');
            }
        }
        let side = match self.side_to_move() {
            Color::Red => 'w',
            Color::Black => 'b',
        };
        out.push(' ');
        out.push(side);
        out.push_str(" - - 0 1");
        out
    }
}

impl FromStr for Board {
    type Err = FenError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Board::from_fen(s)
    }
}
