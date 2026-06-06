//! Tactical pattern detector for common combinations.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalPattern {
    Fork,
    Pin,
    Skewer,
    DiscoveredAttack,
    DoubleCheck,
    BackRankMate,
}

impl TacticalPattern {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Fork => "捉双",
            Self::Pin => "牵制",
            Self::Skewer => "串打",
            Self::DiscoveredAttack => "闪击",
            Self::DoubleCheck => "双将",
            Self::BackRankMate => "底线杀",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct TacticalPatterns {
    pub enabled: bool,
    pub detected: Vec<TacticalPattern>,
    pub history: Vec<(usize, TacticalPattern)>,
}

impl Default for TacticalPatterns {
    fn default() -> Self {
        Self {
            enabled: true,
            detected: Vec::new(),
            history: Vec::new(),
        }
    }
}

impl TacticalPatterns {
    pub fn record(&mut self, move_idx: usize, pattern: TacticalPattern) {
        self.detected.push(pattern);
        self.history.push((move_idx, pattern));
    }
    pub fn clear(&mut self) {
        self.detected.clear();
    }
    pub fn pattern_count(&self, pattern: TacticalPattern) -> usize {
        self.history.iter().filter(|(_, p)| *p == pattern).count()
    }
}

pub fn toggle_patterns(
    keys: Res<ButtonInput<KeyCode>>,
    mut tp: ResMut<TacticalPatterns>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyY) {
        tp.enabled = !tp.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tp.enabled {
                "战术检测已开启"
            } else {
                "战术检测已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut tp = TacticalPatterns::default();
        tp.record(0, TacticalPattern::Fork);
        assert_eq!(tp.detected.len(), 1);
    }
    #[test]
    fn test_count() {
        let mut tp = TacticalPatterns::default();
        tp.record(0, TacticalPattern::Fork);
        tp.record(5, TacticalPattern::Fork);
        assert_eq!(tp.pattern_count(TacticalPattern::Fork), 2);
    }
    #[test]
    fn test_labels() {
        assert_eq!(TacticalPattern::Fork.label_cn(), "捉双");
        assert_eq!(TacticalPattern::Pin.label_cn(), "牵制");
    }
}
