//! Endgame puzzle generator from game positions.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct GeneratedPuzzle {
    pub fen: String,
    pub solution: Vec<String>,
    pub difficulty: u8,
    pub description: String,
}

#[derive(Resource, Debug, Clone)]
pub struct EndgamePuzzleGen {
    pub enabled: bool,
    pub generated: Vec<GeneratedPuzzle>,
    pub min_moves_for_puzzle: usize,
}

impl Default for EndgamePuzzleGen {
    fn default() -> Self {
        Self {
            enabled: false,
            generated: Vec::new(),
            min_moves_for_puzzle: 20,
        }
    }
}

impl EndgamePuzzleGen {
    pub fn generate_from_position(&mut self, fen: &str, solution: Vec<String>, difficulty: u8) {
        self.generated.push(GeneratedPuzzle {
            fen: fen.to_string(),
            solution,
            difficulty,
            description: format!("残局难题 #{}", self.generated.len() + 1),
        });
    }
    pub fn puzzle_count(&self) -> usize {
        self.generated.len()
    }
    pub fn clear(&mut self) {
        self.generated.clear();
    }
}

pub fn toggle_generator(
    keys: Res<ButtonInput<KeyCode>>,
    mut gen: ResMut<EndgamePuzzleGen>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyZ) {
        gen.enabled = !gen.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gen.enabled {
                "残局生成器已开启"
            } else {
                "残局生成器已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_generate() {
        let mut gen = EndgamePuzzleGen::default();
        gen.generate_from_position("fen", vec!["e3e9".to_string()], 2);
        assert_eq!(gen.puzzle_count(), 1);
    }
    #[test]
    fn test_clear() {
        let mut gen = EndgamePuzzleGen::default();
        gen.generate_from_position("fen", vec![], 1);
        gen.clear();
        assert_eq!(gen.puzzle_count(), 0);
    }
}
