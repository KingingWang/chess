//! Puzzle/tactics training mode.
//!
//! Allows users to practice tactical positions with guided solutions.

use bevy::prelude::*;
use chess_core::{Board, Color as ChessColor, Move};

/// A tactical puzzle.
#[derive(Debug, Clone)]
pub struct Puzzle {
    /// Unique identifier for the puzzle.
    pub id: String,
    /// Initial position in FEN format.
    pub fen: String,
    /// Side to move first.
    pub side_to_move: ChessColor,
    /// The solution moves (alternating between sides).
    pub solution: Vec<Move>,
    /// Difficulty rating (1-10).
    pub difficulty: u8,
    /// Description or theme (e.g., "Back rank mate", "Fork").
    pub theme: String,
    /// Current move index in the solution.
    pub current_move: usize,
}

impl Puzzle {
    /// Create a new puzzle.
    pub fn new(
        id: String,
        fen: String,
        side_to_move: ChessColor,
        solution: Vec<Move>,
        difficulty: u8,
        theme: String,
    ) -> Self {
        Self {
            id,
            fen,
            side_to_move,
            solution,
            difficulty,
            theme,
            current_move: 0,
        }
    }

    /// Get the next expected move in the solution.
    pub fn next_move(&self) -> Option<&Move> {
        self.solution.get(self.current_move)
    }

    /// Check if the puzzle is solved.
    pub fn is_solved(&self) -> bool {
        self.current_move >= self.solution.len()
    }

    /// Advance to the next move in the solution.
    pub fn advance(&mut self) {
        if self.current_move < self.solution.len() {
            self.current_move += 1;
        }
    }

    /// Reset the puzzle to the beginning.
    pub fn reset(&mut self) {
        self.current_move = 0;
    }

    /// Get the initial board position.
    pub fn initial_board(&self) -> Result<Board, String> {
        Board::from_fen(&self.fen).map_err(|e| format!("Invalid FEN: {}", e))
    }
}

/// Resource managing the puzzle mode state.
#[derive(Resource, Default)]
pub struct PuzzleMode {
    /// Whether puzzle mode is active.
    pub active: bool,
    /// Current puzzle being solved.
    pub current_puzzle: Option<Puzzle>,
    /// List of available puzzles.
    pub puzzles: Vec<Puzzle>,
    /// Current puzzle index in the list.
    pub puzzle_index: usize,
    /// Number of puzzles solved correctly.
    pub solved_count: usize,
    /// Number of attempts made.
    pub attempt_count: usize,
}

impl PuzzleMode {
    /// Add a puzzle to the collection.
    pub fn add_puzzle(&mut self, puzzle: Puzzle) {
        self.puzzles.push(puzzle);
    }

    /// Load the next puzzle.
    pub fn load_next_puzzle(&mut self) -> bool {
        if self.puzzles.is_empty() {
            return false;
        }

        self.puzzle_index = (self.puzzle_index + 1) % self.puzzles.len();
        self.current_puzzle = Some(self.puzzles[self.puzzle_index].clone());
        true
    }

    /// Load a specific puzzle by index.
    pub fn load_puzzle(&mut self, index: usize) -> bool {
        if index >= self.puzzles.len() {
            return false;
        }

        self.puzzle_index = index;
        self.current_puzzle = Some(self.puzzles[index].clone());
        true
    }

    /// Check if a move is correct for the current puzzle.
    pub fn check_move(&mut self, mv: Move) -> bool {
        if let Some(puzzle) = &mut self.current_puzzle {
            self.attempt_count += 1;

            if let Some(expected) = puzzle.next_move() {
                if *expected == mv {
                    puzzle.advance();

                    if puzzle.is_solved() {
                        self.solved_count += 1;
                    }

                    return true;
                }
            }
        }
        false
    }

    /// Reset the current puzzle.
    pub fn reset_current_puzzle(&mut self) {
        if let Some(puzzle) = &mut self.current_puzzle {
            puzzle.reset();
        }
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f32 {
        if self.attempt_count == 0 {
            return 0.0;
        }
        (self.solved_count as f32 / self.attempt_count as f32) * 100.0
    }

    /// Toggle puzzle mode.
    pub fn toggle(&mut self) {
        self.active = !self.active;
        if self.active && self.current_puzzle.is_none() && !self.puzzles.is_empty() {
            self.load_puzzle(0);
        }
    }
}

/// Component for puzzle UI elements.
#[derive(Component)]
pub struct PuzzleUI;

/// System to toggle puzzle mode.
pub fn toggle_puzzle_mode(
    mut puzzle_mode: ResMut<PuzzleMode>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        puzzle_mode.toggle();
    }
}

