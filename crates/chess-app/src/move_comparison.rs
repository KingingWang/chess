//! Move comparison between human and AI moves.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct MoveComparison {
    pub move_idx: usize,
    pub human_move: Option<String>,
    pub ai_move: Option<String>,
    pub human_eval: Option<i32>,
    pub ai_eval: Option<i32>,
}

#[derive(Resource, Debug, Clone)]
pub struct MoveComparisonTracker {
    pub active: bool,
    pub comparisons: Vec<MoveComparison>,
}

impl Default for MoveComparisonTracker {
    fn default() -> Self {
        Self {
            active: false,
            comparisons: Vec::new(),
        }
    }
}

impl MoveComparisonTracker {
    pub fn add_comparison(
        &mut self,
        idx: usize,
        human: String,
        human_eval: i32,
        ai: String,
        ai_eval: i32,
    ) {
        self.comparisons.push(MoveComparison {
            move_idx: idx,
            human_move: Some(human),
            ai_move: Some(ai),
            human_eval: Some(human_eval),
            ai_eval: Some(ai_eval),
        });
    }
    pub fn accuracy_score(&self) -> f32 {
        if self.comparisons.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0;
        for c in &self.comparisons {
            if let (Some(he), Some(ae)) = (c.human_eval, c.ai_eval) {
                let diff = (he - ae).abs() as f32;
                total += (100.0 - diff.min(500.0) / 5.0).max(0.0);
                count += 1;
            }
        }
        if count > 0 {
            total / count as f32
        } else {
            0.0
        }
    }
    pub fn match_count(&self) -> usize {
        self.comparisons
            .iter()
            .filter(|c| c.human_move == c.ai_move)
            .count()
    }
    pub fn clear(&mut self) {
        self.comparisons.clear();
    }
}

pub fn toggle_comparison(
    keys: Res<ButtonInput<KeyCode>>,
    mut mc: ResMut<MoveComparisonTracker>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyF) {
        mc.active = !mc.active;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mc.active {
                "着法对比已开启"
            } else {
                "着法对比已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_accuracy_empty() {
        let mc = MoveComparisonTracker::default();
        assert_eq!(mc.accuracy_score(), 0.0);
    }
    #[test]
    fn test_perfect_match() {
        let mut mc = MoveComparisonTracker::default();
        mc.add_comparison(0, "h2e2".to_string(), 100, "h2e2".to_string(), 100);
        assert_eq!(mc.accuracy_score(), 100.0);
        assert_eq!(mc.match_count(), 1);
    }
}
