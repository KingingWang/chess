//! Move hint system for beginners.
//!
//! Shows visual hints for possible moves, including:
//! - Highlighting pieces that can move
//! - Showing legal destinations for the selected piece
//! - Suggesting the best move via engine analysis

use bevy::prelude::*;

use crate::ai_bridge::SearchInfoResource;
use crate::app_state::{CoreGame, UiFonts};

/// Hint level settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HintLevel {
    /// No hints shown.
    #[default]
    Off,
    /// Only highlight movable pieces.
    MovablePieces,
    /// Show legal destinations when hovering.
    LegalMoves,
    /// Show best move suggestion from engine.
    BestMove,
    /// Show all hints (movable + legal + best).
    All,
}

impl HintLevel {
    /// Get the Chinese label.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Off => "关闭",
            Self::MovablePieces => "可动棋子",
            Self::LegalMoves => "合法着法",
            Self::BestMove => "最佳着法",
            Self::All => "全部提示",
        }
    }

    /// Get the next level.
    pub fn next(&self) -> Self {
        match self {
            Self::Off => Self::MovablePieces,
            Self::MovablePieces => Self::LegalMoves,
            Self::LegalMoves => Self::BestMove,
            Self::BestMove => Self::All,
            Self::All => Self::Off,
        }
    }
}

/// Resource managing move hints.
#[derive(Resource, Debug)]
pub struct MoveHint {
    /// Current hint level.
    pub level: HintLevel,
    /// Best move from engine analysis.
    pub best_move: Option<chess_core::Move>,
    /// List of pieces that can make legal moves.
    pub movable_pieces: Vec<chess_core::Square>,
    /// Whether to show hints automatically after a delay.
    pub auto_hint: bool,
    /// Delay before showing auto-hints (seconds).
    pub auto_hint_delay: f32,
    /// Time since last move.
    pub time_since_move: f32,
}

impl Default for MoveHint {
    fn default() -> Self {
        Self {
            level: HintLevel::Off,
            best_move: None,
            movable_pieces: Vec::new(),
            auto_hint: false,
            auto_hint_delay: 10.0,
            time_since_move: 0.0,
        }
    }
}

impl MoveHint {
    /// Update movable pieces list from the current board.
    pub fn update_movable_pieces(&mut self, core: &CoreGame) {
        self.movable_pieces.clear();

        let board = core.game.board();
        let legal_moves = board.legal_moves();

        // Collect unique source squares
        let mut seen = std::collections::HashSet::new();
        for m in &legal_moves {
            if seen.insert((m.from.file(), m.from.rank())) {
                self.movable_pieces.push(m.from);
            }
        }
    }

    /// Check if a square has a movable piece.
    pub fn is_movable(&self, square: chess_core::Square) -> bool {
        self.movable_pieces.iter().any(|s| s == &square)
    }

    /// Get the number of legal moves available.
    pub fn legal_move_count(&self, core: &CoreGame) -> usize {
        core.game.board().legal_moves().len()
    }
}

/// System to update move hints from engine analysis.
pub fn update_move_hints(
    mut hint: ResMut<MoveHint>,
    core: Res<CoreGame>,
    search_info: Res<SearchInfoResource>,
) {
    if hint.level == HintLevel::Off {
        return;
    }

    // Update best move from search info
    if hint.level == HintLevel::BestMove || hint.level == HintLevel::All {
        if let Some(info) = &search_info.latest {
            if let Some(best) = info.pv.first() {
                hint.best_move = Some(*best);
            }
        }
    }

    // Update movable pieces
    if hint.level == HintLevel::MovablePieces || hint.level == HintLevel::All {
        hint.update_movable_pieces(&core);
    }
}

/// System to track time for auto-hints.
pub fn track_hint_time(time: Res<Time>, mut hint: ResMut<MoveHint>, _core: Res<CoreGame>) {
    if !hint.auto_hint || hint.level == HintLevel::Off {
        return;
    }

    hint.time_since_move += time.delta_secs();

    // If enough time has passed, upgrade hint level temporarily
    if hint.time_since_move >= hint.auto_hint_delay && hint.level == HintLevel::MovablePieces {
        hint.level = HintLevel::LegalMoves;
    }
}

/// Reset hint timer when a move is made.
pub fn reset_hint_timer(core: Res<CoreGame>, mut hint: ResMut<MoveHint>) {
    // Simple check: if history length changed, reset timer
    static mut LAST_LEN: usize = 0;
    let current_len = core.game.history_len();
    unsafe {
        if current_len != LAST_LEN {
            LAST_LEN = current_len;
            hint.time_since_move = 0.0;
        }
    }
}

/// Toggle hint level with keyboard shortcut.
pub fn toggle_hints(
    keys: Res<ButtonInput<KeyCode>>,
    mut hint: ResMut<MoveHint>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyH) {
        hint.level = hint.level.next();
        let msg = format!("提示: {}", hint.level.label_cn());
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_default() {
        let hint = MoveHint::default();
        assert_eq!(hint.level, HintLevel::Off);
        assert!(hint.best_move.is_none());
        assert!(hint.movable_pieces.is_empty());
    }

    #[test]
    fn test_hint_level_cycle() {
        let mut level = HintLevel::Off;
        level = level.next();
        assert_eq!(level, HintLevel::MovablePieces);
        level = level.next();
        assert_eq!(level, HintLevel::LegalMoves);
        level = level.next();
        assert_eq!(level, HintLevel::BestMove);
        level = level.next();
        assert_eq!(level, HintLevel::All);
        level = level.next();
        assert_eq!(level, HintLevel::Off);
    }

    #[test]
    fn test_hint_labels() {
        assert_eq!(HintLevel::Off.label_cn(), "关闭");
        assert_eq!(HintLevel::BestMove.label_cn(), "最佳着法");
        assert_eq!(HintLevel::All.label_cn(), "全部提示");
    }

    #[test]
    fn test_is_movable() {
        let mut hint = MoveHint::default();
        let sq = chess_core::Square::new(7, 2).unwrap();
        hint.movable_pieces.push(sq);
        assert!(hint.is_movable(sq));

        let other = chess_core::Square::new(0, 0).unwrap();
        assert!(!hint.is_movable(other));
    }

    #[test]
    fn test_auto_hint_settings() {
        let hint = MoveHint::default();
        assert!(!hint.auto_hint);
        assert_eq!(hint.auto_hint_delay, 10.0);
    }
}
