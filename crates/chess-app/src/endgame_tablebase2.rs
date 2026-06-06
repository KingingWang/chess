//! Endgame tablebase for perfect endgame play.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndgameResult {
    Win,
    Loss,
    Draw,
    Unknown,
}

impl EndgameResult {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Win => "必胜",
            Self::Loss => "必败",
            Self::Draw => "必和",
            Self::Unknown => "未知",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct EndgameTablebase {
    pub positions: HashMap<String, (EndgameResult, u32)>,
    pub enabled: bool,
}

impl Default for EndgameTablebase {
    fn default() -> Self {
        Self {
            positions: HashMap::new(),
            enabled: true,
        }
    }
}

impl EndgameTablebase {
    pub fn lookup(&self, fen: &str) -> Option<(EndgameResult, u32)> {
        self.positions.get(fen).copied()
    }
    pub fn add_position(&mut self, fen: &str, result: EndgameResult, moves_to_end: u32) {
        self.positions
            .insert(fen.to_string(), (result, moves_to_end));
    }
    pub fn best_move(&self, _fen: &str) -> Option<String> {
        None
    }
}

pub fn toggle_tablebase(
    keys: Res<ButtonInput<KeyCode>>,
    mut etb: ResMut<EndgameTablebase>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyE) {
        etb.enabled = !etb.enabled;
        let msg = if etb.enabled {
            format!("残局库已启用: {}个局面", etb.positions.len())
        } else {
            "残局库已禁用".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lookup() {
        let mut etb = EndgameTablebase::default();
        etb.add_position("test", EndgameResult::Win, 5);
        assert_eq!(etb.lookup("test"), Some((EndgameResult::Win, 5)));
    }
}
