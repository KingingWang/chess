//! Move legality checker with detailed error messages.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct IllegalMoveError {
    pub move_str: String,
    pub reason: String,
    pub suggestions: Vec<String>,
}

#[derive(Resource, Debug, Clone)]
pub struct MoveLegalityChecker {
    pub enabled: bool,
    pub last_error: Option<IllegalMoveError>,
    pub show_suggestions: bool,
}

impl Default for MoveLegalityChecker {
    fn default() -> Self {
        Self {
            enabled: true,
            last_error: None,
            show_suggestions: true,
        }
    }
}

impl MoveLegalityChecker {
    pub fn validate_move(&mut self, move_str: &str, legal_moves: &[String]) -> bool {
        if legal_moves.iter().any(|m| m == move_str) {
            self.last_error = None;
            return true;
        }
        let suggestions: Vec<String> = legal_moves
            .iter()
            .filter(|m| m.starts_with(&move_str[..move_str.len().min(2)]))
            .take(3)
            .cloned()
            .collect();
        let reason = if move_str.len() != 4 {
            "着法格式应为4个字符 (如h2e2)".to_string()
        } else {
            "该着法不合法".to_string()
        };
        self.last_error = Some(IllegalMoveError {
            move_str: move_str.to_string(),
            reason,
            suggestions,
        });
        false
    }
    pub fn error_message(&self) -> Option<String> {
        self.last_error.as_ref().map(|e| {
            let mut msg = e.reason.clone();
            if self.show_suggestions && !e.suggestions.is_empty() {
                msg.push_str(&format!("\n建议: {}", e.suggestions.join(", ")));
            }
            msg
        })
    }
}

pub fn toggle_legality_checker(
    keys: Res<ButtonInput<KeyCode>>,
    mut mlc: ResMut<MoveLegalityChecker>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyL) {
        mlc.enabled = !mlc.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mlc.enabled {
                "合法性检查已开启"
            } else {
                "合法性检查已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_valid() {
        let mut mlc = MoveLegalityChecker::default();
        assert!(mlc.validate_move("h2e2", &["h2e2".to_string()]));
        assert!(mlc.last_error.is_none());
    }
    #[test]
    fn test_invalid() {
        let mut mlc = MoveLegalityChecker::default();
        assert!(!mlc.validate_move("h2e3", &["h2e2".to_string()]));
        assert!(mlc.last_error.is_some());
    }
}
