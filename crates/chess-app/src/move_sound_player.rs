//! Move sound player for audio feedback.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveSoundType {
    Normal,
    Capture,
    Check,
    Checkmate,
    Undo,
}

impl MoveSoundType {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Normal => "走棋",
            Self::Capture => "吃子",
            Self::Check => "将军",
            Self::Checkmate => "将杀",
            Self::Undo => "悔棋",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MoveSoundPlayer {
    pub enabled: bool,
    pub volume: f32,
    pub last_sound: Option<MoveSoundType>,
}

impl Default for MoveSoundPlayer {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.8,
            last_sound: None,
        }
    }
}

impl MoveSoundPlayer {
    pub fn play(&mut self, sound_type: MoveSoundType) {
        if self.enabled {
            self.last_sound = Some(sound_type);
        }
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
}

pub fn toggle_move_sounds(
    keys: Res<ButtonInput<KeyCode>>,
    mut msp: ResMut<MoveSoundPlayer>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyS) {
        msp.enabled = !msp.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if msp.enabled {
                "着法音效已开启"
            } else {
                "着法音效已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_play() {
        let mut msp = MoveSoundPlayer::default();
        msp.play(MoveSoundType::Capture);
        assert_eq!(msp.last_sound, Some(MoveSoundType::Capture));
    }
    #[test]
    fn test_disabled() {
        let mut msp = MoveSoundPlayer::default();
        msp.enabled = false;
        msp.play(MoveSoundType::Check);
        assert!(msp.last_sound.is_none());
    }
}
