//! AI difficulty scaling for adaptive challenge.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DifficultyLevel {
    Beginner,
    Easy,
    Medium,
    Hard,
    Expert,
    Master,
}

impl DifficultyLevel {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Beginner => "新手",
            Self::Easy => "简单",
            Self::Medium => "中等",
            Self::Hard => "困难",
            Self::Expert => "专家",
            Self::Master => "大师",
        }
    }
    pub fn depth_limit(&self) -> u32 {
        match self {
            Self::Beginner => 2,
            Self::Easy => 4,
            Self::Medium => 6,
            Self::Hard => 8,
            Self::Expert => 10,
            Self::Master => 12,
        }
    }
    pub fn time_limit_ms(&self) -> u32 {
        match self {
            Self::Beginner => 100,
            Self::Easy => 500,
            Self::Medium => 1000,
            Self::Hard => 2000,
            Self::Expert => 5000,
            Self::Master => 10000,
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Beginner => Self::Easy,
            Self::Easy => Self::Medium,
            Self::Medium => Self::Hard,
            Self::Hard => Self::Expert,
            Self::Expert => Self::Master,
            Self::Master => Self::Beginner,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct AiDifficultyScaling {
    pub current: DifficultyLevel,
    pub adaptive: bool,
    pub player_win_rate: f32,
}

impl Default for AiDifficultyScaling {
    fn default() -> Self {
        Self {
            current: DifficultyLevel::Medium,
            adaptive: false,
            player_win_rate: 0.5,
        }
    }
}

impl AiDifficultyScaling {
    pub fn adjust_difficulty(&mut self) {
        if !self.adaptive {
            return;
        }
        if self.player_win_rate > 0.7 {
            self.current = self.current.next();
        } else if self.player_win_rate < 0.3 {
            self.current = match self.current {
                DifficultyLevel::Beginner => DifficultyLevel::Beginner,
                DifficultyLevel::Easy => DifficultyLevel::Beginner,
                DifficultyLevel::Medium => DifficultyLevel::Easy,
                DifficultyLevel::Hard => DifficultyLevel::Medium,
                DifficultyLevel::Expert => DifficultyLevel::Hard,
                DifficultyLevel::Master => DifficultyLevel::Expert,
            };
        }
    }
}

pub fn cycle_difficulty(
    keys: Res<ButtonInput<KeyCode>>,
    mut ads: ResMut<AiDifficultyScaling>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyD) {
        ads.current = ads.current.next();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("AI难度: {}", ads.current.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut d = DifficultyLevel::Beginner;
        d = d.next();
        assert_eq!(d, DifficultyLevel::Easy);
    }
    #[test]
    fn test_adjust() {
        let mut ads = AiDifficultyScaling::default();
        ads.adaptive = true;
        ads.player_win_rate = 0.8;
        ads.adjust_difficulty();
        assert_eq!(ads.current, DifficultyLevel::Hard);
    }
}
