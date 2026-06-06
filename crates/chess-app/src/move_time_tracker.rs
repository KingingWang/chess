//! Move time tracking and analysis.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct MoveTimeTracker {
    pub times: Vec<f32>,
    pub enabled: bool,
    pub show_average: bool,
}

impl Default for MoveTimeTracker {
    fn default() -> Self {
        Self {
            times: Vec::new(),
            enabled: true,
            show_average: true,
        }
    }
}

impl MoveTimeTracker {
    pub fn record_time(&mut self, time: f32) {
        self.times.push(time);
    }
    pub fn average(&self) -> f32 {
        if self.times.is_empty() {
            0.0
        } else {
            self.times.iter().sum::<f32>() / self.times.len() as f32
        }
    }
    pub fn fastest(&self) -> Option<f32> {
        self.times
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }
    pub fn slowest(&self) -> Option<f32> {
        self.times
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }
    pub fn clear(&mut self) {
        self.times.clear();
    }
}

pub fn toggle_time_tracker(
    keys: Res<ButtonInput<KeyCode>>,
    mut mtt: ResMut<MoveTimeTracker>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyW) {
        mtt.enabled = !mtt.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mtt.enabled {
                "着法时间追踪已开启"
            } else {
                "着法时间追踪已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut mtt = MoveTimeTracker::default();
        mtt.record_time(5.0);
        mtt.record_time(10.0);
        assert_eq!(mtt.average(), 7.5);
        assert_eq!(mtt.fastest(), Some(5.0));
        assert_eq!(mtt.slowest(), Some(10.0));
    }
}