/// System to load sample puzzles on startup.
pub fn load_sample_puzzles(mut puzzle_mode: ResMut<PuzzleMode>) {
    if !puzzle_mode.puzzles.is_empty() {
        return;
    }

    // Add some sample puzzles
    // Puzzle 1: Back rank mate
    if let (Some(from), Some(to)) = (chess_core::Square::new(4, 0), chess_core::Square::new(4, 7)) {
        puzzle_mode.add_puzzle(Puzzle::new(
            "back_rank_1".to_string(),
            "4k4/9/9/9/9/9/9/9/9/4K3R w - - 0 1".to_string(),
            ChessColor::Red,
            vec![Move { from, to }],
            2,
            "Back rank mate".to_string(),
        ));
    }

    // Puzzle 2: Simple fork
    if let (Some(from), Some(to)) = (chess_core::Square::new(3, 0), chess_core::Square::new(3, 4)) {
        puzzle_mode.add_puzzle(Puzzle::new(
            "fork_1".to_string(),
            "4k4/9/9/9/9/9/9/9/9/3K5 w - - 0 1".to_string(),
            ChessColor::Red,
            vec![Move { from, to }],
            1,
            "Knight fork".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn create_test_puzzle() -> Puzzle {
        let from = Square::new(4, 0).unwrap();
        let to = Square::new(4, 7).unwrap();
        Puzzle::new(
            "test_1".to_string(),
            "4k4/9/9/9/9/9/9/9/9/4K4 w - - 0 1".to_string(),
            ChessColor::Red,
            vec![Move { from, to }],
            1,
            "Test puzzle".to_string(),
        )
    }

    #[test]
    fn test_puzzle_creation() {
        let puzzle = create_test_puzzle();
        assert_eq!(puzzle.id, "test_1");
        assert_eq!(puzzle.difficulty, 1);
        assert_eq!(puzzle.current_move, 0);
        assert!(!puzzle.is_solved());
    }

    #[test]
    fn test_puzzle_next_move() {
        let puzzle = create_test_puzzle();
        assert!(puzzle.next_move().is_some());

        let mut puzzle = puzzle;
        puzzle.advance();
        assert!(puzzle.next_move().is_none());
        assert!(puzzle.is_solved());
    }

    #[test]
    fn test_puzzle_reset() {
        let mut puzzle = create_test_puzzle();
        puzzle.advance();
        assert!(puzzle.is_solved());

        puzzle.reset();
        assert!(!puzzle.is_solved());
        assert_eq!(puzzle.current_move, 0);
    }

    #[test]
    fn test_puzzle_initial_board() {
        let puzzle = create_test_puzzle();
        let board = puzzle.initial_board();
        if let Err(e) = &board {
            println!("FEN parse error: {}", e);
        }
        assert!(board.is_ok());
    }

    #[test]
    fn test_puzzle_mode_default() {
        let mode = PuzzleMode::default();
        assert!(!mode.active);
        assert!(mode.current_puzzle.is_none());
        assert_eq!(mode.solved_count, 0);
        assert_eq!(mode.attempt_count, 0);
    }

    #[test]
    fn test_puzzle_mode_add_puzzle() {
        let mut mode = PuzzleMode::default();
        let puzzle = create_test_puzzle();

        mode.add_puzzle(puzzle);
        assert_eq!(mode.puzzles.len(), 1);
    }

    #[test]
    fn test_puzzle_mode_load_puzzle() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());

        assert!(mode.load_puzzle(0));
        assert!(mode.current_puzzle.is_some());

        assert!(!mode.load_puzzle(99));
    }

    #[test]
    fn test_puzzle_mode_load_next_puzzle() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.add_puzzle(create_test_puzzle());

        assert!(mode.load_next_puzzle());
        assert_eq!(mode.puzzle_index, 1);

        // Wrap around
        assert!(mode.load_next_puzzle());
        assert_eq!(mode.puzzle_index, 0);
    }

    #[test]
    fn test_puzzle_mode_check_move_correct() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.load_puzzle(0);

        let correct_move = mode.current_puzzle.as_ref().unwrap().solution[0];
        assert!(mode.check_move(correct_move));
        assert_eq!(mode.attempt_count, 1);
    }

    #[test]
    fn test_puzzle_mode_check_move_incorrect() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.load_puzzle(0);

        let wrong_from = Square::new(0, 0).unwrap();
        let wrong_to = Square::new(0, 1).unwrap();
        let wrong_move = Move {
            from: wrong_from,
            to: wrong_to,
        };

        assert!(!mode.check_move(wrong_move));
        assert_eq!(mode.attempt_count, 1);
    }

    #[test]
    fn test_puzzle_mode_solved_count() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.load_puzzle(0);

        let correct_move = mode.current_puzzle.as_ref().unwrap().solution[0];
        mode.check_move(correct_move);

        assert_eq!(mode.solved_count, 1);
    }

    #[test]
    fn test_puzzle_mode_success_rate() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.load_puzzle(0);

        // First attempt: correct
        let correct_move = mode.current_puzzle.as_ref().unwrap().solution[0];
        mode.check_move(correct_move);

        // Reset and try again
        mode.reset_current_puzzle();

        // Second attempt: wrong move
        let wrong_from = Square::new(0, 0).unwrap();
        let wrong_to = Square::new(0, 1).unwrap();
        let wrong_move = Move {
            from: wrong_from,
            to: wrong_to,
        };
        mode.check_move(wrong_move);

        // Success rate: 1 solved / 2 attempts = 50%
        assert!((mode.success_rate() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_puzzle_mode_toggle() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());

        assert!(!mode.active);
        mode.toggle();
        assert!(mode.active);
        assert!(mode.current_puzzle.is_some());

        mode.toggle();
        assert!(!mode.active);
    }

    #[test]
    fn test_puzzle_mode_reset_current_puzzle() {
        let mut mode = PuzzleMode::default();
        mode.add_puzzle(create_test_puzzle());
        mode.load_puzzle(0);

        let correct_move = mode.current_puzzle.as_ref().unwrap().solution[0];
        mode.check_move(correct_move);

        assert!(mode.current_puzzle.as_ref().unwrap().is_solved());

        mode.reset_current_puzzle();
        assert!(!mode.current_puzzle.as_ref().unwrap().is_solved());
    }
}
