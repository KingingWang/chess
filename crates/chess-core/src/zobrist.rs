//! Zobrist hashing for fast, incremental board position hashing.
//!
//! Each (piece, color, square) triple and the side-to-move flag have a
//! pre-generated random `u64` key. A position hash is computed by XOR-ing
//! the relevant keys together. Incremental updates on make/unmake are O(1):
//! XOR out the moved piece's old square key, XOR in the new square key, and
//! flip the side-to-move key.
//!
//! The random values are generated at compile time from a fixed seed so
//! hashes are deterministic across builds.

use crate::piece::{Color, Piece, PieceKind};
use crate::square::{Square, NUM_SQUARES};

/// Number of distinct (color, piece_kind) combinations = 2 colors × 7 kinds.
const NUM_PIECES: usize = 14;

/// Pre-computed Zobrist keys.
pub struct ZobristKeys {
    /// `piece_sq[piece_index][square_index]` — one random u64 per
    /// (color, kind, square) triple.
    piece_sq: [[u64; NUM_SQUARES]; NUM_PIECES],
    /// XOR-ed into the hash when it is Black's turn.
    side: u64,
}

/// Map a piece (color + kind) to an index in `0..14`.
#[inline]
const fn piece_index(color: Color, kind: PieceKind) -> usize {
    let c = match color {
        Color::Red => 0,
        Color::Black => 7,
    };
    let k = match kind {
        PieceKind::King => 0,
        PieceKind::Advisor => 1,
        PieceKind::Elephant => 2,
        PieceKind::Horse => 3,
        PieceKind::Chariot => 4,
        PieceKind::Cannon => 5,
        PieceKind::Pawn => 6,
    };
    c + k
}

/// Simple xorshift64 PRNG for deterministic key generation at compile time.
const fn xorshift64(mut state: u64) -> (u64, u64) {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    (state, state)
}

/// Build all Zobrist keys at compile time.
const fn build_keys() -> ZobristKeys {
    let mut piece_sq = [[0u64; NUM_SQUARES]; NUM_PIECES];
    let mut state: u64 = 0x3243f6a8885a308d; // π-derived seed

    let mut p = 0;
    while p < NUM_PIECES {
        let mut s = 0;
        while s < NUM_SQUARES {
            let (next_state, val) = xorshift64(state);
            state = next_state;
            piece_sq[p][s] = val;
            s += 1;
        }
        p += 1;
    }

    let (_, side) = xorshift64(state);

    ZobristKeys { piece_sq, side }
}

/// Global Zobrist key table, computed at compile time.
static KEYS: ZobristKeys = build_keys();

/// Compute the full Zobrist hash for a board from scratch.
pub fn hash_board(board: &crate::board::Board) -> u64 {
    let mut h: u64 = 0;
    for (sq, piece) in board.pieces() {
        h ^= KEYS.piece_sq[piece_index(piece.color, piece.kind)][sq.index()];
    }
    if board.side_to_move() == Color::Black {
        h ^= KEYS.side;
    }
    h
}

/// The side-to-move toggle key (XOR to flip sides).
#[inline]
pub fn side_key() -> u64 {
    KEYS.side
}

/// Key for a specific piece on a specific square.
#[inline]
pub fn piece_square_key(piece: Piece, sq: Square) -> u64 {
    KEYS.piece_sq[piece_index(piece.color, piece.kind)][sq.index()]
}

/// Incrementally update a hash after a move:
/// - Remove the piece from its origin square.
/// - If there was a captured piece on the destination, remove it.
/// - Place the piece on the destination square.
/// - Flip the side to move.
#[inline]
pub fn update_hash(
    hash: u64,
    piece: Piece,
    from: Square,
    to: Square,
    captured: Option<Piece>,
) -> u64 {
    let mut h = hash;
    // Remove piece from origin.
    h ^= piece_square_key(piece, from);
    // Remove captured piece from destination (if any).
    if let Some(cap) = captured {
        h ^= piece_square_key(cap, to);
    }
    // Place piece on destination.
    h ^= piece_square_key(piece, to);
    // Flip side to move.
    h ^= side_key();
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::moves::Move;

    #[test]
    fn start_position_hash_is_nonzero() {
        let b = Board::start_position();
        assert_ne!(hash_board(&b), 0);
    }

    #[test]
    fn hash_is_deterministic() {
        let b = Board::start_position();
        assert_eq!(hash_board(&b), hash_board(&b));
    }

    #[test]
    fn different_positions_have_different_hashes() {
        let b1 = Board::start_position();
        let mut b2 = Board::start_position();
        let mv = Move::from_iccs("h2e2").unwrap();
        b2.make_move(mv);
        assert_ne!(hash_board(&b1), hash_board(&b2));
    }

    #[test]
    fn incremental_hash_matches_full_hash() {
        let b = Board::start_position();
        let h0 = hash_board(&b);

        let mv = Move::from_iccs("h2e2").unwrap();
        let piece = b.piece_at(mv.from).unwrap();
        let captured = b.piece_at(mv.to);

        let h_inc = update_hash(h0, piece, mv.from, mv.to, captured);

        let mut b2 = b.clone();
        b2.make_move(mv);
        let h_full = hash_board(&b2);

        assert_eq!(h_inc, h_full, "incremental hash must equal full recompute");
    }

    #[test]
    fn make_unmake_restores_hash() {
        let b = Board::start_position();
        let h0 = hash_board(&b);

        let mv = Move::from_iccs("h2e2").unwrap();
        let mut b2 = b.clone();
        let undo = b2.make_move(mv);
        b2.unmake_move(undo);
        let h1 = hash_board(&b2);

        assert_eq!(h0, h1, "hash after make+unmake must equal original");
    }

    #[test]
    fn capture_hash_is_correct() {
        // Create a position with a capturable piece.
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(8, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 4).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        b.set_piece(
            Square::new(4, 8).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::Cannon)),
        );
        b.set_side_to_move(Color::Red);

        let h0 = hash_board(&b);
        let mv = Move::new(Square::new(4, 4).unwrap(), Square::new(4, 8).unwrap());
        let piece = b.piece_at(mv.from).unwrap();
        let captured = b.piece_at(mv.to);

        let h_inc = update_hash(h0, piece, mv.from, mv.to, captured);

        let mut b2 = b.clone();
        b2.make_move(mv);
        let h_full = hash_board(&b2);

        assert_eq!(h_inc, h_full, "capture hash must match full recompute");
    }
}
