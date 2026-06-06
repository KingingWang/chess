//! Opening book management and usage.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BookEntry {
    pub move_str: String,
    pub weight: u32,
    pub games: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct OpeningBookManager {
    pub enabled: bool,
    pub book: HashMap<String, Vec<BookEntry>>,
    pub use_random: bool,
}

impl Default for OpeningBookManager {
    fn default() -> Self {
        let mut book = HashMap::new();
        book.insert(
            "start".to_string(),
            vec![
                BookEntry {
                    move_str: "h2e2".to_string(),
                    weight: 100,
                    games: 50000,
                },
                BookEntry {
                    move_str: "c3c4".to_string(),
                    weight: 80,
                    games: 30000,
                },
                BookEntry {
                    move_str: "b0c2".to_string(),
                    weight: 60,
                    games: 20000,
                },
            ],
        );
        Self {
            enabled: true,
            book,
            use_random: true,
        }
    }
}

impl OpeningBookManager {
    pub fn lookup(&self, position: &str) -> Option<&Vec<BookEntry>> {
        self.book.get(position)
    }
    pub fn select_move(&self, position: &str) -> Option<&str> {
        self.lookup(position).and_then(|entries| {
            entries
                .iter()
                .max_by_key(|e| e.weight)
                .map(|e| e.move_str.as_str())
        })
    }
    pub fn add_entry(&mut self, position: &str, move_str: &str, weight: u32) {
        self.book
            .entry(position.to_string())
            .or_default()
            .push(BookEntry {
                move_str: move_str.to_string(),
                weight,
                games: 0,
            });
    }
}

pub fn toggle_book(
    keys: Res<ButtonInput<KeyCode>>,
    mut obm: ResMut<OpeningBookManager>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyL) {
        obm.enabled = !obm.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if obm.enabled {
                "开局库已启用"
            } else {
                "开局库已禁用"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lookup() {
        let obm = OpeningBookManager::default();
        assert!(obm.lookup("start").is_some());
    }
    #[test]
    fn test_select() {
        let obm = OpeningBookManager::default();
        assert!(obm.select_move("start").is_some());
    }
}
