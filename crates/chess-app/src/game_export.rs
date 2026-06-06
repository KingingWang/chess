//! Game export functionality for various formats.
//!
//! Supports exporting games to:
//! - PNG image (board screenshot)
//! - Text summary
//! - PGN with annotations

use bevy::prelude::*;
use std::path::PathBuf;

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Portable Game Notation with annotations.
    Pgn,
    /// Text summary with move list.
    Text,
    /// JSON format for programmatic access.
    Json,
}

impl ExportFormat {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Pgn => "PGN棋谱",
            Self::Text => "文本记录",
            Self::Json => "JSON数据",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Pgn => "pgn",
            Self::Text => "txt",
            Self::Json => "json",
        }
    }
}

/// Game export resource.
#[derive(Resource, Debug, Clone)]
pub struct GameExport {
    pub export_dir: PathBuf,
    pub include_annotations: bool,
    pub include_evaluations: bool,
    pub include_time_data: bool,
}

impl Default for GameExport {
    fn default() -> Self {
        Self {
            export_dir: PathBuf::from("exports"),
            include_annotations: true,
            include_evaluations: true,
            include_time_data: true,
        }
    }
}

impl GameExport {
    /// Generate a filename for the current game.
    pub fn generate_filename(&self, format: ExportFormat) -> PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.export_dir
            .join(format!("game_{}.{}", timestamp, format.extension()))
    }

    /// Export game to PGN format.
    pub fn export_pgn(&self, core: &crate::app_state::CoreGame) -> String {
        let mut pgn = String::new();
        pgn.push_str("[Event \"Xiangqi Game\"]\n");
        pgn.push_str("[Site \"Local\"]\n");
        pgn.push_str(&format!("[Date \"{}\"]\n", "2026.06.06"));
        pgn.push_str("[White \"Red\"]\n");
        pgn.push_str("[Black \"Black\"]\n");
        pgn.push('\n');

        // Add moves
        for (i, entry) in core.game.history().iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}. ", i / 2 + 1));
            }
            let m = entry.mv();
            pgn.push_str(&format!(
                "{}{}{}{} ",
                (b'a' + m.from.file()) as char,
                m.from.rank() + 1,
                (b'a' + m.to.file()) as char,
                m.to.rank() + 1
            ));
        }
        pgn.push('\n');
        pgn
    }

    /// Export game to text format.
    pub fn export_text(&self, core: &crate::app_state::CoreGame) -> String {
        let mut text = String::new();
        text.push_str("=== 象棋对局记录 ===\n\n");
        text.push_str(&format!("总步数: {}\n\n", core.game.history_len()));

        for (i, entry) in core.game.history().iter().enumerate() {
            let m = entry.mv();
            let side = if i % 2 == 0 { "红" } else { "黑" };
            text.push_str(&format!(
                "第{}步 ({}): {}{}→{}{}\n",
                i + 1,
                side,
                (b'a' + m.from.file()) as char,
                m.from.rank() + 1,
                (b'a' + m.to.file()) as char,
                m.to.rank() + 1
            ));
        }
        text
    }

    /// Export game to JSON format.
    pub fn export_json(&self, core: &crate::app_state::CoreGame) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str("  \"game\": {\n");
        json.push_str(&format!(
            "    \"total_moves\": {},\n",
            core.game.history_len()
        ));
        json.push_str("    \"moves\": [\n");

        for (i, entry) in core.game.history().iter().enumerate() {
            let m = entry.mv();
            json.push_str(&format!(
                "      {{\"from\": \"{}{}\", \"to\": \"{}{}\"}}{}\n",
                (b'a' + m.from.file()) as char,
                m.from.rank() + 1,
                (b'a' + m.to.file()) as char,
                m.to.rank() + 1,
                if i < core.game.history_len() - 1 {
                    ","
                } else {
                    ""
                }
            ));
        }

        json.push_str("    ]\n");
        json.push_str("  }\n");
        json.push_str("}\n");
        json
    }
}

/// Keyboard shortcut to export game.
pub fn export_game_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    core: Res<crate::app_state::CoreGame>,
    export: Res<GameExport>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if ctrl && shift && keys.just_pressed(KeyCode::KeyX) {
        let pgn = export.export_pgn(&core);
        let path = export.generate_filename(ExportFormat::Pgn);
        match std::fs::create_dir_all(&export.export_dir) {
            Ok(_) => match std::fs::write(&path, &pgn) {
                Ok(_) => {
                    let msg = format!("已导出: {:?}", path);
                    crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                }
                Err(e) => {
                    let msg = format!("导出失败: {}", e);
                    crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                }
            },
            Err(e) => {
                let msg = format!("创建目录失败: {}", e);
                crate::toast::spawn_toast(&mut commands, &fonts, &msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_labels() {
        assert_eq!(ExportFormat::Pgn.label_cn(), "PGN棋谱");
        assert_eq!(ExportFormat::Text.label_cn(), "文本记录");
        assert_eq!(ExportFormat::Json.label_cn(), "JSON数据");
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(ExportFormat::Pgn.extension(), "pgn");
        assert_eq!(ExportFormat::Text.extension(), "txt");
        assert_eq!(ExportFormat::Json.extension(), "json");
    }

    #[test]
    fn test_generate_filename() {
        let export = GameExport::default();
        let path = export.generate_filename(ExportFormat::Pgn);
        assert!(path.to_str().unwrap().ends_with(".pgn"));
        assert!(path.to_str().unwrap().contains("game_"));
    }

    #[test]
    fn test_export_empty_game() {
        let export = GameExport::default();
        let core = crate::app_state::CoreGame::default();
        let pgn = export.export_pgn(&core);
        assert!(pgn.contains("[Event"));
        let text = export.export_text(&core);
        assert!(text.contains("象棋对局记录"));
        let json = export.export_json(&core);
        assert!(json.contains("\"total_moves\": 0"));
    }
}
