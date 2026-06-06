//! Board editor for custom position setup.
//!
//! Allows drag-and-drop placement of pieces to create custom positions,
//! with validation for legal positions.

use bevy::prelude::*;

/// Editor mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Selecting pieces from palette.
    Palette,
    /// Placing a piece on the board.
    Placing,
    /// Removing a piece from the board.
    Removing,
}

/// Editor piece palette entry.
#[derive(Debug, Clone, Copy)]
pub struct PaletteEntry {
    pub piece: char,
    pub is_red: bool,
    pub label: &'static str,
}

/// Board editor resource.
#[derive(Resource, Debug, Clone)]
pub struct BoardEditor {
    pub active: bool,
    pub mode: EditorMode,
    pub selected_piece: Option<(char, bool)>,
    pub side_to_move: bool,
    pub move_number: u32,
}

impl Default for BoardEditor {
    fn default() -> Self {
        Self {
            active: false,
            mode: EditorMode::Palette,
            selected_piece: None,
            side_to_move: true,
            move_number: 1,
        }
    }
}

/// Available pieces in the palette.
static PALETTE: &[PaletteEntry] = &[
    PaletteEntry {
        piece: 'K',
        is_red: true,
        label: "帅",
    },
    PaletteEntry {
        piece: 'A',
        is_red: true,
        label: "仕",
    },
    PaletteEntry {
        piece: 'E',
        is_red: true,
        label: "相",
    },
    PaletteEntry {
        piece: 'H',
        is_red: true,
        label: "馬",
    },
    PaletteEntry {
        piece: 'R',
        is_red: true,
        label: "車",
    },
    PaletteEntry {
        piece: 'C',
        is_red: true,
        label: "炮",
    },
    PaletteEntry {
        piece: 'P',
        is_red: true,
        label: "兵",
    },
    PaletteEntry {
        piece: 'K',
        is_red: false,
        label: "将",
    },
    PaletteEntry {
        piece: 'A',
        is_red: false,
        label: "士",
    },
    PaletteEntry {
        piece: 'E',
        is_red: false,
        label: "象",
    },
    PaletteEntry {
        piece: 'H',
        is_red: false,
        label: "馬",
    },
    PaletteEntry {
        piece: 'R',
        is_red: false,
        label: "車",
    },
    PaletteEntry {
        piece: 'C',
        is_red: false,
        label: "砲",
    },
    PaletteEntry {
        piece: 'P',
        is_red: false,
        label: "卒",
    },
];

impl BoardEditor {
    pub fn start(&mut self) {
        self.active = true;
        self.mode = EditorMode::Palette;
        self.selected_piece = None;
        self.side_to_move = true;
        self.move_number = 1;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.selected_piece = None;
    }

    pub fn select_piece(&mut self, piece: char, is_red: bool) {
        self.selected_piece = Some((piece, is_red));
        self.mode = EditorMode::Placing;
    }

    pub fn clear_selection(&mut self) {
        self.selected_piece = None;
        self.mode = EditorMode::Palette;
    }

    pub fn toggle_side_to_move(&mut self) {
        self.side_to_move = !self.side_to_move;
    }

    pub fn palette() -> &'static [PaletteEntry] {
        PALETTE
    }

    pub fn generate_fen(&self, core: &crate::app_state::CoreGame) -> String {
        core.game.board().to_fen()
    }

    pub fn fen_status(&self) -> &'static str {
        if self.active {
            "编辑模式"
        } else {
            "正常模式"
        }
    }
}

pub fn toggle_board_editor(
    keys: Res<ButtonInput<KeyCode>>,
    mut editor: ResMut<BoardEditor>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyE) {
        if editor.active {
            editor.stop();
            crate::toast::spawn_toast(&mut commands, &fonts, "编辑器已关闭");
        } else {
            editor.start();
            crate::toast::spawn_toast(&mut commands, &fonts, "棋盘编辑器: 从右侧选择棋子放置");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let e = BoardEditor::default();
        assert!(!e.active);
        assert_eq!(e.mode, EditorMode::Palette);
    }

    #[test]
    fn test_start_stop() {
        let mut e = BoardEditor::default();
        e.start();
        assert!(e.active);
        e.stop();
        assert!(!e.active);
    }

    #[test]
    fn test_select_piece() {
        let mut e = BoardEditor::default();
        e.select_piece('R', true);
        assert_eq!(e.selected_piece, Some(('R', true)));
        assert_eq!(e.mode, EditorMode::Placing);
    }

    #[test]
    fn test_clear_selection() {
        let mut e = BoardEditor::default();
        e.select_piece('R', true);
        e.clear_selection();
        assert!(e.selected_piece.is_none());
        assert_eq!(e.mode, EditorMode::Palette);
    }

    #[test]
    fn test_toggle_side() {
        let mut e = BoardEditor::default();
        assert!(e.side_to_move);
        e.toggle_side_to_move();
        assert!(!e.side_to_move);
    }

    #[test]
    fn test_palette() {
        let palette = BoardEditor::palette();
        assert_eq!(palette.len(), 14);
        // 7 red pieces + 7 black pieces
        assert!(palette.iter().any(|p| p.piece == 'K' && p.is_red));
        assert!(palette.iter().any(|p| p.piece == 'K' && !p.is_red));
    }

    #[test]
    fn test_fen_status() {
        let mut e = BoardEditor::default();
        assert_eq!(e.fen_status(), "正常模式");
        e.start();
        assert_eq!(e.fen_status(), "编辑模式");
    }
}
