//! Time pressure visual display with color-coded indicators.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct TimePressureDisplay {
    pub enabled: bool,
    pub show_colors: bool,
    pub pulse_animation: bool,
}

impl Default for TimePressureDisplay {
    fn default() -> Self {
        Self {
            enabled: true,
            show_colors: true,
            pulse_animation: true,
        }
    }
}

impl TimePressureDisplay {
    pub fn pressure_color(&self, remaining: f32, total: f32) -> [f32; 3] {
        if !self.show_colors {
            return [1.0, 1.0, 1.0];
        }
        let ratio = if total > 0.0 { remaining / total } else { 1.0 };
        if ratio > 0.5 {
            [0.4, 0.8, 0.4]
        }
        // Green
        else if ratio > 0.25 {
            [0.9, 0.8, 0.3]
        }
        // Yellow
        else if ratio > 0.1 {
            [0.9, 0.5, 0.2]
        }
        // Orange
        else {
            [0.9, 0.2, 0.2]
        } // Red
    }
}

pub fn toggle_pressure_display(
    keys: Res<ButtonInput<KeyCode>>,
    mut tp: ResMut<TimePressureDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyU) {
        tp.enabled = !tp.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tp.enabled {
                "时间压力显示已开启"
            } else {
                "时间压力显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_colors() {
        let tp = TimePressureDisplay::default();
        let green = tp.pressure_color(60.0, 100.0);
        let red = tp.pressure_color(5.0, 100.0);
        assert!(green[1] > green[0]); // Green dominant
        assert!(red[0] > red[1]); // Red dominant
    }
}
