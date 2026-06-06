//! Game collection manager for organizing saved games.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GameCollection {
    pub name: String,
    pub description: String,
    pub game_ids: Vec<u32>,
    pub tags: Vec<String>,
}

#[derive(Resource, Debug, Clone)]
pub struct GameCollectionManager {
    pub collections: HashMap<String, GameCollection>,
    pub visible: bool,
}

impl Default for GameCollectionManager {
    fn default() -> Self {
        Self {
            collections: HashMap::new(),
            visible: false,
        }
    }
}

impl GameCollectionManager {
    pub fn create_collection(&mut self, name: &str, description: &str) {
        self.collections.insert(
            name.to_string(),
            GameCollection {
                name: name.to_string(),
                description: description.to_string(),
                game_ids: Vec::new(),
                tags: Vec::new(),
            },
        );
    }
    pub fn add_game_to_collection(&mut self, collection: &str, game_id: u32) {
        if let Some(coll) = self.collections.get_mut(collection) {
            if !coll.game_ids.contains(&game_id) {
                coll.game_ids.push(game_id);
            }
        }
    }
    pub fn remove_game_from_collection(&mut self, collection: &str, game_id: u32) {
        if let Some(coll) = self.collections.get_mut(collection) {
            coll.game_ids.retain(|&id| id != game_id);
        }
    }
    pub fn collection_count(&self) -> usize {
        self.collections.len()
    }
    pub fn collection_names(&self) -> Vec<&str> {
        self.collections.keys().map(|s| s.as_str()).collect()
    }
}

pub fn toggle_collection_manager(
    keys: Res<ButtonInput<KeyCode>>,
    mut gcm: ResMut<GameCollectionManager>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyC) {
        gcm.visible = !gcm.visible;
        let msg = if gcm.visible {
            format!("收藏管理: {}个收藏", gcm.collection_count())
        } else {
            "收藏管理已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create() {
        let mut gcm = GameCollectionManager::default();
        gcm.create_collection("测试", "测试描述");
        assert_eq!(gcm.collection_count(), 1);
    }
    #[test]
    fn test_add_game() {
        let mut gcm = GameCollectionManager::default();
        gcm.create_collection("test", "");
        gcm.add_game_to_collection("test", 1);
        assert_eq!(gcm.collections["test"].game_ids.len(), 1);
    }
}
