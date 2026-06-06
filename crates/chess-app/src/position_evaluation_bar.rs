//! Visual position evaluation bar.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PositionEvaluationBar {
    pub enabled: bool,
    pub evaluation: i32, // centipawns, positive = red advantage
    pub show_numerical: bool,
}

impl Default for PositionEvaluationBar {
    fn default() -> Self {
        Self {
            enabled: true,
            evaluation: 0,
            show_numerical: true,
        }
    }
}

impl PositionEvaluationBar {
    pub fn update_evaluation(&mut self, eval: i32) {
        self.evaluation = eval;
    }

    pub fn red_advantage(&self) -> f32 {
        // Convert centipawns to percentage (50% = equal)
        let clamped = self.evaluation.clamp(-1000, 1000);
        50.0 + (clamped as f32 / 20.0)
    }

    pub fn evaluation_text(&self) -> String {
        if self.evaluation.abs() >= 9900 {
            let mate_in = (10000 - self.evaluation.abs()) / 2;
            if self.evaluation > 0 {
                format!("红胜 M{}", mate_in)
            } else {
                format!("黑胜 M{}", mate_in)
            }
        } else {
            let pawns = self.evaluation as f32 / 100.0;
            format!("{:+.2}", pawns)
        }
    }
}

pub fn toggle_eval_bar(
    keys: Res<ButtonInput<KeyCode>>,
    mut peb: ResMut<PositionEvaluationBar>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyV) {
        peb.enabled = !peb.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if peb.enabled {
                "评估条已显示"
            } else {
                "评估条已隐藏"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_red_advantage() {
        let mut peb = PositionEvaluationBar::default();
        peb.update_evaluation(0);
        assert_eq!(peb.red_advantage(), 50.0);
        peb.update_evaluation(500);
        assert!(peb.red_advantage() > 50.0);
    }
    #[test]
    fn test_evaluation_text() {
        let mut peb = PositionEvaluationBar::default();
        peb.update_evaluation(150);
        assert_eq!(peb.evaluation_text(), "+1.50");
        peb.update_evaluation(9990);
        assert!(peb.evaluation_text().contains("红胜"));
    }
}
