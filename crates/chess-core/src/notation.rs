//! Chinese chess notation (中文纵线格式) for Xiangqi moves.
//!
//! Implements the standard Chinese notation format used in tournament records:
//! - 炮二平五 (Cannon from file 2 traverses to file 5)
//! - 马8进7 (Horse from file 8 advances to file 7)
//! - 前车进三 (Front chariot advances 3 ranks)
//!
//! # Notation rules:
//! - **Red** uses Chinese numerals: 一二三四五六七八九 (right-to-left, 1-9)
//! - **Black** uses Arabic numerals: 1-9 (right-to-left)
//! - **Actions**: 进 (advance), 退 (retreat), 平 (traverse/horizontal)
//! - **平** is used only by line pieces (Chariot, Cannon, King, Pawn) for
//!   horizontal moves — the destination file number follows.
//! - **进/退** for line pieces: the number of ranks traversed.
//! - **进/退** for diagonal pieces (Horse, Elephant, Advisor): the destination
//!   file number.
//! - **Disambiguation**: When two same-type pieces are on the same file, use
//!   前 (front) / 后 (rear) instead of the file number.

use crate::board::Board;
use crate::moves::Move;
use crate::piece::{Color, Piece, PieceKind};
use crate::square::Square;

/// Chinese numerals for Red's file numbering (index 0 = unused, 1-9 valid).
const CHINESE_NUMS: [char; 10] = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];

/// Convert a move to standard Chinese notation given the board state BEFORE the move.
///
/// # Panics
/// Panics if there is no piece at `mv.from` on `board_before`.
pub fn move_to_chinese(mv: Move, board_before: &Board) -> String {
    let piece = board_before
        .piece_at(mv.from)
        .expect("move_to_chinese: no piece at from square");
    let color = piece.color;
    let kind = piece.kind;

    // Determine if disambiguation is needed (two same-type, same-color pieces
    // on the same file).
    let needs_disambiguation = has_same_piece_on_file(board_before, mv.from, piece);

    // Build the 4-character notation string.
    let mut result = String::with_capacity(8);

    // Part 1: Piece identity (piece glyph + file number, or 前/后 + piece glyph).
    if needs_disambiguation {
        let prefix = disambiguation_prefix(board_before, mv.from, piece);
        result.push(prefix);
        result.push(piece_glyph(kind, color));
    } else {
        result.push(piece_glyph(kind, color));
        result.push(file_to_char(mv.from.file(), color));
    }

    // Part 2: Action + destination.
    let from_rank = mv.from.rank() as i8;
    let to_rank = mv.to.rank() as i8;
    let from_file = mv.from.file();
    let to_file = mv.to.file();

    let forward = match color {
        Color::Red => 1i8,
        Color::Black => -1i8,
    };

    if from_rank == to_rank {
        // Horizontal move → 平.
        result.push('平');
        result.push(file_to_char(to_file, color));
    } else {
        // Vertical or diagonal move.
        let rank_diff = (to_rank - from_rank) * forward;
        let action = if rank_diff > 0 { '进' } else { '退' };
        result.push(action);

        // For line pieces (Chariot, Cannon, King, Pawn moving straight): rank distance.
        // For diagonal pieces (Horse, Elephant, Advisor): destination file.
        if is_line_piece(kind) && from_file == to_file {
            // Straight advance/retreat: count of ranks.
            let distance = rank_diff.unsigned_abs();
            result.push(number_to_char(distance, color));
        } else {
            // Diagonal movement: destination file.
            result.push(file_to_char(to_file, color));
        }
    }

    result
}

/// Check if there is another piece of the same type and color on the same file.
fn has_same_piece_on_file(board: &Board, sq: Square, piece: Piece) -> bool {
    let file = sq.file();
    for (other_sq, other_piece) in board.pieces() {
        if other_sq == sq {
            continue;
        }
        if other_piece == piece && other_sq.file() == file {
            return true;
        }
    }
    false
}

/// Determine the disambiguation prefix: 前 (front/advance side) or 后 (rear).
/// "Front" means closer to the opponent's side (higher rank for Red, lower for Black).
fn disambiguation_prefix(board: &Board, sq: Square, piece: Piece) -> char {
    let file = sq.file();
    let my_rank = sq.rank();

    // Find the other same piece on the same file.
    for (other_sq, other_piece) in board.pieces() {
        if other_sq == sq {
            continue;
        }
        if other_piece == piece && other_sq.file() == file {
            let other_rank = other_sq.rank();
            // "前" = the one more advanced (closer to opponent).
            let is_front = match piece.color {
                Color::Red => my_rank > other_rank,
                Color::Black => my_rank < other_rank,
            };
            return if is_front { '前' } else { '后' };
        }
    }
    // Should not reach here if has_same_piece_on_file was true.
    '前'
}

/// Convert a file number (0-8) to the notation character.
/// Files are numbered right-to-left from each side's perspective.
fn file_to_char(file: u8, color: Color) -> char {
    // Red's perspective: file 8 = "一" (1), file 0 = "九" (9).
    // Black's perspective: file 0 = '1', file 8 = '9'.
    let num = match color {
        Color::Red => 9 - file,   // file 0→9, file 8→1
        Color::Black => file + 1, // file 0→1, file 8→9
    };
    number_to_char(num, color)
}

