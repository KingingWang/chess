//! Game tags and categories for organizing saved games.

use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource, Debug, Clone, Default)]
pub struct GameTags {
    pub tags: HashSet<String>,
    pub current_game_tags: HashSet<String>,
}

impl GameTags {
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.insert(tag.to_string());
        self.current_game_tags.insert(tag.to_string());
    }
    pub fn remove_tag(&mut self, tag: &str) {
        self.current_game_tags.remove(tag);
    }
    pub fn all_tags(&self) -> Vec<&String> {
        self.tags.iter().collect()
    }
    pub fn current_tags(&self) -> Vec<&String> {
        self.current_game_tags.iter().collect()
    }
    pub fn clear_current(&mut self) {
        self.current_game_tags.clear();
    }
}

pub fn add_game_tag(
    keys: Res<ButtonInput<KeyCode>>,
    mut gt: ResMut<GameTags>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyY) {
        let tag = format!("game_{}", gt.tags.len() + 1);
        gt.add_tag(&tag);
        crate::toast::spawn_toast(&mut commands, &fonts, &format!("标签已添加: {}", tag));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut gt = GameTags::default();
        gt.add_tag("重要");
        assert_eq!(gt.all_tags().len(), 1);
    }
    #[test]
    fn test_remove() {
        let mut gt = GameTags::default();
        gt.add_tag("test");
        gt.remove_tag("test");
        assert!(gt.current_tags().is_empty());
    }
}
