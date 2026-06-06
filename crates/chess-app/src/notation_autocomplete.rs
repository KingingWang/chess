//! Auto-complete for move notation input.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct NotationAutocomplete {
    pub enabled: bool,
    pub suggestions: Vec<String>,
    pub current_input: String,
}

impl Default for NotationAutocomplete {
    fn default() -> Self {
        Self {
            enabled: true,
            suggestions: Vec::new(),
            current_input: String::new(),
        }
    }
}

impl NotationAutocomplete {
    pub fn update_suggestions(&mut self, input: &str, legal_moves: &[chess_core::Move]) {
        self.current_input = input.to_string();
        self.suggestions.clear();
        if input.is_empty() {
            return;
        }
        for m in legal_moves {
            let notation = format!(
                "{}{}{}{}",
                (b'a' + m.from.file()) as char,
                m.from.rank() + 1,
                (b'a' + m.to.file()) as char,
                m.to.rank() + 1
            );
            if notation.starts_with(input) {
                self.suggestions.push(notation);
            }
        }
    }
    pub fn best_suggestion(&self) -> Option<&str> {
        self.suggestions.first().map(|s| s.as_str())
    }
    pub fn clear(&mut self) {
        self.suggestions.clear();
        self.current_input.clear();
    }
}

pub fn toggle_autocomplete(
    keys: Res<ButtonInput<KeyCode>>,
    mut ac: ResMut<NotationAutocomplete>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyU) {
        ac.enabled = !ac.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ac.enabled {
                "自动补全已开启"
            } else {
                "自动补全已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_empty() {
        let mut ac = NotationAutocomplete::default();
        ac.update_suggestions("", &[]);
        assert!(ac.suggestions.is_empty());
    }
    #[test]
    fn test_no_match() {
        let mut ac = NotationAutocomplete::default();
        let m = chess_core::Move {
            from: chess_core::Square::new(0, 0).unwrap(),
            to: chess_core::Square::new(1, 1).unwrap(),
        };
        ac.update_suggestions("zzz", &[m]);
        assert!(ac.suggestions.is_empty());
    }
}
