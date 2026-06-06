//! Piece value display for educational purposes.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone)]
pub struct PieceValueDisplay {
    pub enabled: bool,
    pub values: HashMap<char, i32>,
    pub show_on_hover: bool,
}

impl Default for PieceValueDisplay {
    fn default() -> Self {
        let mut values = HashMap::new();
        values.insert('K', 10000);
        values.insert('R', 900);
        values.insert('C', 450);
        values.insert('H', 400);
        values.insert('E', 200);
        values.insert('A', 200);
        values.insert('P', 100);
        Self {
            enabled: false,
            values,
            show_on_hover: true,
        }
    }
}

impl PieceValueDisplay {
    pub fn value(&self, piece: char) -> i32 {
        self.values.get(&piece).copied().unwrap_or(0)
    }
    pub fn material_diff(&self, red_pieces: &[char], black_pieces: &[char]) -> i32 {
        let red: i32 = red_pieces.iter().map(|&p| self.value(p)).sum();
        let black: i32 = black_pieces.iter().map(|&p| self.value(p)).sum();
        red - black
    }
}

pub fn toggle_piece_values(
    keys: Res<ButtonInput<KeyCode>>,
    mut pvd: ResMut<PieceValueDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyW) {
        pvd.enabled = !pvd.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if pvd.enabled {
                "棋子价值显示已开启"
            } else {
                "棋子价值显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_values() {
        let pvd = PieceValueDisplay::default();
        assert_eq!(pvd.value('R'), 900);
        assert_eq!(pvd.value('P'), 100);
    }
    #[test]
    fn test_material_diff() {
        let pvd = PieceValueDisplay::default();
        assert_eq!(pvd.material_diff(&['R'], &['R', 'P']), -100);
    }
}
