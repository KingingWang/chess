//! PGN import and parsing system.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct PgnGame {
    pub headers: Vec<(String, String)>,
    pub moves: Vec<String>,
    pub result: String,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct GamePgnImport {
    pub last_imported: Option<PgnGame>,
    pub import_errors: Vec<String>,
}

impl GamePgnImport {
    pub fn parse_pgn(&mut self, pgn: &str) -> bool {
        self.import_errors.clear();
        let mut headers = Vec::new();
        let mut moves = Vec::new();
        let mut result = String::new();

        for line in pgn.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                if let Some(end) = line.find(']') {
                    let content = &line[1..end];
                    if let Some(space) = content.find(' ') {
                        headers.push((
                            content[..space].to_string(),
                            content[space + 1..].trim_matches('"').to_string(),
                        ));
                    }
                }
            } else if !line.is_empty() && !line.starts_with('{') {
                for token in line.split_whitespace() {
                    if token == "1-0" || token == "0-1" || token == "1/2-1/2" {
                        result = token.to_string();
                    } else if !token.ends_with('.') && !token.chars().all(|c| c.is_ascii_digit()) {
                        moves.push(token.to_string());
                    }
                }
            }
        }

        if moves.is_empty() {
            self.import_errors.push("未找到着法".to_string());
            return false;
        }

        self.last_imported = Some(PgnGame {
            headers,
            moves,
            result,
        });
        true
    }
}

pub fn import_pgn_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    _pgn: ResMut<GamePgnImport>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyI) {
        crate::toast::spawn_toast(&mut commands, &fonts, "PGN导入: 请粘贴PGN文本");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse() {
        let mut pgn = GamePgnImport::default();
        assert!(pgn.parse_pgn("[Event \"Test\"]\n\n1. h2e2 h8g7 1-0"));
        let game = pgn.last_imported.unwrap();
        assert_eq!(game.moves.len(), 2);
        assert_eq!(game.result, "1-0");
    }
}
