//! Move confidence indicator.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
    Uncertain,
}

impl Confidence {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::High => "确定",
            Self::Medium => "较确定",
            Self::Low => "不确定",
            Self::Uncertain => "猜测",
        }
    }
    pub fn from_time(time_secs: f32) -> Self {
        match time_secs {
            t if t > 30.0 => Self::High,
            t if t > 10.0 => Self::Medium,
            t if t > 3.0 => Self::Low,
            _ => Self::Uncertain,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MoveConfidence {
    pub enabled: bool,
    pub confidences: Vec<(usize, Confidence)>,
}

impl MoveConfidence {
    pub fn record(&mut self, move_idx: usize, time_secs: f32) {
        self.confidences
            .push((move_idx, Confidence::from_time(time_secs)));
    }
    pub fn average_confidence(&self) -> f32 {
        if self.confidences.is_empty() {
            return 0.0;
        }
        let scores: f32 = self
            .confidences
            .iter()
            .map(|(_, c)| match c {
                Confidence::High => 1.0,
                Confidence::Medium => 0.75,
                Confidence::Low => 0.5,
                Confidence::Uncertain => 0.25,
            })
            .sum();
        scores / self.confidences.len() as f32
    }
}

pub fn toggle_confidence(
    keys: Res<ButtonInput<KeyCode>>,
    mut mc: ResMut<MoveConfidence>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyC) {
        mc.enabled = !mc.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mc.enabled {
                "置信度显示已开启"
            } else {
                "置信度显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_from_time() {
        assert_eq!(Confidence::from_time(60.0), Confidence::High);
        assert_eq!(Confidence::from_time(15.0), Confidence::Medium);
        assert_eq!(Confidence::from_time(5.0), Confidence::Low);
        assert_eq!(Confidence::from_time(1.0), Confidence::Uncertain);
    }
    #[test]
    fn test_average() {
        let mut mc = MoveConfidence::default();
        mc.record(0, 60.0);
        mc.record(1, 60.0);
        assert_eq!(mc.average_confidence(), 1.0);
    }
}
