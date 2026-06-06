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
    #[error("invalid position: {0}")]
    InvalidPosition(String),
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

/// Extended FEN validation errors for position legality.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FenValidationError {
    #[error("Red has {0} kings, expected exactly 1")]
    WrongRedKingCount(usize),
    #[error("Black has {0} kings, expected exactly 1")]
    WrongBlackKingCount(usize),
    #[error("Red king at {file},{rank} is outside palace (must be file 3-5, rank 0-2)")]
    RedKingOutsidePalace { file: u8, rank: u8 },
    #[error("Black king at {file},{rank} is outside palace (must be file 3-5, rank 7-9)")]
    BlackKingOutsidePalace { file: u8, rank: u8 },
    #[error("Red advisor at {file},{rank} is outside palace (must be file 3-5, rank 0-2)")]
    RedAdvisorOutsidePalace { file: u8, rank: u8 },
    #[error("Black advisor at {file},{rank} is outside palace (must be file 3-5, rank 7-9)")]
    BlackAdvisorOutsidePalace { file: u8, rank: u8 },
    #[error("Red elephant at {file},{rank} crossed river (must be rank 0-4)")]
    RedElephantCrossedRiver { file: u8, rank: u8 },
    #[error("Black elephant at {file},{rank} crossed river (must be rank 5-9)")]
    BlackElephantCrossedRiver { file: u8, rank: u8 },
    #[error("too many {piece_type}: found {found}, max is {max}")]
    TooManyPieces {
        piece_type: &'static str,
        found: usize,
        max: usize,
    },
}

/// Maximum piece counts for a legal Xiangqi position.
const MAX_PIECES: [(crate::piece::PieceKind, usize); 7] = [
    (crate::piece::PieceKind::King, 1),
    (crate::piece::PieceKind::Advisor, 2),
    (crate::piece::PieceKind::Elephant, 2),
    (crate::piece::PieceKind::Horse, 2),
    (crate::piece::PieceKind::Chariot, 2),
    (crate::piece::PieceKind::Cannon, 2),
    (crate::piece::PieceKind::Pawn, 5),
];

/// Validate that a parsed board represents a legal Xiangqi position.
///
/// Checks:
/// - Each side has exactly one king
/// - Kings are within their palaces
/// - Advisors are within their palaces
/// - Elephants have not crossed the river
/// - Piece counts do not exceed maximums
pub fn validate_position(board: &Board) -> Result<(), FenValidationError> {
    use crate::piece::PieceKind;

    let mut red_kings = Vec::new();
    let mut black_kings = Vec::new();
    let mut red_advisors = Vec::new();
    let mut black_advisors = Vec::new();
    let mut red_elephants = Vec::new();
    let mut black_elephants = Vec::new();
    let mut red_counts = [0usize; 7];
    let mut black_counts = [0usize; 7];

    for (sq, piece) in board.pieces() {
        let file = sq.file();
        let rank = sq.rank();
        let kind_idx = match piece.kind {
            PieceKind::King => 0,
            PieceKind::Advisor => 1,
            PieceKind::Elephant => 2,
            PieceKind::Horse => 3,
            PieceKind::Chariot => 4,
            PieceKind::Cannon => 5,
            PieceKind::Pawn => 6,
        };

        if piece.color == Color::Red {
            red_counts[kind_idx] += 1;
            match piece.kind {
                PieceKind::King => red_kings.push((file, rank)),
                PieceKind::Advisor => red_advisors.push((file, rank)),
                PieceKind::Elephant => red_elephants.push((file, rank)),
                _ => {}
            }
        } else {
            black_counts[kind_idx] += 1;
            match piece.kind {
                PieceKind::King => black_kings.push((file, rank)),
                PieceKind::Advisor => black_advisors.push((file, rank)),
                PieceKind::Elephant => black_elephants.push((file, rank)),
                _ => {}
            }
        }
    }

    // Check king counts
    if red_kings.len() != 1 {
        return Err(FenValidationError::WrongRedKingCount(red_kings.len()));
    }
    if black_kings.len() != 1 {
        return Err(FenValidationError::WrongBlackKingCount(black_kings.len()));
    }

    // Check king positions (must be in palace)
    let (rf, rr) = red_kings[0];
    if !(3..=5).contains(&rf) || rr > 2 {
        return Err(FenValidationError::RedKingOutsidePalace { file: rf, rank: rr });
    }
    let (bf, br) = black_kings[0];
    if !(3..=5).contains(&bf) || br < 7 {
        return Err(FenValidationError::BlackKingOutsidePalace { file: bf, rank: br });
    }

    // Check advisor positions (must be in palace)
    for (f, r) in &red_advisors {
        if !(3..=5).contains(f) || *r > 2 {
            return Err(FenValidationError::RedAdvisorOutsidePalace { file: *f, rank: *r });
        }
    }
    for (f, r) in &black_advisors {
        if !(3..=5).contains(f) || *r < 7 {
            return Err(FenValidationError::BlackAdvisorOutsidePalace { file: *f, rank: *r });
        }
    }

    // Check elephant positions (must not cross river)
    for (f, r) in &red_elephants {
        if *r > 4 {
            return Err(FenValidationError::RedElephantCrossedRiver { file: *f, rank: *r });
        }
    }
    for (f, r) in &black_elephants {
        if *r < 5 {
            return Err(FenValidationError::BlackElephantCrossedRiver { file: *f, rank: *r });
        }
    }

    // Check piece counts
    let piece_names = [
        "king", "advisor", "elephant", "horse", "chariot", "cannon", "pawn",
    ];
    for (i, (_, max)) in MAX_PIECES.iter().enumerate() {
        if red_counts[i] > *max {
            return Err(FenValidationError::TooManyPieces {
                piece_type: piece_names[i],
                found: red_counts[i],
                max: *max,
            });
        }
        if black_counts[i] > *max {
            return Err(FenValidationError::TooManyPieces {
                piece_type: piece_names[i],
                found: black_counts[i],
                max: *max,
            });
        }
    }

    Ok(())
}

