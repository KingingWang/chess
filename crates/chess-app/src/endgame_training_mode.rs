//! Interactive endgame training mode with guided practice.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct EndgamePuzzle {
    pub fen: String,
    pub solution: Vec<String>,
    pub difficulty: u8,
    pub name: String,
}

#[derive(Resource, Debug, Clone)]
pub struct EndgameTrainingMode {
    pub active: bool,
    pub puzzles: Vec<EndgamePuzzle>,
    pub current_puzzle: usize,
    pub solved: u32,
    pub attempts: u32,
}

impl Default for EndgameTrainingMode {
    fn default() -> Self {
        Self {
            active: false,
            puzzles: vec![
                EndgamePuzzle {
                    fen: "4k4/9/9/9/9/9/9/4R4/9/4K4 w - - 0 1".to_string(),
                    solution: vec!["e3e9".to_string()],
                    difficulty: 1,
                    name: "单车杀王".to_string(),
                },
                EndgamePuzzle {
                    fen: "4k4/9/9/9/9/9/9/R8/8R/4K4 w - - 0 1".to_string(),
                    solution: vec!["a3a9".to_string(), "h3h9".to_string()],
                    difficulty: 2,
                    name: "双车杀王".to_string(),
                },
            ],
            current_puzzle: 0,
            solved: 0,
            attempts: 0,
        }
    }
}

impl EndgameTrainingMode {
    pub fn next_puzzle(&mut self) {
        if self.current_puzzle + 1 < self.puzzles.len() {
            self.current_puzzle += 1;
        }
    }

    pub fn record_attempt(&mut self, correct: bool) {
        self.attempts += 1;
        if correct {
            self.solved += 1;
        }
    }

    pub fn success_rate(&self) -> f32 {
        if self.attempts == 0 {
            0.0
        } else {
            self.solved as f32 / self.attempts as f32 * 100.0
        }
    }

    pub fn current_puzzle(&self) -> Option<&EndgamePuzzle> {
        self.puzzles.get(self.current_puzzle)
    }
}

pub fn toggle_training_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut etm: ResMut<EndgameTrainingMode>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyE) {
        etm.active = !etm.active;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if etm.active {
                "残局训练模式已开启"
            } else {
                "残局训练模式已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record_attempt() {
        let mut etm = EndgameTrainingMode::default();
        etm.record_attempt(true);
        etm.record_attempt(false);
        assert_eq!(etm.attempts, 2);
        assert_eq!(etm.solved, 1);
        assert_eq!(etm.success_rate(), 50.0);
    }
    #[test]
    fn test_next_puzzle() {
        let mut etm = EndgameTrainingMode::default();
        etm.next_puzzle();
        assert_eq!(etm.current_puzzle, 1);
    }
}
