//! Piece movement trails visualization.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct TrailPoint {
    pub file: u8,
    pub rank: u8,
    pub alpha: f32,
}

#[derive(Resource, Debug, Clone)]
pub struct MovementTrails {
    pub enabled: bool,
    pub trails: Vec<Vec<TrailPoint>>,
    pub max_length: usize,
    pub fade_speed: f32,
}

impl Default for MovementTrails {
    fn default() -> Self {
        Self {
            enabled: false,
            trails: Vec::new(),
            max_length: 10,
            fade_speed: 0.5,
        }
    }
}

impl MovementTrails {
    pub fn add_trail(&mut self, points: Vec<TrailPoint>) {
        self.trails.push(points);
        if self.trails.len() > self.max_length {
            self.trails.remove(0);
        }
    }
    pub fn update_fade(&mut self, delta: f32) {
        for trail in &mut self.trails {
            for point in trail.iter_mut() {
                point.alpha = (point.alpha - delta * self.fade_speed).max(0.0);
            }
        }
        self.trails.retain(|t| t.iter().any(|p| p.alpha > 0.01));
    }
    pub fn clear(&mut self) {
        self.trails.clear();
    }
}

pub fn toggle_trails(
    keys: Res<ButtonInput<KeyCode>>,
    mut mt: ResMut<MovementTrails>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyT) {
        mt.enabled = !mt.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mt.enabled {
                "移动轨迹已开启"
            } else {
                "移动轨迹已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut mt = MovementTrails::default();
        mt.add_trail(vec![TrailPoint {
            file: 0,
            rank: 0,
            alpha: 1.0,
        }]);
        assert_eq!(mt.trails.len(), 1);
    }
    #[test]
    fn test_fade() {
        let mut mt = MovementTrails::default();
        mt.add_trail(vec![TrailPoint {
            file: 0,
            rank: 0,
            alpha: 1.0,
        }]);
        mt.update_fade(0.5);
        assert!(mt.trails[0][0].alpha < 1.0);
    }
    #[test]
    fn test_max_length() {
        let mut mt = MovementTrails::default();
        mt.max_length = 3;
        for _ in 0..5 {
            mt.add_trail(vec![TrailPoint {
                file: 0,
                rank: 0,
                alpha: 1.0,
            }]);
        }
        assert_eq!(mt.trails.len(), 3);
    }
}
