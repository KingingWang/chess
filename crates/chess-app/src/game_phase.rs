//! Game phase indicator based on material and move count.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamePhase {
    #[default]
    Opening,
    Middlegame,
    Endgame,
    Unknown,
}

impl GamePhase {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Opening => "开局",
            Self::Middlegame => "中局",
            Self::Endgame => "残局",
            Self::Unknown => "未知",
        }
    }
    pub fn from_move_count(moves: usize) -> Self {
        match moves {
            0..=10 => Self::Opening,
            11..=40 => Self::Middlegame,
            41.. => Self::Endgame,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GamePhaseIndicator {
    pub enabled: bool,
    pub current: GamePhase,
    pub transition_moves: Vec<usize>,
}

impl Default for GamePhaseIndicator {
    fn default() -> Self {
        Self {
            enabled: true,
            current: GamePhase::Opening,
            transition_moves: Vec::new(),
        }
    }
}

impl GamePhaseIndicator {
    pub fn update(&mut self, move_count: usize) {
        let new_phase = GamePhase::from_move_count(move_count);
        if new_phase != self.current {
            self.transition_moves.push(move_count);
            self.current = new_phase;
        }
    }
}

pub fn toggle_phase(
    keys: Res<ButtonInput<KeyCode>>,
    mut gp: ResMut<GamePhaseIndicator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyQ) {
        gp.enabled = !gp.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gp.enabled {
                "阶段指示器已开启"
            } else {
                "阶段指示器已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_phases() {
        assert_eq!(GamePhase::from_move_count(5), GamePhase::Opening);
        assert_eq!(GamePhase::from_move_count(25), GamePhase::Middlegame);
        assert_eq!(GamePhase::from_move_count(50), GamePhase::Endgame);
    }
    #[test]
    fn test_transition() {
        let mut gp = GamePhaseIndicator::default();
        gp.update(15);
        assert_eq!(gp.current, GamePhase::Middlegame);
        assert_eq!(gp.transition_moves.len(), 1);
    }
}
