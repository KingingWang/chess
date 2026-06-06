//! Sound theme selection for different audio experiences.
//!
//! Allows users to choose from different sound themes:
//! - Traditional (wood pieces on wood board)
//! - Modern (clean digital sounds)
//! - Minimal (subtle click sounds)

use bevy::prelude::*;

/// Available sound themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoundTheme {
    /// Traditional wooden chess sounds.
    #[default]
    Traditional,
    /// Clean digital/modern sounds.
    Modern,
    /// Minimal, subtle sounds.
    Minimal,
    /// No sounds at all.
    Muted,
}

impl SoundTheme {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Traditional => "传统",
            Self::Modern => "现代",
            Self::Minimal => "简约",
            Self::Muted => "静音",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Traditional => Self::Modern,
            Self::Modern => Self::Minimal,
            Self::Minimal => Self::Muted,
            Self::Muted => Self::Traditional,
        }
    }
}

/// Sound theme resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct SoundThemeResource {
    pub theme: SoundTheme,
}

/// Cycle through sound themes with keyboard shortcut.
pub fn cycle_sound_theme(
    keys: Res<ButtonInput<KeyCode>>,
    mut sound_theme: ResMut<SoundThemeResource>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyS) {
        sound_theme.theme = sound_theme.theme.next();
        let msg = format!("音效主题: {}", sound_theme.theme.label_cn());
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_cycle() {
        let mut t = SoundTheme::Traditional;
        t = t.next();
        assert_eq!(t, SoundTheme::Modern);
        t = t.next();
        assert_eq!(t, SoundTheme::Minimal);
        t = t.next();
        assert_eq!(t, SoundTheme::Muted);
        t = t.next();
        assert_eq!(t, SoundTheme::Traditional);
    }

    #[test]
    fn test_labels() {
        assert_eq!(SoundTheme::Traditional.label_cn(), "传统");
        assert_eq!(SoundTheme::Modern.label_cn(), "现代");
        assert_eq!(SoundTheme::Minimal.label_cn(), "简约");
        assert_eq!(SoundTheme::Muted.label_cn(), "静音");
    }

    #[test]
    fn test_default() {
        let r = SoundThemeResource::default();
        assert_eq!(r.theme, SoundTheme::Traditional);
    }
}
