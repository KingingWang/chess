//! User settings persistence.
//!
//! Saves and loads user preferences (board theme, sound volume, animation
//! speed, coordinate visibility) to a JSON file in the save directory.
//! Missing or malformed files are silently ignored — defaults are used.

use crate::animation::AnimSpeed;
use crate::board_theme::ThemeId;
use crate::sound::VolumeLevel;

/// Persisted user preferences.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSettings {
    #[serde(default)]
    pub theme: ThemeId,
    #[serde(default)]
    pub volume: VolumeLevel,
    #[serde(default)]
    pub anim_speed: AnimSpeed,
    #[serde(default = "default_true")]
    pub show_coordinates: bool,
    #[serde(default)]
    pub difficulty: Option<chess_ai::Difficulty>,
    #[serde(default)]
    pub version: u32,
}

fn default_true() -> bool {
    true
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: ThemeId::default(),
            volume: VolumeLevel::default(),
            anim_speed: AnimSpeed::default(),
            show_coordinates: true,
            difficulty: None,
            version: 1,
        }
    }
}

/// Directory where settings and saves are stored.
pub fn save_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join("xiangqi_saves"))
        .unwrap_or_else(|_| std::path::PathBuf::from("saves"))
}

/// Path to the settings file.
fn settings_path() -> std::path::PathBuf {
    save_dir().join("settings.json")
}

/// Load settings from disk. Returns defaults if file is missing or invalid.
pub fn load_settings() -> UserSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UserSettings::default(),
    }
}

/// Save current settings to disk. Errors are logged but not propagated.
pub fn save_settings(settings: &UserSettings) {
    let dir = save_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        bevy::log::warn!(error = %e, "failed to create settings directory");
        return;
    }
    let path = settings_path();
    match serde_json::to_string_pretty(settings) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                bevy::log::warn!(error = %e, "failed to write settings");
            }
        }
        Err(e) => {
            bevy::log::warn!(error = %e, "failed to serialize settings");
        }
    }
}

/// Build a `UserSettings` from the current resource values.
pub fn collect_settings(
    theme: &crate::board_theme::BoardTheme,
    volume: &crate::sound::SoundVolume,
    anim_speed: &crate::animation::AnimSpeedSetting,
    show_coords: &crate::board_view::ShowCoordinates,
) -> UserSettings {
    let existing = load_settings();
    UserSettings {
        theme: theme.id,
        volume: volume.level,
        anim_speed: anim_speed.0,
        show_coordinates: show_coords.0,
        difficulty: existing.difficulty,
        version: 1,
    }
}

/// Save only the AI difficulty by doing a read-modify-write of the settings
/// file. Used by callers that don't have access to all settings resources.
pub fn save_difficulty(diff: chess_ai::Difficulty) {
    let mut s = load_settings();
    s.difficulty = Some(diff);
    save_settings(&s);
}
