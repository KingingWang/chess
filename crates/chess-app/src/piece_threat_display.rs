//! Piece threat visualization showing attack/defense relationships.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PieceThreatDisplay {
    pub enabled: bool,
    pub show_attacks: bool,
    pub show_defenses: bool,
}

impl Default for PieceThreatDisplay {
    fn default() -> Self {
        Self {
            enabled: false,
            show_attacks: true,
            show_defenses: true,
        }
    }
}

pub fn toggle_threat_display(
    keys: Res<ButtonInput<KeyCode>>,
    mut ptd: ResMut<PieceThreatDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyD) {
        ptd.enabled = !ptd.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ptd.enabled {
                "威胁显示已开启"
            } else {
                "威胁显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let ptd = PieceThreatDisplay::default();
        assert!(!ptd.enabled);
        assert!(ptd.show_attacks);
        assert!(ptd.show_defenses);
    }
}
