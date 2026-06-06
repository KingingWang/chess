//! Manual position evaluator for learning.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct PositionEvaluator {
    pub active: bool,
    pub user_eval: i32,
    pub actual_eval: Option<i32>,
    pub history: Vec<(i32, i32)>,
}

impl PositionEvaluator {
    pub fn submit_eval(&mut self) {
        if let Some(actual) = self.actual_eval {
            self.history.push((self.user_eval, actual));
        }
    }
    pub fn accuracy(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let errors: f32 = self
            .history
            .iter()
            .map(|(u, a)| (*u - *a).abs() as f32)
            .sum();
        let avg_error = errors / self.history.len() as f32;
        (100.0 - (avg_error / 10.0)).max(0.0)
    }
    pub fn eval_label(eval: i32) -> String {
        if eval.abs() >= 9900 {
            let mate_in = (10000 - eval.abs()) / 2;
            format!("M{} ({})", mate_in, if eval > 0 { "红胜" } else { "黑胜" })
        } else {
            format!("{:+.2}", eval as f32 / 100.0)
        }
    }
}

pub fn toggle_position_eval(
    keys: Res<ButtonInput<KeyCode>>,
    mut pe: ResMut<PositionEvaluator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyV) {
        pe.active = !pe.active;
        let msg = if pe.active {
            "评估练习: 猜测当前局面评估值"
        } else {
            "评估练习已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_accuracy_empty() {
        let pe = PositionEvaluator::default();
        assert_eq!(pe.accuracy(), 0.0);
    }
    #[test]
    fn test_eval_label() {
        assert_eq!(PositionEvaluator::eval_label(150), "+1.50");
        assert_eq!(PositionEvaluator::eval_label(-250), "-2.50");
        assert!(PositionEvaluator::eval_label(9990).contains("M"));
    }
    #[test]
    fn test_submit() {
        let mut pe = PositionEvaluator::default();
        pe.user_eval = 100;
        pe.actual_eval = Some(120);
        pe.submit_eval();
        assert_eq!(pe.history.len(), 1);
    }
}
