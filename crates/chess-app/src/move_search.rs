//! Move search for finding specific moves in game history.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct MoveSearch {
    pub active: bool,
    pub query: String,
    pub results: Vec<usize>,
    pub current_result: usize,
}

impl MoveSearch {
    pub fn search(&mut self, query: &str, history: &[chess_core::HistoryEntry]) {
        self.query = query.to_string();
        self.results.clear();
        self.current_result = 0;
        if query.is_empty() {
            return;
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
            if notation.contains(query) {
                self.results.push(i);
            }
        }
    }

    pub fn next_result(&mut self) -> Option<usize> {
        if self.results.is_empty() {
            return None;
        }
        self.current_result = (self.current_result + 1) % self.results.len();
        Some(self.results[self.current_result])
    }

    pub fn prev_result(&mut self) -> Option<usize> {
        if self.results.is_empty() {
            return None;
        }
        if self.current_result == 0 {
            self.current_result = self.results.len() - 1;
        } else {
            self.current_result -= 1;
        }
        Some(self.results[self.current_result])
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

pub fn toggle_move_search(
    keys: Res<ButtonInput<KeyCode>>,
    mut search: ResMut<MoveSearch>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyF) {
        search.active = !search.active;
        let msg = if search.active {
            "搜索模式 (输入着法如 h2e2)"
        } else {
            "搜索已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let s = MoveSearch::default();
        assert!(!s.active);
    }
    #[test]
    fn test_empty_search() {
        let mut s = MoveSearch::default();
        s.search("", &[]);
        assert_eq!(s.result_count(), 0);
    }
}
