//! Puzzle generator from game positions.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub fen: String,
    pub solution: Vec<String>,
    pub difficulty: u8,
    pub theme: String,
}

#[derive(Resource, Debug, Clone)]
pub struct PuzzleGenerator {
    pub puzzles: Vec<Puzzle>,
    pub current: usize,
    pub solved: u32,
}

impl Default for PuzzleGenerator {
    fn default() -> Self {
        Self {
            puzzles: vec![
                Puzzle {
                    fen: "4k4/9/9/9/9/9/9/4R4/9/4K4 w - - 0 1".to_string(),
                    solution: vec!["e3e9".to_string()],
                    difficulty: 1,
                    theme: "单车杀王".to_string(),
                },
                Puzzle {
                    fen: "4k4/9/9/9/9/9/9/R8/8R/4K4 w - - 0 1".to_string(),
                    solution: vec!["a3a9".to_string(), "h3h9".to_string()],
                    difficulty: 2,
                    theme: "双车杀王".to_string(),
                },
            ],
            current: 0,
            solved: 0,
        }
    }
}

impl PuzzleGenerator {
    pub fn next_puzzle(&mut self) -> Option<&Puzzle> {
        if self.current + 1 < self.puzzles.len() {
            self.current += 1;
            self.puzzles.get(self.current)
        } else {
            None
        }
    }
    pub fn current_puzzle(&self) -> Option<&Puzzle> {
        self.puzzles.get(self.current)
    }
    pub fn record_solved(&mut self, correct: bool) {
        if correct {
            self.solved += 1;
        }
    }
    pub fn success_rate(&self) -> f32 {
        if self.current == 0 {
            0.0
        } else {
            self.solved as f32 / self.current as f32 * 100.0
        }
    }
}

pub fn next_puzzle_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mut pg: ResMut<PuzzleGenerator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyZ) {
        if let Some(puzzle) = pg.next_puzzle() {
            crate::toast::spawn_toast(
                &mut commands,
                &fonts,
                &format!("下一题: {} (难度{})", puzzle.theme, puzzle.difficulty),
            );
        } else {
            crate::toast::spawn_toast(&mut commands, &fonts, "没有更多题目了");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_next() {
        let mut pg = PuzzleGenerator::default();
        let first = pg.current_puzzle().unwrap();
        assert_eq!(first.difficulty, 1);
        let second = pg.next_puzzle().unwrap();
        assert_eq!(second.difficulty, 2);
    }
}
