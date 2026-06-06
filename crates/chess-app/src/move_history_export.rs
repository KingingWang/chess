//! Move history export in various formats.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Pgn,
    Text,
    Json,
    Iccs,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Pgn => "pgn",
            Self::Text => "txt",
            Self::Json => "json",
            Self::Iccs => "iccs",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MoveHistoryExport {
    pub format: ExportFormat,
    pub last_export: Option<String>,
}

impl Default for MoveHistoryExport {
    fn default() -> Self {
        Self {
            format: ExportFormat::Pgn,
            last_export: None,
        }
    }
}

impl MoveHistoryExport {
    pub fn export_pgn(&self, moves: &[String]) -> String {
        let mut pgn = "[Event \"Xiangqi Game\"]\n\n".to_string();
        for (i, m) in moves.iter().enumerate() {
            if i % 2 == 0 {
                pgn.push_str(&format!("{}. ", i / 2 + 1));
            }
            pgn.push_str(m);
            pgn.push(' ');
        }
        pgn
    }
    pub fn export_text(&self, moves: &[String]) -> String {
        let mut text = String::new();
        for (i, m) in moves.iter().enumerate() {
            let side = if i % 2 == 0 { "红" } else { "黑" };
            text.push_str(&format!("{}{}: {}\n", i + 1, side, m));
        }
        text
    }
}

pub fn cycle_export_format(
    keys: Res<ButtonInput<KeyCode>>,
    mut mhe: ResMut<MoveHistoryExport>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyX) {
        mhe.format = match mhe.format {
            ExportFormat::Pgn => ExportFormat::Text,
            ExportFormat::Text => ExportFormat::Json,
            ExportFormat::Json => ExportFormat::Iccs,
            ExportFormat::Iccs => ExportFormat::Pgn,
        };
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("导出格式: .{}", mhe.format.extension()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pgn() {
        let mhe = MoveHistoryExport::default();
        let pgn = mhe.export_pgn(&["h2e2".to_string(), "h8g7".to_string()]);
        assert!(pgn.contains("1."));
    }
}
