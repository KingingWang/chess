//! Opening name detection for Xiangqi (Chinese Chess) openings.
//!
//! Recognises common opening sequences and displays the opening name
//! as a toast notification when detected.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};

/// Resource tracking the current detected opening.
#[derive(Resource, Debug, Clone)]
pub struct OpeningName {
    /// Current opening name (Chinese).
    pub name_cn: String,
    /// Current opening name (English).
    pub name_en: String,
    /// Number of moves matched in the opening line.
    pub moves_matched: usize,
    /// Whether we've already shown a toast for this opening.
    pub toast_shown: bool,
}

impl Default for OpeningName {
    fn default() -> Self {
        Self {
            name_cn: String::new(),
            name_en: String::new(),
            moves_matched: 0,
            toast_shown: false,
        }
    }
}

impl OpeningName {
    /// Detect the opening from the move history.
    pub fn detect(&mut self, moves: &[String]) {
        self.name_cn.clear();
        self.name_en.clear();
        self.moves_matched = 0;

        let mut best_match: Option<(&str, &str, usize)> = None;

        for (cn, en, line) in OPENING_LINES.iter() {
            let matched = moves
                .iter()
                .zip(line.iter())
                .take_while(|(a, b)| a == b)
                .count();
            if matched > 0 && matched > best_match.map_or(0, |(_, _, m)| m) {
                best_match = Some((cn, en, matched));
            }
        }

        if let Some((cn, en, matched)) = best_match {
            self.name_cn = cn.to_string();
            self.name_en = en.to_string();
            self.moves_matched = matched;
        }
    }

    /// Get a display string.
    pub fn display(&self) -> String {
        if self.name_cn.is_empty() {
            String::new()
        } else {
            format!("{} ({})", self.name_cn, self.name_en)
        }
    }

    /// Reset the opening detection.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Common Xiangqi opening lines as (Chinese name, English name, move sequence).
///
/// Moves are in ICCS notation (e.g., "h2e2" = cannon from h2 to e2).
static OPENING_LINES: &[(&str, &str, &[&str])] = &[
    (
        "中炮对屏风马",
        "Central Cannon vs Screen Horse",
        &["h2e2", "h8g7", "h0g2", "i9h9", "b0a2"],
    ),
    (
        "中炮对反宫马",
        "Central Cannon vs Reverse Palace Horse",
        &["h2e2", "b7c7", "h0g2", "b9c7", "b0a2"],
    ),
    (
        "中炮对单提马",
        "Central Cannon vs Single Horse",
        &["h2e2", "h8g7", "h0g2", "i9h9", "b0a2", "g6h5"],
    ),
    (
        "中炮对左炮封车",
        "Central Cannon vs Left Cannon Block",
        &["h2e2", "h8g7", "h0g2", "i9h9", "i0h0", "b7c4"],
    ),
    ("飞相局", "Flying Elephant Opening", &["c0e2"]),
    (
        "飞相对左中炮",
        "Flying Elephant vs Left Central Cannon",
        &["c0e2", "h8e8"],
    ),
    (
        "飞相对右中炮",
        "Flying Elephant vs Right Central Cannon",
        &["c0e2", "b8e8"],
    ),
    ("仕角炮", "Palace Corner Cannon", &["h2f2"]),
    ("过宫炮", "Cross Palace Cannon", &["h2a2"]),
    ("仙人指路", "Pawn Opening (Immortal's Guide)", &["c3c4"]),
    (
        "仙人指路对卒底炮",
        "Pawn Opening vs Bottom Cannon",
        &["c3c4", "b7c7"],
    ),
    (
        "仙人指路对飞象",
        "Pawn Opening vs Flying Elephant",
        &["c3c4", "g9e7"],
    ),
    ("起马局", "Horse Opening", &["b0c2"]),
    (
        "起马对进卒",
        "Horse Opening vs Advancing Pawn",
        &["b0c2", "c6c5"],
    ),
    (
        "鸳鸯炮",
        "Mandarin Duck Cannons",
        &["h2e2", "h8g7", "h0g2", "i9h9", "i0h0", "b7c4"],
    ),
];

/// System to detect and track opening names.
pub fn detect_opening(
    core: Res<CoreGame>,
    mut opening: ResMut<OpeningName>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let moves: Vec<String> = core
        .game
        .history()
        .iter()
        .map(|m| {
            format!(
                "{}{}",
                (b'a' + m.mv().from.file()) as char,
                m.mv().from.rank() + 1
            )
        })
        .collect();

    let prev_matched = opening.moves_matched;
    opening.detect(&moves);

    if opening.moves_matched > 0 && opening.moves_matched > prev_matched && !opening.toast_shown {
        if opening.moves_matched >= 2 {
            let msg = format!("开局: {}", opening.display());
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
            opening.toast_shown = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_central_cannon() {
        let mut opening = OpeningName::default();
        opening.detect(&["h2e2".to_string()]);
        assert!(!opening.name_cn.is_empty());
        assert!(opening.name_cn.contains("中炮") || opening.name_cn.contains("炮"));
        assert_eq!(opening.moves_matched, 1);
    }

    #[test]
    fn test_detect_flying_elephant() {
        let mut opening = OpeningName::default();
        opening.detect(&["c0e2".to_string()]);
        assert_eq!(opening.name_cn, "飞相局");
        assert_eq!(opening.moves_matched, 1);
    }

    #[test]
    fn test_detect_pawn_opening() {
        let mut opening = OpeningName::default();
        opening.detect(&["c3c4".to_string()]);
        assert!(opening.name_cn.contains("仙人"));
    }

    #[test]
    fn test_detect_longer_line() {
        let mut opening = OpeningName::default();
        opening.detect(&["c0e2".to_string(), "h8e8".to_string()]);
        assert_eq!(opening.name_cn, "飞相对左中炮");
        assert_eq!(opening.moves_matched, 2);
    }

    #[test]
    fn test_no_match() {
        let mut opening = OpeningName::default();
        opening.detect(&["z9z9".to_string()]);
        assert!(opening.name_cn.is_empty());
        assert_eq!(opening.moves_matched, 0);
    }

    #[test]
    fn test_empty_history() {
        let mut opening = OpeningName::default();
        opening.detect(&[]);
        assert!(opening.name_cn.is_empty());
    }

    #[test]
    fn test_display() {
        let mut opening = OpeningName::default();
        opening.detect(&["c0e2".to_string()]);
        let display = opening.display();
        assert!(display.contains("飞相"));
    }

    #[test]
    fn test_display_empty() {
        let opening = OpeningName::default();
        assert_eq!(opening.display(), "");
    }

    #[test]
    fn test_reset() {
        let mut opening = OpeningName::default();
        opening.detect(&["c0e2".to_string()]);
        assert!(!opening.name_cn.is_empty());
        opening.reset();
        assert!(opening.name_cn.is_empty());
        assert_eq!(opening.moves_matched, 0);
        assert!(!opening.toast_shown);
    }

    #[test]
    fn test_opening_lines_data() {
        for (cn, en, line) in OPENING_LINES.iter() {
            assert!(!cn.is_empty(), "Empty Chinese name");
            assert!(!en.is_empty(), "Empty English name");
            assert!(!line.is_empty(), "Empty line for {} / {}", cn, en);
        }
    }

    #[test]
    fn test_longest_match_wins() {
        let mut opening = OpeningName::default();
        opening.detect(&["c0e2".to_string(), "h8e8".to_string()]);
        assert_eq!(opening.name_cn, "飞相对左中炮");
        assert_eq!(opening.moves_matched, 2);
    }
}
