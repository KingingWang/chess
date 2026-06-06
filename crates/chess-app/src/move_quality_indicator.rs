//! Real-time move quality indicator (!, ?, !!, ??).

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveQuality {
    Excellent,   // !!
    Good,        // !
    Interesting, // !?
    Dubious,     // ?!
    Mistake,     // ?
    Blunder,     // ??
}

impl MoveQuality {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Excellent => "!!",
            Self::Good => "!",
            Self::Interesting => "!?",
            Self::Dubious => "?!",
            Self::Mistake => "?",
            Self::Blunder => "??",
        }
    }

    pub fn from_eval_diff(diff: i32) -> Self {
        match diff {
            d if d > 200 => Self::Excellent,
            d if d > 50 => Self::Good,
            d if d > 0 => Self::Interesting,
            d if d > -50 => Self::Dubious,
            d if d > -200 => Self::Mistake,
            _ => Self::Blunder,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MoveQualityIndicator {
    pub enabled: bool,
    pub last_quality: Option<MoveQuality>,
}

impl Default for MoveQualityIndicator {
    fn default() -> Self {
        Self {
            enabled: true,
            last_quality: None,
        }
    }
}

pub fn toggle_quality_indicator(
    keys: Res<ButtonInput<KeyCode>>,
    mut mqi: ResMut<MoveQualityIndicator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyQ) {
        mqi.enabled = !mqi.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mqi.enabled {
                "着法质量指示器已开启"
            } else {
                "着法质量指示器已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_symbols() {
        assert_eq!(MoveQuality::Excellent.symbol(), "!!");
        assert_eq!(MoveQuality::Blunder.symbol(), "??");
    }
    #[test]
    fn test_from_diff() {
        assert_eq!(MoveQuality::from_eval_diff(300), MoveQuality::Excellent);
        assert_eq!(MoveQuality::from_eval_diff(-300), MoveQuality::Blunder);
    }
}
