//! Clock alarm system for time warnings.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct ClockAlarm {
    pub enabled: bool,
    pub warning_threshold: f32,
    pub critical_threshold: f32,
    pub last_warning: f32,
    pub alarm_playing: bool,
}

impl Default for ClockAlarm {
    fn default() -> Self {
        Self {
            enabled: true,
            warning_threshold: 60.0,
            critical_threshold: 30.0,
            last_warning: f32::MAX,
            alarm_playing: false,
        }
    }
}

impl ClockAlarm {
    pub fn should_warn(&mut self, remaining: f32) -> bool {
        if !self.enabled {
            return false;
        }
        if remaining <= self.critical_threshold && self.last_warning > self.critical_threshold {
            self.last_warning = remaining;
            return true;
        }
        if remaining <= self.warning_threshold && self.last_warning > self.warning_threshold {
            self.last_warning = remaining;
            return true;
        }
        false
    }
    pub fn reset(&mut self) {
        self.last_warning = f32::MAX;
        self.alarm_playing = false;
    }
    pub fn alarm_level(&self, remaining: f32) -> &'static str {
        if remaining <= self.critical_threshold {
            "critical"
        } else if remaining <= self.warning_threshold {
            "warning"
        } else {
            "normal"
        }
    }
}

pub fn toggle_alarm(
    keys: Res<ButtonInput<KeyCode>>,
    mut alarm: ResMut<ClockAlarm>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyK) {
        alarm.enabled = !alarm.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if alarm.enabled {
                "时钟警报已开启"
            } else {
                "时钟警报已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_warning() {
        let mut a = ClockAlarm::default();
        assert!(a.should_warn(55.0));
        assert!(!a.should_warn(55.0));
        assert!(a.should_warn(25.0));
    }
    #[test]
    fn test_levels() {
        let a = ClockAlarm::default();
        assert_eq!(a.alarm_level(120.0), "normal");
        assert_eq!(a.alarm_level(55.0), "warning");
        assert_eq!(a.alarm_level(25.0), "critical");
    }
}
