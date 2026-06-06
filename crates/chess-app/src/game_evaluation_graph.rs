//! Game evaluation graph visualization.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct GameEvaluationGraph {
    pub enabled: bool,
    pub evaluations: Vec<i32>,
    pub max_display: usize,
}

impl Default for GameEvaluationGraph {
    fn default() -> Self {
        Self {
            enabled: false,
            evaluations: Vec::new(),
            max_display: 100,
        }
    }
}

impl GameEvaluationGraph {
    pub fn add_eval(&mut self, eval: i32) {
        self.evaluations.push(eval);
        if self.evaluations.len() > self.max_display {
            self.evaluations.remove(0);
        }
    }
    pub fn min_eval(&self) -> i32 {
        self.evaluations.iter().copied().min().unwrap_or(0)
    }
    pub fn max_eval(&self) -> i32 {
        self.evaluations.iter().copied().max().unwrap_or(0)
    }
    pub fn current_eval(&self) -> i32 {
        self.evaluations.last().copied().unwrap_or(0)
    }
    pub fn clear(&mut self) {
        self.evaluations.clear();
    }
}

pub fn toggle_eval_graph(
    keys: Res<ButtonInput<KeyCode>>,
    mut eg: ResMut<GameEvaluationGraph>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyE) {
        eg.enabled = !eg.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if eg.enabled {
                "评估图表已开启"
            } else {
                "评估图表已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut eg = GameEvaluationGraph::default();
        eg.add_eval(100);
        eg.add_eval(-50);
        assert_eq!(eg.evaluations.len(), 2);
    }
    #[test]
    fn test_min_max() {
        let mut eg = GameEvaluationGraph::default();
        eg.add_eval(100);
        eg.add_eval(-50);
        eg.add_eval(200);
        assert_eq!(eg.min_eval(), -50);
        assert_eq!(eg.max_eval(), 200);
    }
}
