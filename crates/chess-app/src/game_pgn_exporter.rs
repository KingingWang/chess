//! PGN export with full game information.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct GamePgnExporter {
    pub include_comments: bool,
    pub include_evaluations: bool,
    pub include_variations: bool,
}

impl Default for GamePgnExporter {
    fn default() -> Self {
        Self {
            include_comments: true,
            include_evaluations: false,
            include_variations: false,
        }
    }
}

impl GamePgnExporter {
    pub fn export(
        &self,
        headers: &[(&str, &str)],
        moves: &[String],
        comments: &[Option<String>],
    ) -> String {
        let mut pgn = String::new();
        for (key, value) in headers {
            pgn.push_str(&format!("[{} \"{}\"]\n", key, value));
        }
        pgn.push('\n');
        for (i, m) in moves.iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}. ", i / 2 + 1));
            }
            pgn.push_str(m);
            if self.include_comments {
                if let Some(comment) = comments.get(i).and_then(|c| c.as_ref()) {
                    pgn.push_str(&format!(" {{ {} }}", comment));
                }
            }
            pgn.push(' ');
        }
        pgn
    }
}

pub fn export_pgn_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    exporter: Res<GamePgnExporter>,
    core: Res<crate::app_state::CoreGame>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyP) {
        let headers = vec![
            ("Event", "Xiangqi Game"),
            ("Site", "Local"),
            ("Date", "2026.06.06"),
        ];
        let moves: Vec<String> = core
            .game
            .history()
            .iter()
            .map(|e| {
                let m = e.mv();
                format!(
                    "{}{}{}{}",
                    (b'a' + m.from.file()) as char,
                    m.from.rank() + 1,
                    (b'a' + m.to.file()) as char,
                    m.to.rank() + 1
                )
            })
            .collect();
        let pgn = exporter.export(&headers, &moves, &vec![None; moves.len()]);
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("PGN已导出: {}步", moves.len()),
        );
        let _ = std::fs::write("game.pgn", pgn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_export() {
        let exporter = GamePgnExporter::default();
        let pgn = exporter.export(
            &[("Event", "Test")],
            &["h2e2".to_string(), "h8g7".to_string()],
            &[None, None],
        );
        assert!(pgn.contains("[Event"));
        assert!(pgn.contains("1. h2e2"));
    }
}
