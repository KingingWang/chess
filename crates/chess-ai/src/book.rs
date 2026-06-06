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
        // Comprehensive Xiangqi opening book with common variations.
        // Each line is a sequence of ICCS moves with weights.
        let lines: &[(&[&str], &[u32])] = &[
            // ===== 中炮开局 (Central Cannon) =====
            (&["h2e2"], &[100]),
            // Black responses to 中炮
            (&["h2e2", "h9g7"], &[100, 90]), // 屏风马
            (&["h2e2", "b9c7"], &[100, 85]), // 反宫马
            (&["h2e2", "c9e7"], &[100, 70]), // 飞象
            (&["h2e2", "a9a8"], &[100, 60]), // 横车
            // 中炮 → 屏风马 variations
            (&["h2e2", "h9g7", "b0c2"], &[100, 90, 95]), // 马八进七
            (&["h2e2", "h9g7", "h0g2"], &[100, 90, 90]), // 马二进三
            (&["h2e2", "h9g7", "a0a1"], &[100, 90, 75]), // 车一进一
            (&["h2e2", "h9g7", "b0c2", "b7b6"], &[100, 90, 95, 85]),
            (&["h2e2", "h9g7", "b0c2", "i9h9"], &[100, 90, 95, 80]),
            (&["h2e2", "h9g7", "h0g2", "b9c7"], &[100, 90, 90, 85]),
            (&["h2e2", "h9g7", "h0g2", "i9h9"], &[100, 90, 90, 80]),
            // 中炮 → 反宫马 variations
            (&["h2e2", "b9c7", "b0c2"], &[100, 85, 90]),
            (&["h2e2", "b9c7", "h0g2"], &[100, 85, 85]),
            (&["h2e2", "b9c7", "b0c2", "h9g7"], &[100, 85, 90, 90]),
            (&["h2e2", "b9c7", "b0c2", "a9a8"], &[100, 85, 90, 75]),
            // ===== 仙人指路 (Pawn to Center) =====
            (&["e3e4"], &[80]),
            // Black responses
            (&["e3e4", "e6e5"], &[80, 90]), // 对兵
            (&["e3e4", "h9g7"], &[80, 85]), // 马8进7
            (&["e3e4", "b9c7"], &[80, 80]), // 马2进3
            (&["e3e4", "c9e7"], &[80, 70]), // 飞象
            // 仙人指路 → 对兵 variations
            (&["e3e4", "e6e5", "b0c2"], &[80, 90, 85]),
            (&["e3e4", "e6e5", "h0g2"], &[80, 90, 80]),
            (&["e3e4", "e6e5", "b0c2", "b9c7"], &[80, 90, 85, 85]),
            (&["e3e4", "e6e5", "h0g2", "h9g7"], &[80, 90, 80, 85]),
            // 仙人指路 → 马8进7 variations
            (&["e3e4", "h9g7", "b0c2"], &[80, 85, 85]),
            (&["e3e4", "h9g7", "h0g2"], &[80, 85, 80]),
            (&["e3e4", "h9g7", "b0c2", "i9h9"], &[80, 85, 85, 80]),
            // ===== 飞相局 (Elephant Opening) =====
            (&["c0e2"], &[70]),
            // Black responses
            (&["c0e2", "h9g7"], &[70, 85]), // 马8进7
            (&["c0e2", "b9c7"], &[70, 80]), // 马2进3
            (&["c0e2", "c6c5"], &[70, 75]), // 卒3进1
            (&["c0e2", "h9g7", "b0c2"], &[70, 85, 85]),
            (&["c0e2", "h9g7", "h0g2"], &[70, 85, 80]),
            (&["c0e2", "b9c7", "b0c2"], &[70, 80, 85]),
            (&["c0e2", "b9c7", "h0g2"], &[70, 80, 80]),
            // ===== 起马局 (Horse Opening) =====
            // Queenside horse
            (&["b0c2"], &[60]),
            (&["b0c2", "b9c7"], &[60, 80]),
            (&["b0c2", "h9g7"], &[60, 85]),
            (&["b0c2", "c6c5"], &[60, 75]),
            (&["b0c2", "b9c7", "c3c4"], &[60, 80, 75]),
            (&["b0c2", "h9g7", "c3c4"], &[60, 85, 75]),
            // Kingside horse
            (&["h0g2"], &[55]),
            (&["h0g2", "h9g7"], &[55, 85]),
            (&["h0g2", "b9c7"], &[55, 80]),
            (&["h0g2", "h9g7", "g3g4"], &[55, 85, 75]),
            (&["h0g2", "b9c7", "g3g4"], &[55, 80, 75]),
            // ===== 仕角炮 (Palace Cannon) =====
            (&["b2d2"], &[50]),
            (&["b2d2", "h9g7"], &[50, 85]),
            (&["b2d2", "b9c7"], &[50, 80]),
            (&["b2d2", "h9g7", "h0g2"], &[50, 85, 80]),
            // Note: Cross-palace cannon (过宫炮) requires specific setup, not included in basic book
            // ===== 边马局 (Edge Horse) =====
            (&["a0a1"], &[40]),
            (&["a0a1", "h9g7"], &[40, 85]),
            (&["a0a1", "b9c7"], &[40, 80]),
            // ===== 进边兵 (Edge Pawn Advance) =====
            (&["a3a4"], &[35]),
            (&["a3a4", "h9g7"], &[35, 85]),
            (&["a3a4", "b9c7"], &[35, 80]),
            // ===== 五六炮 (5-6 Cannon) =====
            (
                &["h2e2", "h9g7", "b0c2", "i9h9", "a0b0"],
                &[100, 90, 95, 80, 85],
            ),
            (
                &["h2e2", "h9g7", "h0g2", "i9h9", "a0b0"],
                &[100, 90, 90, 80, 85],
            ),
            // ===== 五七炮 (5-7 Cannon) =====
            (
                &["h2e2", "h9g7", "b0c2", "i9h9", "a0a1"],
                &[100, 90, 95, 80, 80],
            ),
            (
                &["h2e2", "h9g7", "h0g2", "i9h9", "a0a1"],
                &[100, 90, 90, 80, 80],
            ),
            // ===== 巡河炮 (River Patrol Cannon) =====
            (
                &["h2e2", "h9g7", "b0c2", "b7b6", "h7h8"],
                &[100, 90, 95, 85, 75],
            ),
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

#[cfg(test)]
mod expansion_tests {
    use super::*;

    #[test]
    fn book_has_multiple_opening_moves() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();
        let candidates = book.entries.get(&b.to_fen());
        assert!(candidates.is_some());
        let candidates = candidates.unwrap();
        // Should have at least 5 different opening moves
        assert!(
            candidates.len() >= 5,
            "Expected at least 5 opening moves, got {}",
            candidates.len()
        );
    }

    #[test]
    fn book_covers_central_cannon_variations() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Play 中炮 h2e2
        let mv = Move::from_iccs("h2e2").unwrap();
        let mut b = b.clone();
        b.make_move(mv);

        // Should have responses
        let candidates = book.entries.get(&b.to_fen());
        assert!(candidates.is_some(), "Should have responses to 中炮");
        assert!(
            candidates.unwrap().len() >= 3,
            "Should have multiple responses"
        );
    }

    #[test]
    fn book_covers_pawn_opening() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Play 仙人指路 e3e4
        let mv = Move::from_iccs("e3e4").unwrap();
        let mut b = b.clone();
        b.make_move(mv);

        let candidates = book.entries.get(&b.to_fen());
        assert!(candidates.is_some(), "Should have responses to 仙人指路");
    }

    #[test]
    fn book_covers_elephant_opening() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Play 飞相 c0e2
        let mv = Move::from_iccs("c0e2").unwrap();
        let mut b = b.clone();
        b.make_move(mv);

        let candidates = book.entries.get(&b.to_fen());
        assert!(candidates.is_some(), "Should have responses to 飞相");
    }

    #[test]
    fn book_covers_horse_openings() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Test both horse openings
        for mv_str in &["b0c2", "h0g2"] {
            let mv = Move::from_iccs(mv_str).unwrap();
            let mut board = b.clone();
            board.make_move(mv);
            let candidates = book.entries.get(&board.to_fen());
            assert!(candidates.is_some(), "Should have responses to {}", mv_str);
        }
    }

    #[test]
    fn book_all_moves_are_legal() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Play a few moves and check all book moves are legal
        let moves = ["h2e2", "h9g7", "b0c2"];
        let mut board = b.clone();

        for mv_str in &moves {
            let fen = board.to_fen();
            if let Some(candidates) = book.entries.get(&fen) {
                let legal_moves = board.legal_moves();
                for bm in candidates {
                    assert!(
                        legal_moves.contains(&bm.mv),
                        "Book move {} is not legal in position {}",
                        bm.mv.to_iccs(),
                        fen
                    );
                }
            }
            let mv = Move::from_iccs(mv_str).unwrap();
            board.make_move(mv);
        }
    }

    #[test]
    fn book_depth_covers_multiple_plies() {
        let book = OpeningBook::default_book();
        let b = Board::start_position();

        // Check that book has entries at different depths
        let mut board = b.clone();
        let moves = ["h2e2", "h9g7", "b0c2", "i9h9"];

        let mut found_depths = 0;
        for mv_str in &moves {
            let fen = board.to_fen();
            if book.entries.contains_key(&fen) {
                found_depths += 1;
            }
            let mv = Move::from_iccs(mv_str).unwrap();
            board.make_move(mv);
        }

        assert!(
            found_depths >= 3,
            "Book should cover at least 3 plies deep, found {}",
            found_depths
        );
    }
}
