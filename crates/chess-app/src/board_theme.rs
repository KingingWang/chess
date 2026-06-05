//! Board colour theme system.
//!
//! Stores the active colour palette in a [`BoardTheme`] resource. The
//! `setup_board` and `redraw_pieces` systems read from it instead of
//! hard-coded `const`s. The player can cycle through themes with the `T` key
//! (handled in [`keyboard.rs`]).
//!
//! Five built-in themes:
//! - **Classic** — warm aged-paper wood tones (the original palette).
//! - **Dark** — dark tournament green/brown.
//! - **Paper** — light cream minimalist.
//! - **Rosewood** — rich reddish-brown.
//! - **Jade** — cool blue-green inspired by Chinese jade.

use bevy::prelude::*;

/// All board-rendering colours packaged into one struct.
#[derive(Debug, Clone)]
pub struct Palette {
    /// Outer lacquer frame.
    pub frame_dark: Color,
    /// Inner gold-brown rim.
    pub frame_edge: Color,
    /// Main board surface.
    pub board_bg: Color,
    /// Grid lines.
    pub line_color: Color,
    /// River text / decorative labels.
    pub river_color: Color,
    /// Piece disc background.
    pub disc_face: Color,
    /// Red side ink.
    pub red_ink: Color,
    /// Black side ink.
    pub black_ink: Color,
    /// Piece disc border ring.
    pub disc_border: Color,
}

/// Index for the active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThemeId {
    #[default]
    Classic,
    Dark,
    Paper,
    Rosewood,
    Jade,
}

impl ThemeId {
    /// Cycle to the next theme.
    pub fn next(self) -> Self {
        match self {
            ThemeId::Classic => ThemeId::Dark,
            ThemeId::Dark => ThemeId::Paper,
            ThemeId::Paper => ThemeId::Rosewood,
            ThemeId::Rosewood => ThemeId::Jade,
            ThemeId::Jade => ThemeId::Classic,
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Classic => "经典",
            ThemeId::Dark => "暗色",
            ThemeId::Paper => "素纸",
            ThemeId::Rosewood => "红木",
            ThemeId::Jade => "翡翠",
        }
    }
    /// Emoji icon for this theme.
    pub fn emoji(self) -> &'static str {
        match self {
            ThemeId::Classic => "📜",
            ThemeId::Dark => "🌙",
            ThemeId::Paper => "📄",
            ThemeId::Rosewood => "🪵",
            ThemeId::Jade => "🟢",
        }
    }
}

impl ThemeId {
    pub fn palette(self) -> Palette {
        match self {
            ThemeId::Classic => Palette {
                frame_dark: Color::srgb(0.28, 0.16, 0.09),
                frame_edge: Color::srgb(0.45, 0.29, 0.16),
                board_bg: Color::srgb(0.90, 0.79, 0.57),
                line_color: Color::srgb(0.30, 0.19, 0.10),
                river_color: Color::srgba(0.30, 0.19, 0.10, 0.55),
                disc_face: Color::srgb(0.97, 0.93, 0.83),
                red_ink: Color::srgb(0.72, 0.11, 0.11),
                black_ink: Color::srgb(0.12, 0.12, 0.14),
                disc_border: Color::srgba(0.0, 0.0, 0.0, 0.15),
            },
            ThemeId::Dark => Palette {
                frame_dark: Color::srgb(0.10, 0.12, 0.08),
                frame_edge: Color::srgb(0.22, 0.28, 0.18),
                board_bg: Color::srgb(0.22, 0.32, 0.18),
                line_color: Color::srgb(0.70, 0.65, 0.55),
                river_color: Color::srgba(0.70, 0.65, 0.55, 0.55),
                disc_face: Color::srgb(0.92, 0.88, 0.78),
                red_ink: Color::srgb(0.85, 0.15, 0.10),
                black_ink: Color::srgb(0.10, 0.10, 0.12),
                disc_border: Color::srgba(0.80, 0.75, 0.60, 0.20),
            },
            ThemeId::Paper => Palette {
                frame_dark: Color::srgb(0.65, 0.60, 0.52),
                frame_edge: Color::srgb(0.78, 0.73, 0.65),
                board_bg: Color::srgb(0.96, 0.94, 0.90),
                line_color: Color::srgb(0.40, 0.38, 0.35),
                river_color: Color::srgba(0.40, 0.38, 0.35, 0.50),
                disc_face: Color::srgb(0.99, 0.97, 0.93),
                red_ink: Color::srgb(0.75, 0.18, 0.12),
                black_ink: Color::srgb(0.15, 0.15, 0.18),
                disc_border: Color::srgba(0.30, 0.28, 0.25, 0.15),
            },
            ThemeId::Rosewood => Palette {
                frame_dark: Color::srgb(0.20, 0.08, 0.05),
                frame_edge: Color::srgb(0.40, 0.18, 0.10),
                board_bg: Color::srgb(0.75, 0.48, 0.28),
                line_color: Color::srgb(0.20, 0.10, 0.05),
                river_color: Color::srgba(0.20, 0.10, 0.05, 0.50),
                disc_face: Color::srgb(0.95, 0.90, 0.80),
                red_ink: Color::srgb(0.80, 0.12, 0.08),
                black_ink: Color::srgb(0.08, 0.06, 0.05),
                disc_border: Color::srgba(0.0, 0.0, 0.0, 0.18),
            },
            ThemeId::Jade => Palette {
                frame_dark: Color::srgb(0.10, 0.18, 0.15),
                frame_edge: Color::srgb(0.18, 0.32, 0.26),
                board_bg: Color::srgb(0.65, 0.80, 0.72),
                line_color: Color::srgb(0.15, 0.25, 0.20),
                river_color: Color::srgba(0.15, 0.25, 0.20, 0.50),
                disc_face: Color::srgb(0.94, 0.96, 0.93),
                red_ink: Color::srgb(0.78, 0.15, 0.10),
                black_ink: Color::srgb(0.08, 0.12, 0.10),
                disc_border: Color::srgba(0.05, 0.15, 0.10, 0.18),
            },
        }
    }
}

/// Resource holding the active board theme.
#[derive(Resource)]
pub struct BoardTheme {
    pub id: ThemeId,
    pub palette: Palette,
}

impl Default for BoardTheme {
    fn default() -> Self {
        let id = ThemeId::Classic;
        BoardTheme {
            id,
            palette: id.palette(),
        }
    }
}

impl BoardTheme {
    /// Switch to the next theme.
    pub fn cycle(&mut self) {
        self.id = self.id.next();
        self.palette = self.id.palette();
    }
}
