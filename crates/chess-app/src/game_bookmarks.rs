//! Game bookmarks for saving interesting positions.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub move_idx: usize,
    pub label: String,
    pub fen: String,
    pub timestamp: u64,
}

#[derive(Resource, Debug, Clone)]
pub struct GameBookmarks {
    pub bookmarks: Vec<Bookmark>,
    pub active: bool,
}

impl Default for GameBookmarks {
    fn default() -> Self {
        Self {
            bookmarks: Vec::new(),
            active: false,
        }
    }
}

impl GameBookmarks {
    pub fn add(&mut self, move_idx: usize, label: String, fen: String) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.bookmarks.push(Bookmark {
            move_idx,
            label,
            fen,
            timestamp: ts,
        });
    }
    pub fn remove(&mut self, idx: usize) {
        if idx < self.bookmarks.len() {
            self.bookmarks.remove(idx);
        }
    }
    pub fn count(&self) -> usize {
        self.bookmarks.len()
    }
    pub fn clear(&mut self) {
        self.bookmarks.clear();
    }
    pub fn find_by_label(&self, query: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.label.contains(query))
            .collect()
    }
}

pub fn add_bookmark(
    keys: Res<ButtonInput<KeyCode>>,
    core: Res<crate::app_state::CoreGame>,
    mut bm: ResMut<GameBookmarks>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyD) {
        let idx = core.game.history_len();
        let fen = core.game.board().to_fen();
        bm.add(idx, format!("第{}步", idx + 1), fen);
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("已添加书签 #{}", bm.count()),
        );
    }
}

pub fn toggle_bookmarks(
    keys: Res<ButtonInput<KeyCode>>,
    mut bm: ResMut<GameBookmarks>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyD) {
        bm.active = !bm.active;
        let msg = if bm.active {
            format!("书签面板: {}个书签", bm.count())
        } else {
            "书签面板已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut b = GameBookmarks::default();
        b.add(0, "test".to_string(), "fen".to_string());
        assert_eq!(b.count(), 1);
    }
    #[test]
    fn test_remove() {
        let mut b = GameBookmarks::default();
        b.add(0, "test".to_string(), "fen".to_string());
        b.remove(0);
        assert_eq!(b.count(), 0);
    }
    #[test]
    fn test_find() {
        let mut b = GameBookmarks::default();
        b.add(0, "好棋".to_string(), "fen1".to_string());
        b.add(5, "妙手".to_string(), "fen2".to_string());
        assert_eq!(b.find_by_label("好").len(), 1);
    }
}
