//! Per-piece animation customization.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PieceAnimationCustom {
    pub enabled: bool,
    pub move_speed: f32,
    pub capture_bounce: f32,
    pub check_shake: f32,
}

impl Default for PieceAnimationCustom {
    fn default() -> Self {
        Self {
            enabled: true,
            move_speed: 1.0,
            capture_bounce: 1.0,
            check_shake: 1.0,
        }
    }
}

impl PieceAnimationCustom {
    pub fn faster(&mut self) {
        self.move_speed = (self.move_speed * 1.5).min(5.0);
    }
    pub fn slower(&mut self) {
        self.move_speed = (self.move_speed / 1.5).max(0.2);
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn cycle_animation_speed(
    keys: Res<ButtonInput<KeyCode>>,
    mut pa: ResMut<PieceAnimationCustom>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyA) {
        pa.faster();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("动画速度: {:.1}x", pa.move_speed),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_faster() {
        let mut pa = PieceAnimationCustom::default();
        pa.faster();
        assert!(pa.move_speed > 1.0);
    }
    #[test]
    fn test_slower() {
        let mut pa = PieceAnimationCustom::default();
        pa.slower();
        assert!(pa.move_speed < 1.0);
    }
}
