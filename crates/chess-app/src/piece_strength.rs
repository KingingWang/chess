//! Piece strength indicator showing relative piece values.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PieceStrength {
    pub enabled: bool,
    pub show_on_hover: bool,
    pub base_values: std::collections::HashMap<char, i32>,
}

impl Default for PieceStrength {
    fn default() -> Self {
        let mut base_values = std::collections::HashMap::new();
        base_values.insert('K', 10000);
        base_values.insert('R', 900);
        base_values.insert('C', 450);
        base_values.insert('H', 400);
        base_values.insert('E', 200);
        base_values.insert('A', 200);
        base_values.insert('P', 100);
        Self {
            enabled: true,
            show_on_hover: true,
            base_values,
        }
    }
}

impl PieceStrength {
    pub fn value(&self, piece: char) -> i32 {
        self.base_values.get(&piece).copied().unwrap_or(0)
    }
    pub fn strength_label(&self, piece: char) -> String {
        match self.value(piece) {
            10000 => "帅/将".to_string(),
            900 => "车".to_string(),
            450 => "炮".to_string(),
            400 => "马".to_string(),
            200 => "相/象或仕/士".to_string(),
            100 => "兵/卒".to_string(),
            _ => "?".to_string(),
        }
    }
}

pub fn toggle_strength(
    keys: Res<ButtonInput<KeyCode>>,
    mut ps: ResMut<PieceStrength>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyI) {
        ps.enabled = !ps.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ps.enabled {
                "棋子强度显示已开启"
            } else {
                "棋子强度显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_values() {
        let ps = PieceStrength::default();
        assert_eq!(ps.value('R'), 900);
        assert_eq!(ps.value('P'), 100);
    }
    #[test]
    fn test_labels() {
        let ps = PieceStrength::default();
        assert_eq!(ps.strength_label('R'), "车");
    }
}
