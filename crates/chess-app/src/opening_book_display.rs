//! Opening book display for current position.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct OpeningBookEntry {
    pub move_str: String,
    pub games: u32,
    pub win_rate: f32,
}

#[derive(Resource, Debug, Clone)]
pub struct OpeningBookDisplay {
    pub enabled: bool,
    pub entries: Vec<OpeningBookEntry>,
    pub max_entries: usize,
}

impl Default for OpeningBookDisplay {
    fn default() -> Self {
        Self {
            enabled: false,
            entries: Vec::new(),
            max_entries: 5,
        }
    }
}

impl OpeningBookDisplay {
    pub fn add_entry(&mut self, entry: OpeningBookEntry) {
        self.entries.push(entry);
        self.entries.sort_by_key(|b| std::cmp::Reverse(b.games));
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }
    }
    pub fn best_move(&self) -> Option<&str> {
        self.entries.first().map(|e| e.move_str.as_str())
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

pub fn toggle_opening_book(
    keys: Res<ButtonInput<KeyCode>>,
    mut ob: ResMut<OpeningBookDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyO) {
        ob.enabled = !ob.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ob.enabled {
                "开局库显示已开启"
            } else {
                "开局库显示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut ob = OpeningBookDisplay::default();
        ob.add_entry(OpeningBookEntry {
            move_str: "h2e2".to_string(),
            games: 100,
            win_rate: 55.0,
        });
        assert_eq!(ob.entries.len(), 1);
    }
    #[test]
    fn test_best() {
        let mut ob = OpeningBookDisplay::default();
        ob.add_entry(OpeningBookEntry {
            move_str: "h2e2".to_string(),
            games: 100,
            win_rate: 55.0,
        });
        assert_eq!(ob.best_move(), Some("h2e2"));
    }
}