/// Convert a number (1-9) to the appropriate character for the color.
fn number_to_char(num: u8, color: Color) -> char {
    match color {
        Color::Red => CHINESE_NUMS[num as usize],
        Color::Black => char::from_digit(num as u32, 10).unwrap_or('?'),
    }
}

/// Get the piece glyph for notation (uses the side-appropriate character).
fn piece_glyph(kind: PieceKind, color: Color) -> char {
    Piece::new(color, kind).glyph()
}

/// Is this a "line piece" that moves in straight lines (files/ranks)?
/// Line pieces report rank-distance for 进/退; diagonal pieces report dest file.
fn is_line_piece(kind: PieceKind) -> bool {
    matches!(
        kind,
        PieceKind::Chariot | PieceKind::Cannon | PieceKind::King | PieceKind::Pawn
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Board, Game, Move, START_FEN};

    #[test]
    fn cannon_center_opening() {
        // Red's cannon from h-file (file 7) to center (file 4): "炮二平五".
        // Red file 7 → "二" (9-7=2), traverse to file 4 → "五" (9-4=5).
        let b = Board::start_position();
        let mv = Move::from_iccs("h2e2").unwrap(); // file 7 rank 2 → file 4 rank 2
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "炮二平五", "got: {notation}");
    }

    #[test]
    fn horse_opening() {
        // Red horse from b0 (file 1, rank 0) to c2 (file 2, rank 2): "马八进七".
        // Red file 1 → "八" (9-1=8), advance to file 2 → "七" (9-2=7).
        let b = Board::start_position();
        let mv = Move::from_iccs("b0c2").unwrap();
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "马八进七", "got: {notation}");
    }

    #[test]
    fn chariot_advance() {
        // Red chariot from a0 (file 0, rank 0) to a1 (file 0, rank 1): "车九进一".
        // File 0 → "九", advance 1 rank → "一".
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(0, 0).unwrap(), Square::new(0, 1).unwrap());
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "车九进一", "got: {notation}");
    }

    #[test]
    fn black_horse_opening() {
        // Black horse from h9 (file 7, rank 9) to g7 (file 6, rank 7): "馬8进7"
        // Black file 7 → "8" (7+1=8), advance to file 6 → "7" (6+1=7).
        // Black advances toward lower ranks, so rank 9→7 with forward=-1 means rank_diff = (7-9)*(-1) = 2 > 0 → 进.
        let b = Board::start_position();
        // Actually we need Black to move. Let's set up the board for Black's turn.
        let mut b2 = b.clone();
        b2.set_side_to_move(Color::Black);
        let mv = Move::from_iccs("h9g7").unwrap();
        let notation = move_to_chinese(mv, &b2);
        assert_eq!(notation, "馬8进7", "got: {notation}");
    }

    #[test]
    fn pawn_forward() {
        // Red pawn at e3 (file 4, rank 3) advances to e4 (file 4, rank 4): "兵五进一".
        let mut b = Board::empty();
        b.set_piece(
            Square::new(4, 3).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Pawn)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(4, 3).unwrap(), Square::new(4, 4).unwrap());
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "兵五进一", "got: {notation}");
    }

    #[test]
    fn disambiguation_front_rear() {
        // Two red chariots on the same file — test 前/后 disambiguation.
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 2).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(0, 5).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        // The front chariot (rank 5, closer to Black) moves: "前车进一".
        let mv = Move::new(Square::new(0, 5).unwrap(), Square::new(0, 6).unwrap());
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "前车进一", "got: {notation}");

        // The rear chariot (rank 2) moves: "后车进一".
        let mv2 = Move::new(Square::new(0, 2).unwrap(), Square::new(0, 3).unwrap());
        let notation2 = move_to_chinese(mv2, &b);
        assert_eq!(notation2, "后车进一", "got: {notation2}");
    }

    #[test]
    fn chariot_retreat() {
        // Red chariot at a5 (file 0, rank 5) retreats to a3 (file 0, rank 3): "车九退二".
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 5).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(0, 5).unwrap(), Square::new(0, 3).unwrap());
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "车九退二", "got: {notation}");
    }

    #[test]
    fn elephant_advance() {
        // Red elephant at c0 (file 2, rank 0) to a2 (file 0, rank 2): "相七进九".
        // File 2 → "七" (9-2=7), diagonal advance to file 0 → "九" (9-0=9).
        let mut b = Board::empty();
        b.set_piece(
            Square::new(2, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Elephant)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(2, 0).unwrap(), Square::new(0, 2).unwrap());
        let notation = move_to_chinese(mv, &b);
        assert_eq!(notation, "相七进九", "got: {notation}");
    }
}

