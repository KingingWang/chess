//! Thinking time warnings for time management.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePressure {
    Relaxed,
    Moderate,
    Critical,
    Extreme,
}

impl TimePressure {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Relaxed => "从容",
            Self::Moderate => "适中",
            Self::Critical => "紧迫",
            Self::Extreme => "极限",
        }
    }
    pub fn from_remaining_ratio(ratio: f32) -> Self {
        if ratio > 0.5 {
            Self::Relaxed
        } else if ratio > 0.25 {
            Self::Moderate
        } else if ratio > 0.1 {
            Self::Critical
        } else {
            Self::Extreme
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct TimeWarnings {
    pub enabled: bool,
    pub red_pressure: TimePressure,
    pub black_pressure: TimePressure,
    pub warning_threshold_secs: f32,
    pub critical_threshold_secs: f32,
}

impl Default for TimeWarnings {
    fn default() -> Self {
        Self {
            enabled: true,
            red_pressure: TimePressure::Relaxed,
            black_pressure: TimePressure::Relaxed,
            warning_threshold_secs: 60.0,
            critical_threshold_secs: 30.0,
        }
    }
}

impl TimeWarnings {
    pub fn update_pressure(&mut self, is_red: bool, remaining_secs: f32, total_secs: f32) {
        let ratio = if total_secs > 0.0 {
            remaining_secs / total_secs
        } else {
            1.0
        };
        let pressure = TimePressure::from_remaining_ratio(ratio);
        if is_red {
            self.red_pressure = pressure;
        } else {
            self.black_pressure = pressure;
        }
    }
}

pub fn toggle_time_warnings(
    keys: Res<ButtonInput<KeyCode>>,
    mut tw: ResMut<TimeWarnings>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyW) {
        tw.enabled = !tw.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tw.enabled {
                "用时提醒已开启"
            } else {
                "用时提醒已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pressure_levels() {
        assert_eq!(
            TimePressure::from_remaining_ratio(0.8),
            TimePressure::Relaxed
        );
        assert_eq!(
            TimePressure::from_remaining_ratio(0.3),
            TimePressure::Moderate
        );
        assert_eq!(
            TimePressure::from_remaining_ratio(0.15),
            TimePressure::Critical
        );
        assert_eq!(
            TimePressure::from_remaining_ratio(0.05),
            TimePressure::Extreme
        );
    }
}
