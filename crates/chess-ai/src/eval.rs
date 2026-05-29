//! Static evaluation for the built-in engine.
//!
//! Scores are in centipawns from the perspective of [`Color::Red`]; positive
//! favours Red. The search negates as appropriate. Values combine material
//! with simple piece-square tables that encode well-known Xiangqi heuristics
//! (central chariots, advanced/centralised horses & cannons, pawns gaining
//! value after crossing the river).

use chess_core::{Board, Color, PieceKind, Square};

/// A decisive (mate) score, kept well below `i32::MAX` so `±MATE - ply` stays
/// representable and orderable.
pub const MATE: i32 = 1_000_000;

#[inline]
fn material(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::King => 60_000,
        PieceKind::Chariot => 1_000,
        PieceKind::Cannon => 500,
        PieceKind::Horse => 450,
        PieceKind::Elephant => 200,
        PieceKind::Advisor => 200,
        PieceKind::Pawn => 100,
    }
}

// Piece-square tables are indexed from RED's point of view, rank 0 (Red back
// rank) first. For Black we mirror the rank. Files are symmetric so we only
// store the natural orientation. Values are small nudges (centipawns).

#[rustfmt::skip]
const PAWN_PST: [i32; 90] = [
    // rank 0..9, file 0..8
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     2,  0,  4,  0,  6,  0,  4,  0,  2,   // own pawn start rank
     6, 12, 18, 24, 30, 24, 18, 12,  6,   // just crossed
    20, 30, 40, 50, 55, 50, 40, 30, 20,
    20, 30, 45, 55, 60, 55, 45, 30, 20,
    20, 28, 40, 50, 55, 50, 40, 28, 20,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const HORSE_PST: [i32; 90] = [
     0, -4,  0,  0,  0,  0,  0, -4,  0,
    -4,  2,  6,  6,  4,  6,  6,  2, -4,
     2,  6, 10, 12, 12, 12, 10,  6,  2,
     4, 12, 14, 16, 16, 16, 14, 12,  4,
     6, 14, 18, 20, 22, 20, 18, 14,  6,
     6, 14, 18, 20, 22, 20, 18, 14,  6,
     4, 12, 16, 18, 18, 18, 16, 12,  4,
     2,  8, 12, 14, 14, 14, 12,  8,  2,
     0,  4,  8, 10, 10, 10,  8,  4,  0,
     0,  0,  4,  6,  6,  6,  4,  0,  0,
];

#[rustfmt::skip]
const CHARIOT_PST: [i32; 90] = [
    -2,  4,  4,  8,  8,  8,  4,  4, -2,
     4,  8,  8, 12, 12, 12,  8,  8,  4,
     2,  6,  6, 10, 10, 10,  6,  6,  2,
     6, 10, 10, 14, 14, 14, 10, 10,  6,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 14, 14, 18, 18, 18, 14, 14,  8,
    10, 14, 14, 18, 18, 18, 14, 14, 10,
     6, 10, 10, 14, 14, 14, 10, 10,  6,
];

#[rustfmt::skip]
const CANNON_PST: [i32; 90] = [
     2,  2,  0, -2, -4, -2,  0,  2,  2,
     0,  2,  0,  2,  4,  2,  0,  2,  0,
     2,  2,  0,  6,  8,  6,  0,  2,  2,
     0,  0,  2,  8, 10,  8,  2,  0,  0,
     0,  2,  4,  8, 12,  8,  4,  2,  0,
     0,  0,  4,  8, 12,  8,  4,  0,  0,
     2,  2,  4, 10, 12, 10,  4,  2,  2,
     2,  2,  4,  8, 10,  8,  4,  2,  2,
     0,  2,  2,  6,  8,  6,  2,  2,  0,
     0,  0,  2,  4,  6,  4,  2,  0,  0,
];

#[inline]
fn pst_value(kind: PieceKind, sq: Square, color: Color) -> i32 {
    // Mirror the rank for Black so both sides read the same "advance" table.
    let idx = match color {
        Color::Red => sq.index(),
        Color::Black => {
            let mirrored_rank = chess_core::square::RANKS - 1 - sq.rank();
            (mirrored_rank * chess_core::square::FILES + sq.file()) as usize
        }
    };
    match kind {
        PieceKind::Pawn => PAWN_PST[idx],
        PieceKind::Horse => HORSE_PST[idx],
        PieceKind::Chariot => CHARIOT_PST[idx],
        PieceKind::Cannon => CANNON_PST[idx],
        _ => 0,
    }
}

/// Static evaluation from `side`'s perspective (positive = good for `side`).
pub fn evaluate(board: &Board, side: Color) -> i32 {
    let mut red = 0i32;
    for (sq, piece) in board.pieces() {
        let v = material(piece.kind) + pst_value(piece.kind, sq, piece.color);
        match piece.color {
            Color::Red => red += v,
            Color::Black => red -= v,
        }
    }
    match side {
        Color::Red => red,
        Color::Black => -red,
    }
}