/// Convert a move to algebraic notation (international format).
///
/// Format: Piece + FromFile + Action + Destination
/// - Pieces: K (King), R (Chariot), C (Cannon), H (Horse), E (Elephant), A (Advisor), P (Pawn)
/// - Files: 1-9 (from each side's perspective, right-to-left)
/// - Actions: + (advance), - (retreat), = (traverse/horizontal)
///
/// Examples:
/// - C2=5 (Cannon from file 2 traverses to file 5)
/// - H8+7 (Horse from file 8 advances to file 7)
/// - R1+1 (Chariot from file 1 advances 1 rank)
pub fn move_to_algebraic(mv: Move, board_before: &Board) -> String {
    let piece = board_before
        .piece_at(mv.from)
        .expect("move_to_algebraic: no piece at from square");
    let color = piece.color;
    let kind = piece.kind;

    let mut result = String::with_capacity(6);

    // Piece letter
    result.push(algebraic_piece_letter(kind));

    // Check for disambiguation
    let needs_disambiguation = has_same_piece_on_file(board_before, mv.from, piece);

    if needs_disambiguation {
        let prefix = algebraic_disambiguation(board_before, mv.from, piece);
        result.push(prefix);
    } else {
        // From file number
        let from_file = file_to_algebraic(mv.from.file(), color);
        result.push(from_file);
    }

    // Action and destination
    let from_rank = mv.from.rank() as i8;
    let to_rank = mv.to.rank() as i8;
    let from_file = mv.from.file();
    let to_file = mv.to.file();

    let forward = match color {
        Color::Red => 1i8,
        Color::Black => -1i8,
    };

    if from_rank == to_rank {
        // Horizontal move → =
        result.push('=');
        result.push(file_to_algebraic(to_file, color));
    } else {
        // Vertical or diagonal move
        let rank_diff = (to_rank - from_rank) * forward;
        let action = if rank_diff > 0 { '+' } else { '-' };
        result.push(action);

        // For line pieces: rank distance
        // For diagonal pieces: destination file
        if is_line_piece(kind) && from_file == to_file {
            let distance = rank_diff.unsigned_abs();
            result.push_str(&distance.to_string());
        } else {
            result.push(file_to_algebraic(to_file, color));
        }
    }

    result
}

/// Get algebraic piece letter.
fn algebraic_piece_letter(kind: PieceKind) -> char {
    match kind {
        PieceKind::King => 'K',
        PieceKind::Chariot => 'R',
        PieceKind::Cannon => 'C',
        PieceKind::Horse => 'H',
        PieceKind::Elephant => 'E',
        PieceKind::Advisor => 'A',
        PieceKind::Pawn => 'P',
    }
}

/// Convert file number to algebraic notation (1-9).
fn file_to_algebraic(file: u8, color: Color) -> char {
    let num = match color {
        Color::Red => 9 - file,   // file 0→9, file 8→1
        Color::Black => file + 1, // file 0→1, file 8→9
    };
    char::from_digit(num as u32, 10).unwrap_or('?')
}

/// Algebraic disambiguation: + (front) or - (rear).
fn algebraic_disambiguation(board: &Board, sq: Square, piece: Piece) -> char {
    let file = sq.file();
    let my_rank = sq.rank();

    for (other_sq, other_piece) in board.pieces() {
        if other_sq == sq {
            continue;
        }
        if other_piece == piece && other_sq.file() == file {
            let other_rank = other_sq.rank();
            let is_front = match piece.color {
                Color::Red => my_rank > other_rank,
                Color::Black => my_rank < other_rank,
            };
            return if is_front { '+' } else { '-' };
        }
    }
    '+'
}

#[cfg(test)]
mod algebraic_tests {
    use super::*;
    use crate::{Board, Piece, PieceKind, Square};

    #[test]
    fn cannon_center_opening_algebraic() {
        let b = Board::start_position();
        let mv = Move::from_iccs("h2e2").unwrap();
        let notation = move_to_algebraic(mv, &b);
        assert_eq!(notation, "C2=5", "got: {notation}");
    }

    #[test]
    fn horse_opening_algebraic() {
        let b = Board::start_position();
        let mv = Move::from_iccs("b0c2").unwrap();
        let notation = move_to_algebraic(mv, &b);
        assert_eq!(notation, "H8+7", "got: {notation}");
    }

    #[test]
    fn chariot_advance_algebraic() {
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(0, 0).unwrap(), Square::new(0, 1).unwrap());
        let notation = move_to_algebraic(mv, &b);
        assert_eq!(notation, "R9+1", "got: {notation}");
    }

    #[test]
    fn chariot_retreat_algebraic() {
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 5).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(0, 5).unwrap(), Square::new(0, 3).unwrap());
        let notation = move_to_algebraic(mv, &b);
        assert_eq!(notation, "R9-2", "got: {notation}");
    }

    #[test]
    fn pawn_forward_algebraic() {
        let mut b = Board::empty();
        b.set_piece(
            Square::new(4, 3).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Pawn)),
        );
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_side_to_move(Color::Red);

        let mv = Move::new(Square::new(4, 3).unwrap(), Square::new(4, 4).unwrap());
        let notation = move_to_algebraic(mv, &b);
        assert_eq!(notation, "P5+1", "got: {notation}");
    }
}
