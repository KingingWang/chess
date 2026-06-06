//! Endgame knowledge base for Xiangqi.
//!
//! Provides specialized evaluation for known endgame positions where
//! the general evaluation function may not be accurate. This helps
//! the engine recognize theoretical wins and draws.

use chess_core::{Board, Color, PieceKind};

/// Result of endgame classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndgameResult {
    /// Theoretical win for the specified side.
    Win(Color),
    /// Theoretical draw.
    Draw,
    /// Not a known endgame position (use general evaluation).
    Unknown,
}

/// Count pieces by type for a given color.
fn count_pieces(board: &Board, color: Color) -> [usize; 7] {
    let mut counts = [0usize; 7];
    for (_, piece) in board.pieces() {
        if piece.color == color {
            let idx = match piece.kind {
                PieceKind::King => 0,
                PieceKind::Advisor => 1,
                PieceKind::Elephant => 2,
                PieceKind::Horse => 3,
                PieceKind::Chariot => 4,
                PieceKind::Cannon => 5,
                PieceKind::Pawn => 6,
            };
            counts[idx] += 1;
        }
    }
    counts
}

/// Check if position is a simple endgame (few pieces remaining).
fn is_simple_endgame(red_counts: &[usize; 7], black_counts: &[usize; 7]) -> bool {
    // Count non-king pieces
    let red_pieces: usize = red_counts[1..].iter().sum();
    let black_pieces: usize = black_counts[1..].iter().sum();

    // Simple endgame if both sides have at most 3 non-king pieces
    // and no side has more than 1 chariot
    red_pieces <= 3 && black_pieces <= 3 && red_counts[4] <= 1 && black_counts[4] <= 1
}

/// Classify the endgame position.
pub fn classify_endgame(board: &Board) -> EndgameResult {
    let red_counts = count_pieces(board, Color::Red);
    let black_counts = count_pieces(board, Color::Black);

    // Verify both kings are present
    if red_counts[0] != 1 || black_counts[0] != 1 {
        return EndgameResult::Unknown;
    }

    // Check if it's a simple endgame
    if !is_simple_endgame(&red_counts, &black_counts) {
        return EndgameResult::Unknown;
    }

    // Get attacking pieces (excluding king)
    let red_attackers: usize = red_counts[1..].iter().sum();
    let black_attackers: usize = black_counts[1..].iter().sum();

    // King vs King - draw
    if red_attackers == 0 && black_attackers == 0 {
        return EndgameResult::Draw;
    }

    // Check theoretical draws
    if red_attackers == 0 && black_attackers == 0 {
        return EndgameResult::Draw;
    }

    // Single minor piece vs King - draw
    if red_attackers == 0 {
        // Black has pieces, Red has only king
        if black_attackers == 1 {
            // Single advisor, elephant, horse, or cannon cannot force mate
            if black_counts[1] == 1
                || black_counts[2] == 1
                || black_counts[3] == 1
                || black_counts[5] == 1
            {
                return EndgameResult::Draw;
            }
            // Single pawn - depends on position, but generally weak
            if black_counts[6] == 1 {
                return EndgameResult::Unknown;
            }
        }
    }

    if black_attackers == 0 {
        // Red has pieces, Black has only king
        if red_attackers == 1 {
            // Single advisor, elephant, horse, or cannon cannot force mate
            if red_counts[1] == 1 || red_counts[2] == 1 || red_counts[3] == 1 || red_counts[5] == 1
            {
                return EndgameResult::Draw;
            }
            // Single pawn - depends on position
            if red_counts[6] == 1 {
                return EndgameResult::Unknown;
            }
        }
    }

    // Check theoretical wins
    // Single Chariot vs King - win
    if red_attackers == 1 && black_attackers == 0 && red_counts[4] == 1 {
        return EndgameResult::Win(Color::Red);
    }
    if black_attackers == 1 && red_attackers == 0 && black_counts[4] == 1 {
        return EndgameResult::Win(Color::Black);
    }

    // Chariot vs minor pieces - likely win for chariot side
    if red_counts[4] == 1 && black_counts[4] == 0 && red_attackers == 1 && black_attackers <= 2 {
        return EndgameResult::Win(Color::Red);
    }
    if black_counts[4] == 1 && red_counts[4] == 0 && black_attackers == 1 && red_attackers <= 2 {
        return EndgameResult::Win(Color::Black);
    }

    // Double Horse vs King - win
    if red_counts[3] == 2 && red_attackers == 2 && black_attackers == 0 {
        return EndgameResult::Win(Color::Red);
    }
    if black_counts[3] == 2 && black_attackers == 2 && red_attackers == 0 {
        return EndgameResult::Win(Color::Black);
    }

    // Horse + Cannon vs King - win
    if red_counts[3] == 1 && red_counts[5] == 1 && red_attackers == 2 && black_attackers == 0 {
        return EndgameResult::Win(Color::Red);
    }
    if black_counts[3] == 1 && black_counts[5] == 1 && black_attackers == 2 && red_attackers == 0 {
        return EndgameResult::Win(Color::Black);
    }

    // Not a known endgame
    EndgameResult::Unknown
}

/// Get evaluation bonus/penalty for endgame positions.
/// Returns score in centipawns from Red's perspective.
pub fn endgame_eval(board: &Board) -> Option<i32> {
    match classify_endgame(board) {
        EndgameResult::Win(Color::Red) => Some(800), // Big bonus for theoretical win
        EndgameResult::Win(Color::Black) => Some(-800),
        EndgameResult::Draw => Some(0), // Neutral for draws
        EndgameResult::Unknown => None, // Use general evaluation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Board;

    #[test]
    fn king_vs_king_is_draw() {
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4K4 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Draw);
    }

    #[test]
    fn single_chariot_wins() {
        // Red chariot vs Black king
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4KR3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Win(Color::Red));
    }

    #[test]
    fn single_horse_draws() {
        // Red horse vs Black king - theoretical draw
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4KN3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Draw);
    }

    #[test]
    fn double_horse_wins() {
        // Red double horse vs Black king
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/3KN1N2 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Win(Color::Red));
    }

    #[test]
    fn horse_cannon_wins() {
        // Red horse + cannon vs Black king
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/3KN1C2 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Win(Color::Red));
    }

    #[test]
    fn single_cannon_draws() {
        // Red cannon vs Black king - needs screen piece, generally draw
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4KC3 w - - 0 1").unwrap();
        assert_eq!(classify_endgame(&board), EndgameResult::Draw);
    }

    #[test]
    fn start_position_unknown() {
        let board = Board::start_position();
        assert_eq!(classify_endgame(&board), EndgameResult::Unknown);
    }

    #[test]
    fn endgame_eval_returns_correct_scores() {
        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4KR3 w - - 0 1").unwrap();
        assert_eq!(endgame_eval(&board), Some(800));

        let board = Board::from_fen("4k4/9/9/9/9/9/9/9/9/4K4 w - - 0 1").unwrap();
        assert_eq!(endgame_eval(&board), Some(0));

        let board = Board::start_position();
        assert_eq!(endgame_eval(&board), None);
    }
}
