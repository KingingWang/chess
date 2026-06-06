//! Game timer presets for common time controls.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerPreset {
    Bullet1,
    Blitz3,
    Blitz5,
    Rapid10,
    Rapid15,
    Classical30,
    Unlimited,
}

impl TimerPreset {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Bullet1 => "1分钟",
            Self::Blitz3 => "3分钟",
            Self::Blitz5 => "5分钟",
            Self::Rapid10 => "10分钟",
            Self::Rapid15 => "15分钟",
            Self::Classical30 => "30分钟",
            Self::Unlimited => "无限时",
        }
    }
    pub fn seconds(&self) -> u32 {
        match self {
            Self::Bullet1 => 60,
            Self::Blitz3 => 180,
            Self::Blitz5 => 300,
            Self::Rapid10 => 600,
            Self::Rapid15 => 900,
            Self::Classical30 => 1800,
            Self::Unlimited => 0,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GameTimerPresets {
    pub selected: TimerPreset,
    pub increment: u32,
}

impl Default for GameTimerPresets {
    fn default() -> Self {
        Self {
            selected: TimerPreset::Blitz5,
            increment: 0,
        }
    }
}

pub fn cycle_timer_preset(
    keys: Res<ButtonInput<KeyCode>>,
    mut gtp: ResMut<GameTimerPresets>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyN) {
        gtp.selected = match gtp.selected {
            TimerPreset::Bullet1 => TimerPreset::Blitz3,
            TimerPreset::Blitz3 => TimerPreset::Blitz5,
            TimerPreset::Blitz5 => TimerPreset::Rapid10,
            TimerPreset::Rapid10 => TimerPreset::Rapid15,
            TimerPreset::Rapid15 => TimerPreset::Classical30,
            TimerPreset::Classical30 => TimerPreset::Unlimited,
            TimerPreset::Unlimited => TimerPreset::Bullet1,
        };
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("时间控制: {}", gtp.selected.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_seconds() {
        assert_eq!(TimerPreset::Bullet1.seconds(), 60);
        assert_eq!(TimerPreset::Unlimited.seconds(), 0);
    }
}
