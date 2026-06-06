//! Individual volume control per sound type.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundType {
    Move,
    Capture,
    Check,
    Checkmate,
    Undo,
    Clock,
    Ui,
}

impl SoundType {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Move => "走棋",
            Self::Capture => "吃子",
            Self::Check => "将军",
            Self::Checkmate => "将杀",
            Self::Undo => "悔棋",
            Self::Clock => "时钟",
            Self::Ui => "界面",
        }
    }
    pub fn all() -> Vec<SoundType> {
        vec![
            Self::Move,
            Self::Capture,
            Self::Check,
            Self::Checkmate,
            Self::Undo,
            Self::Clock,
            Self::Ui,
        ]
    }
}

#[derive(Resource, Debug, Clone)]
pub struct SoundVolumePerType {
    pub volumes: HashMap<SoundType, f32>,
    pub master: f32,
}

impl Default for SoundVolumePerType {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        for st in SoundType::all() {
            volumes.insert(st, 1.0);
        }
        Self {
            volumes,
            master: 1.0,
        }
    }
}

impl SoundVolumePerType {
    pub fn effective_volume(&self, sound_type: SoundType) -> f32 {
        self.volumes.get(&sound_type).copied().unwrap_or(1.0) * self.master
    }
    pub fn set_volume(&mut self, sound_type: SoundType, volume: f32) {
        self.volumes.insert(sound_type, volume.clamp(0.0, 1.0));
    }
    pub fn set_master(&mut self, volume: f32) {
        self.master = volume.clamp(0.0, 1.0);
    }
}

pub fn cycle_sound_focus(
    keys: Res<ButtonInput<KeyCode>>,
    mut sv: ResMut<SoundVolumePerType>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyK) {
        sv.master = if sv.master > 0.5 { 0.0 } else { 1.0 };
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if sv.master > 0.5 {
                "音效已开启"
            } else {
                "音效已静音"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_effective() {
        let mut sv = SoundVolumePerType::default();
        sv.set_volume(SoundType::Move, 0.5);
        sv.set_master(0.8);
        assert!((sv.effective_volume(SoundType::Move) - 0.4).abs() < 0.01);
    }
    #[test]
    fn test_mute() {
        let mut sv = SoundVolumePerType::default();
        sv.set_master(0.0);
        assert_eq!(sv.effective_volume(SoundType::Move), 0.0);
    }
}
