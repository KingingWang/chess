//! Game speed presets for controlling overall game pace.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GameSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
    VeryFast,
}

impl GameSpeed {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Slow => "慢速",
            Self::Normal => "正常",
            Self::Fast => "快速",
            Self::VeryFast => "极速",
        }
    }
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Slow => 0.5,
            Self::Normal => 1.0,
            Self::Fast => 1.5,
            Self::VeryFast => 2.0,
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Slow => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::VeryFast,
            Self::VeryFast => Self::Slow,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GameSpeedResource {
    pub speed: GameSpeed,
}

impl Default for GameSpeedResource {
    fn default() -> Self {
        Self {
            speed: GameSpeed::Normal,
        }
    }
}

pub fn cycle_game_speed(
    keys: Res<ButtonInput<KeyCode>>,
    mut gs: ResMut<GameSpeedResource>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyD) {
        gs.speed = gs.speed.next();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("游戏速度: {}", gs.speed.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_multipliers() {
        assert_eq!(GameSpeed::Normal.multiplier(), 1.0);
        assert!(GameSpeed::Fast.multiplier() > 1.0);
    }
    #[test]
    fn test_cycle() {
        let mut s = GameSpeed::Normal;
        s = s.next();
        assert_eq!(s, GameSpeed::Fast);
    }
}
