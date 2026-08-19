//! Shared UI design tokens — the "玄玉" (ink & jade) visual language.
//!
//! A single source of truth for colours and recurring styles so every menu,
//! panel and dialog feels like one product. Warm ink backgrounds, brass-gold
//! ornaments, jade for primary/positive actions, cinnabar for the Red side
//! and dangerous actions.

use bevy::prelude::*;

// --- Core palette ---------------------------------------------------------

/// Deep warm ink — the app backdrop.
pub const INK: Color = Color::srgb(0.070, 0.064, 0.055);
/// Full-panel surface (sidebars).
pub const PANEL: Color = Color::srgb(0.104, 0.095, 0.082);
/// Raised card surface.
pub const CARD: Color = Color::srgb(0.141, 0.129, 0.110);
/// Slightly lighter card for hover/nested surfaces.
pub const CARD_RAISED: Color = Color::srgb(0.180, 0.165, 0.140);

/// Hairline border — barely-there warm line.
pub const HAIRLINE: Color = Color::srgba(0.85, 0.70, 0.42, 0.14);
/// Stronger hairline for emphasis (hover, active cards).
pub const HAIRLINE_STRONG: Color = Color::srgba(0.85, 0.70, 0.42, 0.38);

/// Brass-gold ornament (titles, seals, fine lines).
pub const GOLD: Color = Color::srgb(0.792, 0.639, 0.373);
/// Brighter gold for the big title.
pub const GOLD_BRIGHT: Color = Color::srgb(0.898, 0.760, 0.494);
/// Dim gold line.
pub const GOLD_DIM: Color = Color::srgba(0.792, 0.639, 0.373, 0.45);

/// Jade — primary actions, positive affordances.
pub const JADE: Color = Color::srgb(0.322, 0.651, 0.522);
pub const JADE_BRIGHT: Color = Color::srgb(0.427, 0.780, 0.631);
/// Translucent jade fill for primary buttons.
pub const JADE_FILL: Color = Color::srgba(0.290, 0.600, 0.478, 0.26);
pub const JADE_FILL_HOVER: Color = Color::srgba(0.322, 0.651, 0.522, 0.38);
pub const JADE_BORDER: Color = Color::srgba(0.322, 0.651, 0.522, 0.55);

/// Cinnabar — the Red side, dangerous actions.
pub const CINNABAR: Color = Color::srgb(0.678, 0.263, 0.208);
pub const CINNABAR_HOVER: Color = Color::srgb(0.780, 0.322, 0.251);
pub const CINNABAR_FILL: Color = Color::srgba(0.678, 0.263, 0.208, 0.15);
pub const CINNABAR_BORDER: Color = Color::srgba(0.678, 0.263, 0.208, 0.55);

/// Primary text — warm paper white.
pub const TEXT: Color = Color::srgb(0.937, 0.910, 0.851);
/// Secondary text.
pub const TEXT_DIM: Color = Color::srgb(0.647, 0.604, 0.537);
/// Tertiary text / hints.
pub const TEXT_FAINT: Color = Color::srgb(0.427, 0.396, 0.345);

/// Modal scrim behind dialogs.
pub const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.62);

/// Soft drop shadow used by cards and dialogs.
pub fn card_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.55),
        Val::Px(0.0),
        Val::Px(14.0),
        Val::Px(10.0),
        Val::Px(40.0),
    )
}

/// Per-button colour set so hover systems can restore the right base style
/// (primary/secondary/danger buttons no longer share one global look).
#[derive(Component, Debug, Clone, Copy)]
pub struct BtnStyle {
    pub bg: Color,
    pub bg_hover: Color,
    pub border: Color,
    pub border_hover: Color,
}

impl BtnStyle {
    /// Jade-tinted primary action.
    pub fn primary() -> Self {
        BtnStyle {
            bg: JADE_FILL,
            bg_hover: JADE_FILL_HOVER,
            border: JADE_BORDER,
            border_hover: JADE_BRIGHT,
        }
    }

    /// Quiet outlined button on the panel surface.
    pub fn secondary() -> Self {
        BtnStyle {
            bg: Color::srgba(0.141, 0.129, 0.110, 0.6),
            bg_hover: CARD_RAISED,
            border: HAIRLINE,
            border_hover: HAIRLINE_STRONG,
        }
    }

    /// Cinnabar danger action (resign etc.).
    pub fn danger() -> Self {
        BtnStyle {
            bg: CINNABAR_FILL,
            bg_hover: Color::srgba(0.678, 0.263, 0.208, 0.30),
            border: CINNABAR_BORDER,
            border_hover: CINNABAR_HOVER,
        }
    }

    /// Borderless ghost (e.g. back-to-menu).
    pub fn ghost() -> Self {
        BtnStyle {
            bg: Color::NONE,
            bg_hover: Color::srgba(0.85, 0.70, 0.42, 0.10),
            border: Color::NONE,
            border_hover: Color::NONE,
        }
    }
}
