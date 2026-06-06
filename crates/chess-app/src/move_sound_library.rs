//! Move sound effects library with various sounds.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    MoveNormal,
    MoveCapture,
    MoveCheck,
    MoveCastle,
    GameStart,
    GameEnd,
    ClockTick,
    ClockLow,
    ButtonClick,
    Notification,
}

impl SoundEffect {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::MoveNormal => "走棋",
            Self::MoveCapture => "吃子",
            Self::MoveCheck => "将军",
            Self::MoveCastle => "移位",
            Self::GameStart => "开始",
            Self::GameEnd => "结束",
            Self::ClockTick => "时钟",
            Self::ClockLow => "警告",
            Self::ButtonClick => "按钮",
            Self::Notification => "通知",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MoveSoundLibrary {
    pub enabled: bool,
    pub volume: f32,
    pub sounds: Vec<SoundEffect>,
}

impl Default for MoveSoundLibrary {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.8,
            sounds: vec![
                SoundEffect::MoveNormal,
                SoundEffect::MoveCapture,
                SoundEffect::MoveCheck,
                SoundEffect::GameStart,
                SoundEffect::GameEnd,
                SoundEffect::ClockTick,
                SoundEffect::ClockLow,
                SoundEffect::ButtonClick,
                SoundEffect::Notification,
            ],
        }
    }
}

impl MoveSoundLibrary {
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
    pub fn toggle_sound(&mut self, effect: SoundEffect) {
        if self.sounds.contains(&effect) {
            self.sounds.retain(|&s| s != effect);
        } else {
            self.sounds.push(effect);
        }
    }
    pub fn is_enabled(&self, effect: SoundEffect) -> bool {
        self.enabled && self.sounds.contains(&effect)
    }
}

pub fn toggle_sound_library(
    keys: Res<ButtonInput<KeyCode>>,
    mut msl: ResMut<MoveSoundLibrary>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyS) {
        msl.enabled = !msl.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if msl.enabled {
                "音效库已开启"
            } else {
                "音效库已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_toggle() {
        let mut msl = MoveSoundLibrary::default();
        msl.toggle_sound(SoundEffect::MoveNormal);
        assert!(!msl.is_enabled(SoundEffect::MoveNormal));
        msl.toggle_sound(SoundEffect::MoveNormal);
        assert!(msl.is_enabled(SoundEffect::MoveNormal));
    }
    #[test]
    fn test_volume() {
        let mut msl = MoveSoundLibrary::default();
        msl.set_volume(0.5);
        assert_eq!(msl.volume, 0.5);
        msl.set_volume(2.0);
        assert_eq!(msl.volume, 1.0);
    }
}
