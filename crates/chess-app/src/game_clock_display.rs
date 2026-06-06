//! Enhanced game clock display with visual indicators.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct GameClockDisplay {
    pub red_time: f32,
    pub black_time: f32,
    pub is_red_turn: bool,
    pub show_progress_bar: bool,
    pub low_time_threshold: f32,
}

impl Default for GameClockDisplay {
    fn default() -> Self {
        Self {
            red_time: 300.0,
            black_time: 300.0,
            is_red_turn: true,
            show_progress_bar: true,
            low_time_threshold: 30.0,
        }
    }
}

impl GameClockDisplay {
    pub fn format_time(seconds: f32) -> String {
        let mins = (seconds / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        format!("{}:{:02}", mins, secs)
    }
    pub fn tick(&mut self, delta: f32) {
        if self.is_red_turn {
            self.red_time = (self.red_time - delta).max(0.0);
        } else {
            self.black_time = (self.black_time - delta).max(0.0);
        }
    }
    pub fn switch_turn(&mut self) {
        self.is_red_turn = !self.is_red_turn;
    }
    pub fn is_low_time(&self) -> bool {
        let current = if self.is_red_turn {
            self.red_time
        } else {
            self.black_time
        };
        current <= self.low_time_threshold
    }
}

pub fn toggle_clock_display(
    keys: Res<ButtonInput<KeyCode>>,
    mut gcd: ResMut<GameClockDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyF) {
        gcd.show_progress_bar = !gcd.show_progress_bar;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gcd.show_progress_bar {
                "进度条已显示"
            } else {
                "进度条已隐藏"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format() {
        assert_eq!(GameClockDisplay::format_time(90.0), "1:30");
        assert_eq!(GameClockDisplay::format_time(5.0), "0:05");
    }
    #[test]
    fn test_tick() {
        let mut gcd = GameClockDisplay::default();
        gcd.tick(1.0);
        assert!(gcd.red_time < 300.0);
    }
    #[test]
    fn test_low_time() {
        let mut gcd = GameClockDisplay::default();
        gcd.red_time = 20.0;
        assert!(gcd.is_low_time());
    }
}
