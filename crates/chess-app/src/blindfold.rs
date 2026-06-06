//! Blindfold mode for advanced training.
//!
//! Hides pieces on the board, showing only move indicators and
//! coordinate labels. Players must rely on memory to track piece positions.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};

/// Blindfold difficulty levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlindfoldLevel {
    /// All pieces visible (normal mode).
    None,
    /// Only opponent's pieces are hidden.
    OpponentHidden,
    /// All pieces hidden, only last move shown.
    FullBlindfold,
    /// All pieces hidden, no move indicators.
    ExtremeBlindfold,
}

impl BlindfoldLevel {
    /// Get the Chinese label.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::None => "正常",
            Self::OpponentHidden => "半盲棋",
            Self::FullBlindfold => "全盲棋",
            Self::ExtremeBlindfold => "极限盲棋",
        }
    }

    /// Get the next level.
    pub fn next(&self) -> Self {
        match self {
            Self::None => Self::OpponentHidden,
            Self::OpponentHidden => Self::FullBlindfold,
            Self::FullBlindfold => Self::ExtremeBlindfold,
            Self::ExtremeBlindfold => Self::None,
        }
    }
}

impl Default for BlindfoldLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Resource managing blindfold mode.
#[derive(Resource, Debug, Clone)]
pub struct BlindfoldMode {
    /// Current blindfold level.
    pub level: BlindfoldLevel,
    /// Whether blindfold mode is active.
    pub active: bool,
}

impl Default for BlindfoldMode {
    fn default() -> Self {
        Self {
            level: BlindfoldLevel::None,
            active: false,
        }
    }
}

impl BlindfoldMode {
    /// Toggle blindfold mode.
    pub fn toggle(&mut self) {
        if self.active {
            self.level = self.level.next();
            if self.level == BlindfoldLevel::None {
                self.active = false;
            }
        } else {
            self.active = true;
            self.level = BlindfoldLevel::OpponentHidden;
        }
    }

    /// Check if a piece should be visible.
    ///
    /// `piece_is_red`: whether the piece belongs to Red.
    /// `player_is_red`: whether the local player is Red.
    pub fn is_piece_visible(&self, piece_is_red: bool, player_is_red: bool) -> bool {
        match self.level {
            BlindfoldLevel::None => true,
            BlindfoldLevel::OpponentHidden => {
                // Show own pieces, hide opponent's
                if player_is_red {
                    piece_is_red // Red player sees Red pieces
                } else {
                    !piece_is_red // Black player sees Black pieces
                }
            }
            BlindfoldLevel::FullBlindfold | BlindfoldLevel::ExtremeBlindfold => false,
        }
    }

    /// Check if move indicators should be shown.
    pub fn show_move_indicators(&self) -> bool {
        match self.level {
            BlindfoldLevel::None => false, // Not needed in normal mode
            BlindfoldLevel::OpponentHidden => true,
            BlindfoldLevel::FullBlindfold => true,
            BlindfoldLevel::ExtremeBlindfold => false,
        }
    }
}

/// Toggle blindfold mode with keyboard shortcut.
pub fn toggle_blindfold(
    keys: Res<ButtonInput<KeyCode>>,
    mut blindfold: ResMut<BlindfoldMode>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyB) {
        blindfold.toggle();
        dirty.0 = true;
        let msg = if blindfold.active {
            format!("盲棋模式: {}", blindfold.level.label_cn())
        } else {
            "盲棋模式已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blindfold_default() {
        let blindfold = BlindfoldMode::default();
        assert!(!blindfold.active);
        assert_eq!(blindfold.level, BlindfoldLevel::None);
    }

    #[test]
    fn test_toggle() {
        let mut blindfold = BlindfoldMode::default();
        blindfold.toggle();
        assert!(blindfold.active);
        assert_eq!(blindfold.level, BlindfoldLevel::OpponentHidden);

        blindfold.toggle();
        assert!(blindfold.active);
        assert_eq!(blindfold.level, BlindfoldLevel::FullBlindfold);

        blindfold.toggle();
        assert!(blindfold.active);
        assert_eq!(blindfold.level, BlindfoldLevel::ExtremeBlindfold);

        blindfold.toggle();
        assert!(!blindfold.active);
        assert_eq!(blindfold.level, BlindfoldLevel::None);
    }

    #[test]
    fn test_piece_visibility_none() {
        let blindfold = BlindfoldMode::default();
        assert!(blindfold.is_piece_visible(true, true));
        assert!(blindfold.is_piece_visible(false, true));
    }

    #[test]
    fn test_piece_visibility_opponent_hidden() {
        let mut blindfold = BlindfoldMode::default();
        blindfold.level = BlindfoldLevel::OpponentHidden;
        blindfold.active = true;

        // Red player
        assert!(blindfold.is_piece_visible(true, true)); // Own piece visible
        assert!(!blindfold.is_piece_visible(false, true)); // Opponent hidden

        // Black player
        assert!(!blindfold.is_piece_visible(true, false)); // Opponent hidden
        assert!(blindfold.is_piece_visible(false, false)); // Own piece visible
    }

    #[test]
    fn test_piece_visibility_full_blindfold() {
        let mut blindfold = BlindfoldMode::default();
        blindfold.level = BlindfoldLevel::FullBlindfold;
        blindfold.active = true;

        assert!(!blindfold.is_piece_visible(true, true));
        assert!(!blindfold.is_piece_visible(false, true));
        assert!(!blindfold.is_piece_visible(true, false));
        assert!(!blindfold.is_piece_visible(false, false));
    }

    #[test]
    fn test_move_indicators() {
        let mut blindfold = BlindfoldMode::default();
        assert!(!blindfold.show_move_indicators());

        blindfold.level = BlindfoldLevel::OpponentHidden;
        assert!(blindfold.show_move_indicators());

        blindfold.level = BlindfoldLevel::FullBlindfold;
        assert!(blindfold.show_move_indicators());

        blindfold.level = BlindfoldLevel::ExtremeBlindfold;
        assert!(!blindfold.show_move_indicators());
    }

    #[test]
    fn test_labels() {
        assert_eq!(BlindfoldLevel::None.label_cn(), "正常");
        assert_eq!(BlindfoldLevel::OpponentHidden.label_cn(), "半盲棋");
        assert_eq!(BlindfoldLevel::FullBlindfold.label_cn(), "全盲棋");
        assert_eq!(BlindfoldLevel::ExtremeBlindfold.label_cn(), "极限盲棋");
    }

    #[test]
    fn test_level_cycle() {
        let mut level = BlindfoldLevel::None;
        level = level.next();
        assert_eq!(level, BlindfoldLevel::OpponentHidden);
        level = level.next();
        assert_eq!(level, BlindfoldLevel::FullBlindfold);
        level = level.next();
        assert_eq!(level, BlindfoldLevel::ExtremeBlindfold);
        level = level.next();
        assert_eq!(level, BlindfoldLevel::None);
    }
}
