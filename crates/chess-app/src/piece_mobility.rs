//! Piece mobility tracking and display.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone)]
pub struct PieceMobility {
    pub enabled: bool,
    pub mobility: HashMap<(u8, u8), usize>,
}

impl Default for PieceMobility {
    fn default() -> Self {
        Self {
            enabled: false,
            mobility: HashMap::new(),
        }
    }
}

impl PieceMobility {
    pub fn update(&mut self, file: u8, rank: u8, legal_moves: usize) {
        self.mobility.insert((file, rank), legal_moves);
    }
    pub fn get(&self, file: u8, rank: u8) -> usize {
        self.mobility.get(&(file, rank)).copied().unwrap_or(0)
    }
    pub fn total(&self) -> usize {
        self.mobility.values().sum()
    }
    pub fn clear(&mut self) {
        self.mobility.clear();
    }
}

pub fn toggle_mobility(
    keys: Res<ButtonInput<KeyCode>>,
    mut pm: ResMut<PieceMobility>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyM) {
        pm.enabled = !pm.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if pm.enabled {
                "棋子机动性显示已开启"
            } else {
                "棋子机动性显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_update() {
        let mut pm = PieceMobility::default();
        pm.update(0, 0, 5);
        assert_eq!(pm.get(0, 0), 5);
    }
    #[test]
    fn test_total() {
        let mut pm = PieceMobility::default();
        pm.update(0, 0, 5);
        pm.update(1, 1, 3);
        assert_eq!(pm.total(), 8);
    }
}
