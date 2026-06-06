//! AI personality system for different playing styles.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiPersonality {
    Aggressive,
    #[default]
    Balanced,
    Defensive,
    Tactical,
    Positional,
    Creative,
}

impl AiPersonality {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Aggressive => "激进",
            Self::Balanced => "均衡",
            Self::Defensive => "防守",
            Self::Tactical => "战术",
            Self::Positional => "局面",
            Self::Creative => "创新",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Aggressive => Self::Balanced,
            Self::Balanced => Self::Defensive,
            Self::Defensive => Self::Tactical,
            Self::Tactical => Self::Positional,
            Self::Positional => Self::Creative,
            Self::Creative => Self::Aggressive,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct AiPersonalityResource {
    pub personality: AiPersonality,
    pub attack_bonus: i32,
    pub defense_bonus: i32,
    pub randomness: f32,
}

impl Default for AiPersonalityResource {
    fn default() -> Self {
        Self {
            personality: AiPersonality::default(),
            attack_bonus: 0,
            defense_bonus: 0,
            randomness: 0.0,
        }
    }
}

impl AiPersonalityResource {
    pub fn apply_personality(&mut self) {
        match self.personality {
            AiPersonality::Aggressive => {
                self.attack_bonus = 50;
                self.defense_bonus = -20;
                self.randomness = 0.1;
            }
            AiPersonality::Balanced => {
                self.attack_bonus = 0;
                self.defense_bonus = 0;
                self.randomness = 0.0;
            }
            AiPersonality::Defensive => {
                self.attack_bonus = -20;
                self.defense_bonus = 50;
                self.randomness = 0.05;
            }
            AiPersonality::Tactical => {
                self.attack_bonus = 30;
                self.defense_bonus = 10;
                self.randomness = 0.15;
            }
            AiPersonality::Positional => {
                self.attack_bonus = 10;
                self.defense_bonus = 30;
                self.randomness = 0.05;
            }
            AiPersonality::Creative => {
                self.attack_bonus = 20;
                self.defense_bonus = 0;
                self.randomness = 0.3;
            }
        }
    }
}

pub fn cycle_personality(
    keys: Res<ButtonInput<KeyCode>>,
    mut ai: ResMut<AiPersonalityResource>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyA) {
        ai.personality = ai.personality.next();
        ai.apply_personality();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("AI风格: {}", ai.personality.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut p = AiPersonality::Aggressive;
        p = p.next();
        assert_eq!(p, AiPersonality::Balanced);
        p = p.next();
        assert_eq!(p, AiPersonality::Defensive);
    }
    #[test]
    fn test_apply() {
        let mut ai = AiPersonalityResource::default();
        ai.personality = AiPersonality::Aggressive;
        ai.apply_personality();
        assert!(ai.attack_bonus > 0);
    }
}
