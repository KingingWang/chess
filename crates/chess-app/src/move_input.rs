//! Move input via keyboard using algebraic notation.
//!
//! Allows users to type moves like "e2e4" or "h2e2" to make moves.

use bevy::prelude::*;
use chess_core::{Board, Move, Square};

/// Resource managing move input state.
#[derive(Resource)]
pub struct MoveInput {
    /// Whether move input mode is active.
    pub active: bool,
    /// Current input buffer.
    pub input_buffer: String,
    /// Error message if the last input was invalid.
    pub error_message: Option<String>,
    /// Last successfully parsed move.
    pub last_move: Option<Move>,
}

impl Default for MoveInput {
    fn default() -> Self {
        Self {
            active: false,
            input_buffer: String::new(),
            error_message: None,
            last_move: None,
        }
    }
}

impl MoveInput {
    /// Activate move input mode.
    pub fn activate(&mut self) {
        self.active = true;
        self.input_buffer.clear();
        self.error_message = None;
    }

    /// Deactivate move input mode.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.input_buffer.clear();
        self.error_message = None;
    }

    /// Toggle move input mode.
    pub fn toggle(&mut self) {
        if self.active {
            self.deactivate();
        } else {
            self.activate();
        }
    }

    /// Add a character to the input buffer.
    pub fn add_char(&mut self, ch: char) {
        if self.active && ch.is_ascii() && !ch.is_control() {
            self.input_buffer.push(ch);
        }
    }

    /// Remove the last character from the input buffer.
    pub fn backspace(&mut self) {
        if self.active {
            self.input_buffer.pop();
        }
    }

    /// Clear the input buffer.
    pub fn clear(&mut self) {
        self.input_buffer.clear();
        self.error_message = None;
    }

    /// Parse the input buffer as a move.
    pub fn parse_move(&mut self, board: &Board) -> Option<Move> {
        let input = self.input_buffer.trim().to_lowercase();

        // Try to parse as algebraic notation (e.g., "e2e4")
        if let Some(mv) = self.parse_algebraic(&input, board) {
            self.last_move = Some(mv);
            self.error_message = None;
            self.input_buffer.clear();
            return Some(mv);
        }

        // Try to parse as ICCS notation (e.g., "h2-e2")
        if let Some(mv) = self.parse_iccs(&input, board) {
            self.last_move = Some(mv);
            self.error_message = None;
            self.input_buffer.clear();
            return Some(mv);
        }

        // Invalid input
        self.error_message = Some(format!("Invalid move notation: {}", input));
        None
    }

    /// Parse algebraic notation (e.g., "e2e4").
    fn parse_algebraic(&self, input: &str, board: &Board) -> Option<Move> {
        if input.len() != 4 {
            return None;
        }

        let chars: Vec<char> = input.chars().collect();

        // Parse from square
        let from_file = Self::file_from_char(chars[0])?;
        let from_rank = Self::rank_from_char(chars[1])?;
        let from = Square::new(from_file, from_rank)?;

        // Parse to square
        let to_file = Self::file_from_char(chars[2])?;
        let to_rank = Self::rank_from_char(chars[3])?;
        let to = Square::new(to_file, to_rank)?;

        let mv = Move { from, to };

        // Validate the move
        if board.is_legal(mv) {
            Some(mv)
        } else {
            None
        }
    }

    /// Parse ICCS notation (e.g., "h2-e2" or "h2e2").
    fn parse_iccs(&self, input: &str, board: &Board) -> Option<Move> {
        let clean_input = input.replace('-', "");
        self.parse_algebraic(&clean_input, board)
    }

    /// Convert a file character (a-i) to a file number (0-8).
    fn file_from_char(ch: char) -> Option<u8> {
        match ch {
            'a' => Some(0),
            'b' => Some(1),
            'c' => Some(2),
            'd' => Some(3),
            'e' => Some(4),
            'f' => Some(5),
            'g' => Some(6),
            'h' => Some(7),
            'i' => Some(8),
            _ => None,
        }
    }

    /// Convert a rank character (0-9) to a rank number (0-9).
    fn rank_from_char(ch: char) -> Option<u8> {
        match ch {
            '0' => Some(0),
            '1' => Some(1),
            '2' => Some(2),
            '3' => Some(3),
            '4' => Some(4),
            '5' => Some(5),
            '6' => Some(6),
            '7' => Some(7),
            '8' => Some(8),
            '9' => Some(9),
            _ => None,
        }
    }
}

/// Component for move input UI elements.
#[derive(Component)]
pub struct MoveInputUI;

/// System to toggle move input mode.
pub fn toggle_move_input(mut move_input: ResMut<MoveInput>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        move_input.toggle();
    }
}

/// System to handle move input from keyboard.
pub fn handle_move_input(
    mut move_input: ResMut<MoveInput>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !move_input.active {
        return;
    }

    // Handle Enter key to submit the move
    if keyboard.just_pressed(KeyCode::Enter) {
        let board = core.game.board().clone();
        if let Some(mv) = move_input.parse_move(&board) {
            if core.game.make_move(mv).is_ok() {
                dirty.0 = true;
            }
        }
    }

    // Handle Escape key to cancel
    if keyboard.just_pressed(KeyCode::Escape) {
        move_input.deactivate();
    }

    // Handle Backspace
    if keyboard.just_pressed(KeyCode::Backspace) {
        move_input.backspace();
    }
}

