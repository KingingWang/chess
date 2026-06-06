//! Time bonus display for incremental time controls.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct TimeBonusDisplay {
    pub enabled: bool,
    pub bonus_per_move: f32,
    pub show_popup: bool,
}

impl Default for TimeBonusDisplay {
    fn default() -> Self {
        Self {
            enabled: true,
            bonus_per_move: 0.0,
            show_popup: true,
        }
    }
}

pub fn toggle_bonus_display(
    keys: Res<ButtonInput<KeyCode>>,
    mut tb: ResMut<TimeBonusDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyB) {
        tb.enabled = !tb.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tb.enabled {
                "时间奖励显示已开启"
            } else {
                "时间奖励显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let tb = TimeBonusDisplay::default();
        assert!(tb.enabled);
        assert_eq!(tb.bonus_per_move, 0.0);
    }
}
