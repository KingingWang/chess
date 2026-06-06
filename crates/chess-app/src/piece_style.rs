//! Piece style selection and customization.
//!
//! Allows users to choose from different piece visual styles:
//! traditional Chinese characters, simplified characters, or
//! international symbols.

use bevy::prelude::*;

/// Available piece display styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceStyle {
    /// Traditional Chinese characters (車馬炮).
    Traditional,
    /// Simplified Chinese characters (车马炮).
    Simplified,
    /// Single-character abbreviations.
    Abbreviated,
    /// International symbols with letters.
    International,
}

impl PieceStyle {
    /// Get the character for a piece.
    ///
    /// `piece` is one of: K=King(帅/将), A=Advisor(仕/士), E=Elephant(相/象),
    /// H=Horse(馬/马), R=Rook(車/车), C=Cannon(炮), P=Pawn(兵/卒).
    /// `is_red` determines whether to show the red or black character.
    pub fn piece_char(&self, piece: char, is_red: bool) -> String {
        match self {
            Self::Traditional => Self::traditional_char(piece, is_red).to_string(),
            Self::Simplified => Self::simplified_char(piece, is_red).to_string(),
            Self::Abbreviated => Self::abbreviated_char(piece, is_red).to_string(),
            Self::International => Self::international_char(piece, is_red),
        }
    }

    fn traditional_char(piece: char, is_red: bool) -> char {
        match (piece, is_red) {
            ('K', true) => '帅',
            ('K', false) => '将',
            ('A', true) => '仕',
            ('A', false) => '士',
            ('E', true) => '相',
            ('E', false) => '象',
            ('H', true) => '馬',
            ('H', false) => '馬',
            ('R', true) => '車',
            ('R', false) => '車',
            ('C', true) => '炮',
            ('C', false) => '砲',
            ('P', true) => '兵',
            ('P', false) => '卒',
            _ => '?',
        }
    }

    fn simplified_char(piece: char, is_red: bool) -> char {
        match (piece, is_red) {
            ('K', true) => '帅',
            ('K', false) => '将',
            ('A', true) => '仕',
            ('A', false) => '士',
            ('E', true) => '相',
            ('E', false) => '象',
            ('H', true) => '马',
            ('H', false) => '马',
            ('R', true) => '车',
            ('R', false) => '车',
            ('C', true) => '炮',
            ('C', false) => '炮',
            ('P', true) => '兵',
            ('P', false) => '卒',
            _ => '?',
        }
    }

    fn abbreviated_char(piece: char, is_red: bool) -> char {
        match (piece, is_red) {
            ('K', _) => 'K',
            ('A', _) => 'A',
            ('E', _) => 'E',
            ('H', _) => 'H',
            ('R', _) => 'R',
            ('C', _) => 'C',
            ('P', _) => 'P',
            _ => '?',
        }
    }

    fn international_char(piece: char, is_red: bool) -> String {
        let base = match piece {
            'K' => "K",
            'A' => "A",
            'E' => "E",
            'H' => "H",
            'R' => "R",
            'C' => "C",
            'P' => "P",
            _ => "?",
        };
        let suffix = if is_red { "r" } else { "b" };
        format!("{}{}", base, suffix)
    }

    /// Get the Chinese label for this style.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Traditional => "繁体",
            Self::Simplified => "简体",
            Self::Abbreviated => "缩写",
            Self::International => "国际",
        }
    }

    /// Get the next style.
    pub fn next(&self) -> Self {
        match self {
            Self::Traditional => Self::Simplified,
            Self::Simplified => Self::Abbreviated,
            Self::Abbreviated => Self::International,
            Self::International => Self::Traditional,
        }
    }
}

impl Default for PieceStyle {
    fn default() -> Self {
        Self::Simplified
    }
}

/// Resource managing piece style selection.
#[derive(Resource, Debug, Clone)]
pub struct PieceStyleResource {
    /// Current piece style.
    pub style: PieceStyle,
    /// Font size for piece characters.
    pub font_size: f32,
}

impl Default for PieceStyleResource {
    fn default() -> Self {
        Self {
            style: PieceStyle::default(),
            font_size: 28.0,
        }
    }
}

/// Toggle piece style with keyboard shortcut.
pub fn toggle_piece_style(
    keys: Res<ButtonInput<KeyCode>>,
    mut piece_style: ResMut<PieceStyleResource>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyP) {
        piece_style.style = piece_style.style.next();
        dirty.0 = true;
        let msg = format!("棋子样式: {}", piece_style.style.label_cn());
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traditional_chars() {
        let style = PieceStyle::Traditional;
        assert_eq!(style.piece_char('K', true), "帅");
        assert_eq!(style.piece_char('K', false), "将");
        assert_eq!(style.piece_char('R', true), "車");
        assert_eq!(style.piece_char('H', true), "馬");
    }

    #[test]
    fn test_simplified_chars() {
        let style = PieceStyle::Simplified;
        assert_eq!(style.piece_char('K', true), "帅");
        assert_eq!(style.piece_char('R', true), "车");
        assert_eq!(style.piece_char('H', true), "马");
    }

    #[test]
    fn test_abbreviated_chars() {
        let style = PieceStyle::Abbreviated;
        assert_eq!(style.piece_char('K', true), "K");
        assert_eq!(style.piece_char('R', false), "R");
    }

    #[test]
    fn test_international_chars() {
        let style = PieceStyle::International;
        assert_eq!(style.piece_char('K', true), "Kr");
        assert_eq!(style.piece_char('K', false), "Kb");
    }

    #[test]
    fn test_style_cycle() {
        let mut style = PieceStyle::Traditional;
        style = style.next();
        assert_eq!(style, PieceStyle::Simplified);
        style = style.next();
        assert_eq!(style, PieceStyle::Abbreviated);
        style = style.next();
        assert_eq!(style, PieceStyle::International);
        style = style.next();
        assert_eq!(style, PieceStyle::Traditional);
    }

    #[test]
    fn test_labels() {
        assert_eq!(PieceStyle::Traditional.label_cn(), "繁体");
        assert_eq!(PieceStyle::Simplified.label_cn(), "简体");
        assert_eq!(PieceStyle::Abbreviated.label_cn(), "缩写");
        assert_eq!(PieceStyle::International.label_cn(), "国际");
    }

    #[test]
    fn test_default() {
        let style = PieceStyle::default();
        assert_eq!(style, PieceStyle::Simplified);
    }

    #[test]
    fn test_resource_default() {
        let res = PieceStyleResource::default();
        assert_eq!(res.style, PieceStyle::Simplified);
        assert!(res.font_size > 0.0);
    }
}
