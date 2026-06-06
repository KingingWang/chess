//! Game journal for recording thoughts and annotations during play.
//!
//! Allows players to add text notes to specific moves or positions,
//! creating a personal game diary.

use bevy::prelude::*;
use std::collections::HashMap;

/// Resource managing game journal entries.
#[derive(Resource, Debug, Clone, Default)]
pub struct GameJournal {
    /// Map from move number to journal entry.
    pub entries: HashMap<usize, String>,
    /// General game notes.
    pub game_notes: String,
    /// Whether the journal panel is visible.
    pub visible: bool,
}

impl GameJournal {
    /// Add a note for a specific move.
    pub fn add_move_note(&mut self, move_number: usize, note: String) {
        self.entries.insert(move_number, note);
    }

    /// Get the note for a specific move.
    pub fn get_move_note(&self, move_number: usize) -> Option<&String> {
        self.entries.get(&move_number)
    }

    /// Remove a note for a specific move.
    pub fn remove_move_note(&mut self, move_number: usize) -> Option<String> {
        self.entries.remove(&move_number)
    }

    /// Get total number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.game_notes.clear();
    }

    /// Export journal as text.
    pub fn export_text(&self) -> String {
        let mut output = String::new();
        output.push_str("=== 对局日记 ===\n\n");

        if !self.game_notes.is_empty() {
            output.push_str("对局备注:\n");
            output.push_str(&self.game_notes);
            output.push_str("\n\n");
        }

        let mut moves: Vec<_> = self.entries.iter().collect();
        moves.sort_by_key(|(k, _)| *k);

        for (move_num, note) in moves {
            output.push_str(&format!("第{}步: {}\n", move_num + 1, note));
        }

        output
    }
}

/// Toggle journal visibility.
pub fn toggle_journal(
    keys: Res<ButtonInput<KeyCode>>,
    mut journal: ResMut<GameJournal>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyJ) {
        journal.visible = !journal.visible;
        let msg = if journal.visible {
            "日记面板已打开"
        } else {
            "日记面板已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let j = GameJournal::default();
        assert_eq!(j.entry_count(), 0);
        assert!(!j.visible);
    }

    #[test]
    fn test_add_note() {
        let mut j = GameJournal::default();
        j.add_move_note(0, "好棋!".to_string());
        assert_eq!(j.entry_count(), 1);
        assert_eq!(j.get_move_note(0), Some(&"好棋!".to_string()));
    }

    #[test]
    fn test_remove_note() {
        let mut j = GameJournal::default();
        j.add_move_note(0, "test".to_string());
        let removed = j.remove_move_note(0);
        assert_eq!(removed, Some("test".to_string()));
        assert_eq!(j.entry_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut j = GameJournal::default();
        j.add_move_note(0, "a".to_string());
        j.add_move_note(1, "b".to_string());
        j.game_notes = "general".to_string();
        j.clear();
        assert_eq!(j.entry_count(), 0);
        assert!(j.game_notes.is_empty());
    }

    #[test]
    fn test_export_text() {
        let mut j = GameJournal::default();
        j.game_notes = "这是一场精彩的对局".to_string();
        j.add_move_note(0, "中炮开局".to_string());
        j.add_move_note(5, "关键着法".to_string());
        let text = j.export_text();
        assert!(text.contains("对局日记"));
        assert!(text.contains("中炮开局"));
        assert!(text.contains("关键着法"));
    }
}