/// System to update move input from keyboard (placeholder for future implementation).
pub fn update_move_input_chars(
    mut move_input: ResMut<MoveInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !move_input.active {
        return;
    }

    // Handle alphanumeric keys
    for key in [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ] {
        if keyboard.just_pressed(key) {
            let ch = match key {
                KeyCode::KeyA => 'a',
                KeyCode::KeyB => 'b',
                KeyCode::KeyC => 'c',
                KeyCode::KeyD => 'd',
                KeyCode::KeyE => 'e',
                KeyCode::KeyF => 'f',
                KeyCode::KeyG => 'g',
                KeyCode::KeyH => 'h',
                KeyCode::KeyI => 'i',
                KeyCode::KeyJ => 'j',
                KeyCode::KeyK => 'k',
                KeyCode::KeyL => 'l',
                KeyCode::KeyM => 'm',
                KeyCode::KeyN => 'n',
                KeyCode::KeyO => 'o',
                KeyCode::KeyP => 'p',
                KeyCode::KeyQ => 'q',
                KeyCode::KeyR => 'r',
                KeyCode::KeyS => 's',
                KeyCode::KeyT => 't',
                KeyCode::KeyU => 'u',
                KeyCode::KeyV => 'v',
                KeyCode::KeyW => 'w',
                KeyCode::KeyX => 'x',
                KeyCode::KeyY => 'y',
                KeyCode::KeyZ => 'z',
                KeyCode::Digit0 => '0',
                KeyCode::Digit1 => '1',
                KeyCode::Digit2 => '2',
                KeyCode::Digit3 => '3',
                KeyCode::Digit4 => '4',
                KeyCode::Digit5 => '5',
                KeyCode::Digit6 => '6',
                KeyCode::Digit7 => '7',
                KeyCode::Digit8 => '8',
                KeyCode::Digit9 => '9',
                _ => '\0',
            };
            if ch != '\0' {
                move_input.add_char(ch);
            }
        }
    }

    // Handle dash key for ICCS notation
    if keyboard.just_pressed(KeyCode::Minus) {
        move_input.add_char('-');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::START_FEN;

    #[test]
    fn test_move_input_default() {
        let input = MoveInput::default();
        assert!(!input.active);
        assert!(input.input_buffer.is_empty());
        assert!(input.error_message.is_none());
        assert!(input.last_move.is_none());
    }

    #[test]
    fn test_activate_deactivate() {
        let mut input = MoveInput::default();

        input.activate();
        assert!(input.active);

        input.deactivate();
        assert!(!input.active);
    }

    #[test]
    fn test_toggle() {
        let mut input = MoveInput::default();

        input.toggle();
        assert!(input.active);

        input.toggle();
        assert!(!input.active);
    }

    #[test]
    fn test_add_char() {
        let mut input = MoveInput::default();
        input.activate();

        input.add_char('e');
        assert_eq!(input.input_buffer, "e");

        input.add_char('2');
        assert_eq!(input.input_buffer, "e2");
    }

    #[test]
    fn test_add_char_inactive() {
        let mut input = MoveInput::default();
        // Not activated

        input.add_char('e');
        assert!(input.input_buffer.is_empty());
    }

    #[test]
    fn test_backspace() {
        let mut input = MoveInput::default();
        input.activate();

        input.add_char('e');
        input.add_char('2');
        input.backspace();

        assert_eq!(input.input_buffer, "e");
    }

    #[test]
    fn test_clear() {
        let mut input = MoveInput::default();
        input.activate();

        input.add_char('e');
        input.add_char('2');
        input.clear();

        assert!(input.input_buffer.is_empty());
        assert!(input.error_message.is_none());
    }

    #[test]
    fn test_file_from_char() {
        assert_eq!(MoveInput::file_from_char('a'), Some(0));
        assert_eq!(MoveInput::file_from_char('e'), Some(4));
        assert_eq!(MoveInput::file_from_char('i'), Some(8));
        assert_eq!(MoveInput::file_from_char('z'), None);
    }

    #[test]
    fn test_rank_from_char() {
        assert_eq!(MoveInput::rank_from_char('0'), Some(0));
        assert_eq!(MoveInput::rank_from_char('5'), Some(5));
        assert_eq!(MoveInput::rank_from_char('9'), Some(9));
        assert_eq!(MoveInput::rank_from_char('a'), None);
    }

    #[test]
    fn test_parse_valid_move() {
        let mut input = MoveInput::default();
        input.activate();

        let board = Board::from_fen(START_FEN).unwrap();

        // Add "h2e2" (cannon move)
        for ch in "h2e2".chars() {
            input.add_char(ch);
        }

        let mv = input.parse_move(&board);
        assert!(mv.is_some());
        assert!(input.input_buffer.is_empty());
        assert!(input.error_message.is_none());
    }

    #[test]
    fn test_parse_invalid_move() {
        let mut input = MoveInput::default();
        input.activate();

        let board = Board::from_fen(START_FEN).unwrap();

        // Add "e2e4" (invalid in Xiangqi starting position)
        for ch in "e2e4".chars() {
            input.add_char(ch);
        }

        let mv = input.parse_move(&board);
        assert!(mv.is_none());
        assert!(input.error_message.is_some());
    }

    #[test]
    fn test_parse_iccs_notation() {
        let mut input = MoveInput::default();
        input.activate();

        let board = Board::from_fen(START_FEN).unwrap();

        // Add "h2-e2" (ICCS notation with dash)
        for ch in "h2-e2".chars() {
            input.add_char(ch);
        }

        let mv = input.parse_move(&board);
        assert!(mv.is_some());
    }

    #[test]
    fn test_parse_case_insensitive() {
        let mut input = MoveInput::default();
        input.activate();

        let board = Board::from_fen(START_FEN).unwrap();

        // Add "H2E2" (uppercase)
        for ch in "H2E2".chars() {
            input.add_char(ch);
        }

        let mv = input.parse_move(&board);
        assert!(mv.is_some());
    }
}
