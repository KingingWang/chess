//! Material advantage display.

use bevy::prelude::*;

const PIECE_VALUES: &[(char, i32)] = &[
    ('K', 10000),
    ('R', 900),
    ('C', 450),
    ('H', 400),
    ('E', 200),
    ('A', 200),
    ('P', 100),
];

#[derive(Resource, Debug, Clone)]
pub struct MaterialAdvantage {
    pub enabled: bool,
    pub red_material: i32,
    pub black_material: i32,
}

impl Default for MaterialAdvantage {
    fn default() -> Self {
        Self {
            enabled: true,
            red_material: 0,
            black_material: 0,
        }
    }
}

impl MaterialAdvantage {
    pub fn advantage(&self) -> i32 {
        self.red_material - self.black_material
    }
    pub fn advantage_label(&self) -> String {
        let diff = self.advantage();
        if diff > 0 {
            format!("红方优势 +{}", diff)
        } else if diff < 0 {
            format!("黑方优势 +{}", -diff)
        } else {
            "均势".to_string()
        }
    }
    pub fn piece_value(piece: char) -> i32 {
        PIECE_VALUES
            .iter()
            .find(|(p, _)| *p == piece)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }
}

pub fn toggle_material(
    keys: Res<ButtonInput<KeyCode>>,
    mut ma: ResMut<MaterialAdvantage>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyZ) {
        ma.enabled = !ma.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ma.enabled {
                "子力显示已开启"
            } else {
                "子力显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_values() {
        assert_eq!(MaterialAdvantage::piece_value('R'), 900);
        assert_eq!(MaterialAdvantage::piece_value('P'), 100);
    }
    #[test]
    fn test_advantage() {
        let mut ma = MaterialAdvantage::default();
        ma.red_material = 1000;
        ma.black_material = 900;
        assert_eq!(ma.advantage(), 100);
        assert!(ma.advantage_label().contains("红方"));
    }
    #[test]
    fn test_equal() {
        let ma = MaterialAdvantage::default();
        assert_eq!(ma.advantage(), 0);
        assert_eq!(ma.advantage_label(), "均势");
    }
}
