//! Board coordinate display style customization.
//!
//! Allows users to choose between different coordinate display styles:
//! - Western (a-i, 0-9)
//! - Chinese numeric (一-九, 0-9)
//! - Traditional Chinese (一-九, 〇-九)

use bevy::prelude::*;

use crate::app_state::UiFonts;

/// Coordinate display styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordStyle {
    /// Western algebraic: files a-i, ranks 0-9.
    #[default]
    Western,
    /// Chinese numeric: files 一九 (right to left for Red), ranks 0-9.
    ChineseNumeric,
    /// No coordinates shown.
    None,
}

impl CoordStyle {
    /// Get the Chinese label.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Western => "西式坐标",
            Self::ChineseNumeric => "中文数字",
            Self::None => "无坐标",
        }
    }

    /// Get the next style.
    pub fn next(&self) -> Self {
        match self {
            Self::Western => Self::ChineseNumeric,
            Self::ChineseNumeric => Self::None,
            Self::None => Self::Western,
        }
    }
}

/// Chinese numeral characters.
const CHINESE_NUMERALS: &[char] = &['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];

/// Resource managing coordinate display style.
#[derive(Resource, Debug, Clone)]
pub struct CoordStyleResource {
    /// File (column) label style.
    pub file_style: CoordStyle,
    /// Rank (row) label style.
    pub rank_style: CoordStyle,
    /// Whether to show coordinates at all.
    pub visible: bool,
}

impl Default for CoordStyleResource {
    fn default() -> Self {
        Self {
            file_style: CoordStyle::Western,
            rank_style: CoordStyle::Western,
            visible: true,
        }
    }
}

impl CoordStyleResource {
    /// Get the file label for a given file index (0-8).
    pub fn file_label(&self, file: u8) -> String {
        match self.file_style {
            CoordStyle::Western => format!("{}", (b'a' + file) as char),
            CoordStyle::ChineseNumeric => {
                // Chinese convention: files are numbered 9-1 from left to right (Red's view)
                let idx = (8 - file + 1) as usize;
                if idx < CHINESE_NUMERALS.len() {
                    CHINESE_NUMERALS[idx].to_string()
                } else {
                    "?".to_string()
                }
            }
            CoordStyle::None => String::new(),
        }
    }

    /// Get the rank label for a given rank index (0-9).
    pub fn rank_label(&self, rank: u8) -> String {
        match self.rank_style {
            CoordStyle::Western => format!("{}", rank),
            CoordStyle::ChineseNumeric => {
                if (rank as usize) < CHINESE_NUMERALS.len() {
                    CHINESE_NUMERALS[rank as usize].to_string()
                } else {
                    "?".to_string()
                }
            }
            CoordStyle::None => String::new(),
        }
    }

    /// Format a square as a coordinate string.
    pub fn format_square(&self, file: u8, rank: u8) -> String {
        if !self.visible {
            return String::new();
        }
        format!("{}{}", self.file_label(file), self.rank_label(rank))
    }
}

/// Toggle coordinate style with keyboard shortcut.
pub fn toggle_coord_style(
    keys: Res<ButtonInput<KeyCode>>,
    mut coord_style: ResMut<CoordStyleResource>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyL) {
        coord_style.file_style = coord_style.file_style.next();
        coord_style.rank_style = coord_style.file_style; // Keep them in sync
        dirty.0 = true;

        let msg = if coord_style.file_style == CoordStyle::None {
            "坐标已隐藏".to_string()
        } else {
            format!("坐标样式: {}", coord_style.file_style.label_cn())
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_western_files() {
        let style = CoordStyleResource::default();
        assert_eq!(style.file_label(0), "a");
        assert_eq!(style.file_label(8), "i");
    }

    #[test]
    fn test_western_ranks() {
        let style = CoordStyleResource::default();
        assert_eq!(style.rank_label(0), "0");
        assert_eq!(style.rank_label(9), "9");
    }

    #[test]
    fn test_chinese_files() {
        let mut style = CoordStyleResource::default();
        style.file_style = CoordStyle::ChineseNumeric;
        // File 0 = 九 (leftmost from Red's view, which is 9th file)
        assert_eq!(style.file_label(0), "九");
        // File 8 = 一
        assert_eq!(style.file_label(8), "一");
    }

    #[test]
    fn test_chinese_ranks() {
        let mut style = CoordStyleResource::default();
        style.rank_style = CoordStyle::ChineseNumeric;
        assert_eq!(style.rank_label(0), "〇");
        assert_eq!(style.rank_label(1), "一");
        assert_eq!(style.rank_label(9), "九");
    }

    #[test]
    fn test_format_square() {
        let style = CoordStyleResource::default();
        assert_eq!(style.format_square(7, 2), "h2");
        assert_eq!(style.format_square(0, 0), "a0");
    }

    #[test]
    fn test_format_square_hidden() {
        let mut style = CoordStyleResource::default();
        style.visible = false;
        assert_eq!(style.format_square(7, 2), "");
    }

    #[test]
    fn test_style_cycle() {
        let mut style = CoordStyle::Western;
        style = style.next();
        assert_eq!(style, CoordStyle::ChineseNumeric);
        style = style.next();
        assert_eq!(style, CoordStyle::None);
        style = style.next();
        assert_eq!(style, CoordStyle::Western);
    }

    #[test]
    fn test_labels() {
        assert_eq!(CoordStyle::Western.label_cn(), "西式坐标");
        assert_eq!(CoordStyle::ChineseNumeric.label_cn(), "中文数字");
        assert_eq!(CoordStyle::None.label_cn(), "无坐标");
    }
}
