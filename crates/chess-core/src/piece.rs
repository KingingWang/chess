//! Piece and color definitions for Xiangqi (Chinese Chess).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The two sides. Red (红) conventionally moves first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Color {
    Red,
    Black,
}

impl Color {
    #[inline]
    pub const fn opponent(self) -> Color {
        match self {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        }
    }

    /// Forward rank direction for pawns of this color (+1 for Red, -1 for Black).
    #[inline]
    pub const fn forward(self) -> i8 {
        match self {
            Color::Red => 1,
            Color::Black => -1,
        }
    }
}

/// The seven kinds of Xiangqi pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PieceKind {
    /// 将 / 帅 — General (King).
    King,
    /// 士 / 仕 — Advisor (Guard).
    Advisor,
    /// 象 / 相 — Elephant (Minister).
    Elephant,
    /// 马 — Horse (blocked by an adjacent piece: "蹩马腿").
    Horse,
    /// 车 — Chariot (Rook).
    Chariot,
    /// 炮 — Cannon (jumps exactly one screen to capture).
    Cannon,
    /// 兵 / 卒 — Pawn (Soldier).
    Pawn,
}

impl PieceKind {
    /// All kinds in a stable order (useful for iteration / tables).
    pub const ALL: [PieceKind; 7] = [
        PieceKind::King,
        PieceKind::Advisor,
        PieceKind::Elephant,
        PieceKind::Horse,
        PieceKind::Chariot,
        PieceKind::Cannon,
        PieceKind::Pawn,
    ];
}

/// A colored piece occupying a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    #[inline]
    pub const fn new(color: Color, kind: PieceKind) -> Self {
        Piece { color, kind }
    }

    /// FEN character: uppercase = Red, lowercase = Black.
    pub const fn to_fen_char(self) -> char {
        let base = match self.kind {
            PieceKind::King => 'k',
            PieceKind::Advisor => 'a',
            PieceKind::Elephant => 'b',
            PieceKind::Horse => 'n',
            PieceKind::Chariot => 'r',
            PieceKind::Cannon => 'c',
            PieceKind::Pawn => 'p',
        };
        match self.color {
            Color::Red => base.to_ascii_uppercase(),
            Color::Black => base,
        }
    }

    /// Parse a FEN character into a piece, or `None` if unrecognized.
    pub fn from_fen_char(c: char) -> Option<Piece> {
        let color = if c.is_ascii_uppercase() {
            Color::Red
        } else {
            Color::Black
        };
        let kind = match c.to_ascii_lowercase() {
            'k' => PieceKind::King,
            'a' => PieceKind::Advisor,
            'b' | 'e' => PieceKind::Elephant,
            'n' | 'h' => PieceKind::Horse,
            'r' => PieceKind::Chariot,
            'c' => PieceKind::Cannon,
            'p' => PieceKind::Pawn,
            _ => return None,
        };
        Some(Piece::new(color, kind))
    }

    /// Localized display glyph (traditional Chinese characters).
    pub const fn glyph(self) -> char {
        match (self.color, self.kind) {
            (Color::Red, PieceKind::King) => '帅',
            (Color::Red, PieceKind::Advisor) => '仕',
            (Color::Red, PieceKind::Elephant) => '相',
            (Color::Red, PieceKind::Horse) => '马',
            (Color::Red, PieceKind::Chariot) => '车',
            (Color::Red, PieceKind::Cannon) => '炮',
            (Color::Red, PieceKind::Pawn) => '兵',
            (Color::Black, PieceKind::King) => '将',
            (Color::Black, PieceKind::Advisor) => '士',
            (Color::Black, PieceKind::Elephant) => '象',
            (Color::Black, PieceKind::Horse) => '馬',
            (Color::Black, PieceKind::Chariot) => '車',
            (Color::Black, PieceKind::Cannon) => '砲',
            (Color::Black, PieceKind::Pawn) => '卒',
        }
    }
}