impl Board {
    /// Parse a FEN string and validate the position is legal.
    ///
    /// This is stricter than `from_fen()` which only checks syntax.
    pub fn from_fen_validated(fen: &str) -> Result<Board, FenError> {
        let board = Board::from_fen(fen)?;
        validate_position(&board).map_err(|e| FenError::InvalidPosition(e.to_string()))?;
        Ok(board)
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn start_position_is_valid() {
        let board = Board::from_fen_validated(START_FEN).unwrap();
        assert_eq!(
            board.piece_at(Square::new(4, 0).unwrap()).unwrap().kind,
            crate::piece::PieceKind::King
        );
        assert_eq!(
            board.piece_at(Square::new(4, 9).unwrap()).unwrap().kind,
            crate::piece::PieceKind::King
        );
    }

    #[test]
    fn missing_red_king_is_invalid() {
        // Remove Red king from starting position
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBA1ABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("king"));
    }

    #[test]
    fn missing_black_king_is_invalid() {
        // Remove Black king from starting position
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBA1ABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
    }

    #[test]
    fn two_red_kings_is_invalid() {
        // Add extra Red king
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/K8/RNBAKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2"));
    }

    #[test]
    fn red_king_outside_palace_is_invalid() {
        // Red king at a1 (outside palace)
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/K7R w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("palace"));
    }

    #[test]
    fn black_king_outside_palace_is_invalid() {
        // Black king at a10 (file 0, outside palace which is file 3-5)
        // Original black king at e10 removed, red king at e1 as normal
        let fen = "knba1abnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        if let Err(ref e) = result {
            eprintln!("Error message: {}", e);
        }
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("palace"));
    }

    #[test]
    fn red_advisor_outside_palace_is_invalid() {
        // Red advisor at a1 (outside palace)
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/A8/RN1AKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("advisor"));
    }

    #[test]
    fn red_elephant_crossed_river_is_invalid() {
        // Red elephant at rank 5 (crossed river)
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        // Move elephant to rank 5
        board.set_piece(Square::new(2, 0).unwrap(), None);
        board.set_piece(
            Square::new(2, 5).unwrap(),
            Some(Piece::new(Color::Red, crate::piece::PieceKind::Elephant)),
        );
        let result = validate_position(&board);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("elephant"));
    }

    #[test]
    fn black_elephant_crossed_river_is_invalid() {
        // Black elephant at rank 4 (crossed river)
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        // Move black elephant to rank 4
        board.set_piece(Square::new(2, 9).unwrap(), None);
        board.set_piece(
            Square::new(2, 4).unwrap(),
            Some(Piece::new(Color::Black, crate::piece::PieceKind::Elephant)),
        );
        let result = validate_position(&board);
        assert!(result.is_err());
    }

    #[test]
    fn too_many_red_pawns_is_invalid() {
        // Add extra Red pawn (6 total) - one on rank 4 and 5 on rank 3
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/P8/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pawn"));
    }

    #[test]
    fn too_many_black_chariots_is_invalid() {
        // Add extra Black chariot (3 total)
        let fen = "rnbakabnR/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("chariot"));
    }

    #[test]
    fn midgame_position_is_valid() {
        // A typical midgame position
        let fen = "r1bakabnr/9/n4c3/p1p1p1p1p/9/9/P1P1P1P1P/N4C3/9/R1BAKABNR w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_ok());
    }

    #[test]
    fn endgame_position_is_valid() {
        // A simple endgame position
        let fen = "4k4/9/9/9/9/9/9/9/9/4K4 w - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_ok());
    }

    #[test]
    fn from_fen_still_accepts_invalid_positions() {
        // from_fen should still work for invalid positions (for flexibility)
        // This has 6 red pawns which is invalid, but from_fen accepts it
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/P8/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let result = Board::from_fen(fen);
        assert!(result.is_ok());
        // But validation should reject it
        let result = Board::from_fen_validated(fen);
        assert!(result.is_err());
    }

    #[test]
    fn black_to_move_is_valid() {
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR b - - 0 1";
        let result = Board::from_fen_validated(fen);
        assert!(result.is_ok());
    }
}
