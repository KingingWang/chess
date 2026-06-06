//! Comments panel for game annotation.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone)]
pub struct CommentsPanel {
    pub visible: bool,
    pub comments: HashMap<usize, String>,
    pub game_comment: String,
}

impl Default for CommentsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            comments: HashMap::new(),
            game_comment: String::new(),
        }
    }
}

impl CommentsPanel {
    pub fn add_comment(&mut self, move_idx: usize, comment: String) {
        self.comments.insert(move_idx, comment);
    }
    pub fn get_comment(&self, move_idx: usize) -> Option<&String> {
        self.comments.get(&move_idx)
    }
    pub fn remove_comment(&mut self, move_idx: usize) {
        self.comments.remove(&move_idx);
    }
    pub fn comment_count(&self) -> usize {
        self.comments.len()
    }
    pub fn clear(&mut self) {
        self.comments.clear();
        self.game_comment.clear();
    }
    pub fn export_annotated(&self, history: &[chess_core::HistoryEntry]) -> String {
        let mut out = String::new();
        if !self.game_comment.is_empty() {
            out.push_str(&format!("{{ {} }}\n", self.game_comment));
        }
        for (i, entry) in history.iter().enumerate() {
            let m = entry.mv();
            let notation = format!(
                "{}{}{}{}",
                (b'a' + m.from.file()) as char,
                m.from.rank() + 1,
                (b'a' + m.to.file()) as char,
                m.to.rank() + 1
            );
            if i % 2 == 0 {
                out.push_str(&format!("{}. ", i / 2 + 1));
            }
            out.push_str(&notation);
            if let Some(comment) = self.comments.get(&i) {
                out.push_str(&format!(" {{ {} }}", comment));
            }
            out.push(' ');
        }
        out.push('\n');
        out
    }
}

pub fn toggle_comments(
    keys: Res<ButtonInput<KeyCode>>,
    mut cp: ResMut<CommentsPanel>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyC) {
        cp.visible = !cp.visible;
        let msg = if cp.visible {
            "注释面板已打开"
        } else {
            "注释面板已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_comment() {
        let mut c = CommentsPanel::default();
        c.add_comment(0, "好棋".to_string());
        assert_eq!(c.comment_count(), 1);
    }
    #[test]
    fn test_remove() {
        let mut c = CommentsPanel::default();
        c.add_comment(0, "test".to_string());
        c.remove_comment(0);
        assert_eq!(c.comment_count(), 0);
    }
    #[test]
    fn test_clear() {
        let mut c = CommentsPanel::default();
        c.add_comment(0, "a".to_string());
        c.game_comment = "b".to_string();
        c.clear();
        assert_eq!(c.comment_count(), 0);
        assert!(c.game_comment.is_empty());
    }
}
