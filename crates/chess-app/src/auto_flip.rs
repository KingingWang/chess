//! Auto-flip board for online games based on player color.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct AutoFlip {
    pub enabled: bool,
    pub flip_on_connect: bool,
    pub flip_on_turn: bool,
}

impl Default for AutoFlip {
    fn default() -> Self {
        Self {
            enabled: true,
            flip_on_connect: true,
            flip_on_turn: false,
        }
    }
}

pub fn toggle_auto_flip(
    keys: Res<ButtonInput<KeyCode>>,
    mut af: ResMut<AutoFlip>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyJ) {
        af.enabled = !af.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if af.enabled {
                "自动翻转已开启"
            } else {
                "自动翻转已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let af = AutoFlip::default();
        assert!(af.enabled);
        assert!(af.flip_on_connect);
        assert!(!af.flip_on_turn);
    }
}
