//! Endgame training mode for practicing specific endgame positions.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct EndgameExercise {
    pub name_cn: String,
    pub fen: String,
    pub solution: Vec<String>,
    pub difficulty: u8,
    pub max_moves: u32,
}

#[derive(Resource, Debug)]
pub struct EndgameTraining {
    pub active: bool,
    pub exercises: Vec<EndgameExercise>,
    pub current_idx: usize,
    pub completed: Vec<bool>,
    pub attempts: u32,
}

impl Default for EndgameTraining {
    fn default() -> Self {
        let exercises = vec![
            EndgameExercise {
                name_cn: "单车杀王".to_string(),
                fen: "4k4/9/9/9/9/9/9/4R4/9/4K4 w - - 0 1".to_string(),
                solution: vec!["e3e9".to_string()],
                difficulty: 1,
                max_moves: 20,
            },
            EndgameExercise {
                name_cn: "双车杀王".to_string(),
                fen: "4k4/9/9/9/9/9/9/R8/8R/4K4 w - - 0 1".to_string(),
                solution: vec!["a3a9".to_string(), "h3h9".to_string()],
                difficulty: 1,
                max_moves: 10,
            },
        ];
        let count = exercises.len();
        Self {
            active: false,
            exercises,
            current_idx: 0,
            completed: vec![false; count],
            attempts: 0,
        }
    }
}

impl EndgameTraining {
    pub fn current_exercise(&self) -> Option<&EndgameExercise> {
        self.exercises.get(self.current_idx)
    }
    pub fn next_exercise(&mut self) {
        if self.current_idx + 1 < self.exercises.len() {
            self.current_idx += 1;
        }
    }
    pub fn mark_complete(&mut self) {
        if self.current_idx < self.completed.len() {
            self.completed[self.current_idx] = true;
        }
    }
    pub fn progress(&self) -> f32 {
        if self.completed.is_empty() {
            return 0.0;
        }
        self.completed.iter().filter(|&&c| c).count() as f32 / self.completed.len() as f32 * 100.0
    }
}

pub fn toggle_endgame_training(
    keys: Res<ButtonInput<KeyCode>>,
    mut training: ResMut<EndgameTraining>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyT) {
        training.active = !training.active;
        let msg = if training.active {
            let ex = training
                .current_exercise()
                .map(|e| e.name_cn.as_str())
                .unwrap_or("?");
            format!("残局练习: {}", ex)
        } else {
            "残局练习已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let t = EndgameTraining::default();
        assert!(!t.active);
        assert!(!t.exercises.is_empty());
    }
    #[test]
    fn test_progress() {
        let t = EndgameTraining::default();
        assert_eq!(t.progress(), 0.0);
    }
    #[test]
    fn test_mark_complete() {
        let mut t = EndgameTraining::default();
        t.mark_complete();
        assert!(t.completed[0]);
        assert!(t.progress() > 0.0);
    }
}
