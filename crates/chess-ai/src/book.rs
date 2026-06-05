//! Simple opening book for Xiangqi.
//!
//! Stores a small set of commonly played openings as sequences of ICCS moves.
//! When the current game position matches a book line, a book move is returned
//! instead of running the search engine. This provides instant, strong opening
//! play for the built-in engine.

use chess_core::{Board, Move};
use std::collections::HashMap;

/// An opening book mapping position FENs to candidate moves with weights.
pub struct OpeningBook {
    /// Map from position FEN (placement+side only) to candidate moves.
    entries: HashMap<String, Vec<BookMove>>,
}

/// A single book move with a selection weight.
#[derive(Debug, Clone)]
struct BookMove {
    mv: Move,
    weight: u32,
}

impl OpeningBook {
    /// Build the default opening book with common Xiangqi openings.
    pub fn default_book() -> Self {
        let mut book = OpeningBook {
            entries: HashMap::new(),
        };
        book.add_openings();
        book
    }

    /// Look up a book move for the given position. Returns the highest-weighted
    /// legal move, or `None` if the position is not in the book.
    pub fn lookup(&self, board: &Board) -> Option<Move> {
        let fen = board.to_fen();
        let candidates = self.entries.get(&fen)?;
        // Find the highest-weighted legal move.
        let legal = board.legal_moves();
        candidates
            .iter()
            .filter(|bm| legal.contains(&bm.mv))
            .max_by_key(|bm| bm.weight)
            .map(|bm| bm.mv)
    }

    /// Add a single entry to the book.
    fn add(&mut self, fen: &str, mv_iccs: &str, weight: u32) {
        if let Some(mv) = Move::from_iccs(mv_iccs) {
            self.entries
                .entry(fen.to_string())
                .or_default()
                .push(BookMove { mv, weight });
        }
    }

    /// Build common openings by replaying move sequences from the start
    /// position. Each opening line records the FEN at each position so the
    /// book automatically handles transpositions.
    fn add_openings(&mut self) {
        // Define opening lines as sequences of ICCS moves.
        let lines: &[(&[&str], &[u32])] = &[
            // 中炮开局 (Central Cannon)
            (&["h2e2"], &[100]),
            // 仙人指路 (Pawn to center)
            (&["e3e4"], &[80]),
            // 飞相局 (Elephant opening)
            (&["c0e2"], &[70]),
            // 起马局 (Horse opening, queenside)
            (&["b0c2"], &[60]),
            // 起马局 (Horse opening, kingside)
            (&["h0g2"], &[55]),
            // 中炮 → Black 马8进7
            (&["h2e2", "h9g7"], &[100, 85]),
            // 中炮 → Black 马2进3
            (&["h2e2", "b9c7"], &[100, 80]),
            // 中炮 → 马8进7 → Red 马八进七
            (&["h2e2", "h9g7", "b0c2"], &[100, 85, 90]),
            // 中炮 → 马8进7 → Red 马二进三
            (&["h2e2", "h9g7", "h0g2"], &[100, 85, 85]),
            // 仙人指路 → 对兵
            (&["e3e4", "e6e5"], &[80, 90]),
            // 仙人指路 → 马8进7
            (&["e3e4", "h9g7"], &[80, 85]),
            // 飞相 → 马8进7
            (&["c0e2", "h9g7"], &[70, 85]),
        ];

        for (moves, weights) in lines {
            let mut board = Board::start_position();
            for (i, mv_str) in moves.iter().enumerate() {
                let fen = board.to_fen();
                if let Some(mv) = Move::from_iccs(mv_str) {
                    let w = weights.get(i).copied().unwrap_or(50);
                    self.add(&fen, mv_str, w);
                    board.make_move(mv);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Board;

    #[test]
    fn book_has_start_position_moves() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();
        let mv = book.lookup(&b);
        assert!(mv.is_some(), "book should have moves for start position");
    }

    #[test]
    fn book_returns_legal_move() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();
        let mv = book.lookup(&b).unwrap();
        assert!(b.is_legal(mv), "book move must be legal");
    }

    #[test]
    fn book_returns_none_for_unknown() {
        let book = OpeningBook::default_book();
        let b = Board::empty();
        assert!(book.lookup(&b).is_none());
    }
}
