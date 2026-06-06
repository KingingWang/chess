//! Position similarity detection for finding recurring patterns.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PositionSimilarity {
    pub enabled: bool,
    pub saved_positions: Vec<(String, usize)>,
    pub similarity_threshold: f32,
}

impl Default for PositionSimilarity {
    fn default() -> Self {
        Self {
            enabled: false,
            saved_positions: Vec::new(),
            similarity_threshold: 0.8,
        }
    }
}

impl PositionSimilarity {
    pub fn save_position(&mut self, fen: &str, move_idx: usize) {
        self.saved_positions.push((fen.to_string(), move_idx));
    }
    pub fn find_similar(&self, fen: &str) -> Vec<usize> {
        self.saved_positions
            .iter()
            .filter(|(saved, _)| Self::similarity(saved, fen) >= self.similarity_threshold)
            .map(|(_, idx)| *idx)
            .collect()
    }
    fn similarity(a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let matches = a_chars
            .iter()
            .zip(b_chars.iter())
            .filter(|(a, b)| a == b)
            .count();
        let max_len = a_chars.len().max(b_chars.len());
        if max_len == 0 {
            1.0
        } else {
            matches as f32 / max_len as f32
        }
    }
}

pub fn toggle_similarity(
    keys: Res<ButtonInput<KeyCode>>,
    mut ps: ResMut<PositionSimilarity>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyS) {
        ps.enabled = !ps.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ps.enabled {
                "相似位置检测已开启"
            } else {
                "相似位置检测已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_identical() {
        assert_eq!(PositionSimilarity::similarity("abc", "abc"), 1.0);
    }
    #[test]
    fn test_partial() {
        assert!(PositionSimilarity::similarity("abcd", "abxx") > 0.4);
    }
    #[test]
    fn test_find() {
        let mut ps = PositionSimilarity::default();
        ps.save_position("test123", 5);
        let found = ps.find_similar("test123");
        assert_eq!(found.len(), 1);
    }
}
