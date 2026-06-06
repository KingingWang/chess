//! Position setup from FEN strings and PGN files.
//!
//! Allows users to load custom positions for analysis or play.

use bevy::prelude::*;
use chess_core::{Board, Game};
use std::path::PathBuf;

/// Resource managing position setup state.
#[derive(Resource)]
pub struct PositionSetup {
    /// Whether the setup dialog is visible.
    pub visible: bool,
    /// Current FEN string in the input field.
    pub fen_input: String,
    /// Error message if FEN is invalid.
    pub error_message: Option<String>,
    /// Last loaded position (if any).
    pub last_position: Option<Board>,
    /// Path to the last loaded PGN file.
    pub last_pgn_path: Option<PathBuf>,
}

impl Default for PositionSetup {
    fn default() -> Self {
        Self {
            visible: false,
            fen_input: String::new(),
            error_message: None,
            last_position: None,
            last_pgn_path: None,
        }
    }
}

impl PositionSetup {
    /// Show the setup dialog.
    pub fn show(&mut self) {
        self.visible = true;
        self.error_message = None;
    }

    /// Hide the setup dialog.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle the setup dialog visibility.
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Validate and load a FEN string.
    pub fn load_fen(&mut self, fen: &str) -> Result<Board, String> {
        match Board::from_fen(fen) {
            Ok(board) => {
                self.fen_input = fen.to_string();
                self.last_position = Some(board.clone());
                self.error_message = None;
                Ok(board)
            }
            Err(e) => {
                let error = format!("Invalid FEN: {}", e);
                self.error_message = Some(error.clone());
                Err(error)
            }
        }
    }

    /// Load the starting position.
    pub fn load_starting_position(&mut self) -> Board {
        let board = Board::start_position();
        self.fen_input = chess_core::START_FEN.to_string();
        self.last_position = Some(board.clone());
        self.error_message = None;
        board
    }

    /// Clear the current position.
    pub fn clear_position(&mut self) {
        self.fen_input.clear();
        self.last_position = None;
        self.error_message = None;
    }

    /// Load a position from a PGN file (placeholder for future implementation).
    pub fn load_from_pgn(&mut self, _path: PathBuf) -> Result<Game, String> {
        Err("PGN loading not yet implemented".to_string())
    }

    /// Get the current board position (if loaded).
    pub fn current_position(&self) -> Option<&Board> {
        self.last_position.as_ref()
    }
}

/// Component for position setup UI elements.
#[derive(Component)]
pub struct PositionSetupUI;

/// System to toggle position setup dialog.
pub fn toggle_position_setup(
    mut setup: ResMut<PositionSetup>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyS) {
        setup.toggle();
    }
}

/// System to handle FEN input.
pub fn handle_fen_input(
    mut setup: ResMut<PositionSetup>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !setup.visible {
        return;
    }

    // Load starting position with F1
    if keyboard.just_pressed(KeyCode::F1) {
        let board = setup.load_starting_position();
        core.game = Game::from_board(board);
        dirty.0 = true;
    }

    // Clear position with F2
    if keyboard.just_pressed(KeyCode::F2) {
        setup.clear_position();
    }

    // Load FEN with F3 (if valid)
    if keyboard.just_pressed(KeyCode::F3) {
        let fen = setup.fen_input.clone();
        if !fen.is_empty() {
            match setup.load_fen(&fen) {
                Ok(board) => {
                    core.game = Game::from_board(board);
                    dirty.0 = true;
                }
                Err(_) => {
                    // Error already set in load_fen
                }
            }
        }
    }
}

/// System to update FEN input from keyboard (placeholder for future implementation).
pub fn update_fen_input(mut setup: ResMut<PositionSetup>, keyboard: Res<ButtonInput<KeyCode>>) {
    if !setup.visible {
        return;
    }

    // Handle backspace
    if keyboard.just_pressed(KeyCode::Backspace) {
        setup.fen_input.pop();
    }

    // TODO: Add character input handling using Bevy's IME or text input system
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::START_FEN;

    #[test]
    fn test_position_setup_default() {
        let setup = PositionSetup::default();
        assert!(!setup.visible);
        assert!(setup.fen_input.is_empty());
        assert!(setup.error_message.is_none());
        assert!(setup.last_position.is_none());
    }

    #[test]
    fn test_show_hide() {
        let mut setup = PositionSetup::default();

        setup.show();
        assert!(setup.visible);
        assert!(setup.error_message.is_none());

        setup.hide();
        assert!(!setup.visible);
    }

    #[test]
    fn test_toggle() {
        let mut setup = PositionSetup::default();

        setup.toggle();
        assert!(setup.visible);

        setup.toggle();
        assert!(!setup.visible);
    }

    #[test]
    fn test_load_valid_fen() {
        let mut setup = PositionSetup::default();
        let fen = START_FEN;

        let result = setup.load_fen(fen);
        assert!(result.is_ok());
        assert_eq!(setup.fen_input, fen);
        assert!(setup.last_position.is_some());
        assert!(setup.error_message.is_none());
    }

    #[test]
    fn test_load_invalid_fen() {
        let mut setup = PositionSetup::default();
        let invalid_fen = "invalid fen string";

        let result = setup.load_fen(invalid_fen);
        assert!(result.is_err());
        assert!(setup.error_message.is_some());
        assert!(setup
            .error_message
            .as_ref()
            .unwrap()
            .contains("Invalid FEN"));
    }

    #[test]
    fn test_load_starting_position() {
        let mut setup = PositionSetup::default();

        let board = setup.load_starting_position();
        assert_eq!(board.to_fen(), START_FEN);
        assert_eq!(setup.fen_input, START_FEN);
        assert!(setup.last_position.is_some());
    }

    #[test]
    fn test_clear_position() {
        let mut setup = PositionSetup::default();
        setup.load_starting_position();

        setup.clear_position();
        assert!(setup.fen_input.is_empty());
        assert!(setup.last_position.is_none());
        assert!(setup.error_message.is_none());
    }

    #[test]
    fn test_current_position() {
        let mut setup = PositionSetup::default();

        assert!(setup.current_position().is_none());

        setup.load_starting_position();
        assert!(setup.current_position().is_some());
    }

    #[test]
    fn test_load_fen_updates_last_position() {
        let mut setup = PositionSetup::default();
        let fen1 = START_FEN;
        let fen2 = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR b - - 0 1";

        setup.load_fen(fen1).unwrap();
        let pos1 = setup.current_position().unwrap().clone();

        setup.load_fen(fen2).unwrap();
        let pos2 = setup.current_position().unwrap().clone();

        assert_ne!(pos1.to_fen(), pos2.to_fen());
    }

    #[test]
    fn test_error_message_cleared_on_success() {
        let mut setup = PositionSetup::default();

        // Load invalid FEN to set error
        setup.load_fen("invalid").unwrap_err();
        assert!(setup.error_message.is_some());

        // Load valid FEN to clear error
        setup.load_fen(START_FEN).unwrap();
        assert!(setup.error_message.is_none());
    }

    #[test]
    fn test_show_clears_error() {
        let mut setup = PositionSetup::default();

        // Set an error
        setup.load_fen("invalid").unwrap_err();
        assert!(setup.error_message.is_some());

        // Show should clear the error
        setup.show();
        assert!(setup.error_message.is_none());
    }
}
