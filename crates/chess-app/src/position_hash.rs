//! Position hash display using Zobrist hashing.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct PositionHash {
    pub visible: bool,
    pub current_hash: u64,
    pub hash_history: Vec<u64>,
}

impl PositionHash {
    pub fn update_hash(&mut self, hash: u64) {
        self.current_hash = hash;
        self.hash_history.push(hash);
    }
    pub fn has_repetition(&self) -> bool {
        let mut counts = std::collections::HashMap::new();
        for &h in &self.hash_history {
            *counts.entry(h).or_insert(0u32) += 1;
        }
        counts.values().any(|&c| c >= 3)
    }
    pub fn format_hash(hash: u64) -> String {
        format!("{:016X}", hash)
    }
    pub fn clear(&mut self) {
        self.current_hash = 0;
        self.hash_history.clear();
    }
}

pub fn toggle_hash_display(
    keys: Res<ButtonInput<KeyCode>>,
    mut h: ResMut<PositionHash>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyH) {
        h.visible = !h.visible;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if h.visible {
                "位置哈希已显示"
            } else {
                "位置哈希已隐藏"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format() {
        assert_eq!(PositionHash::format_hash(0xDEADBEEF), "00000000DEADBEEF");
    }
    #[test]
    fn test_repetition() {
        let mut h = PositionHash::default();
        h.update_hash(1);
        h.update_hash(2);
        h.update_hash(1);
        h.update_hash(2);
        h.update_hash(1);
        assert!(h.has_repetition());
    }
    #[test]
    fn test_no_repetition() {
        let mut h = PositionHash::default();
        h.update_hash(1);
        h.update_hash(2);
        h.update_hash(3);
        assert!(!h.has_repetition());
    }
}
