//! Automatic game save and resume system.
//!
//! Saves game state periodically and on exit so games can be resumed
//! after unexpected termination or closing the application.

use bevy::prelude::*;
use std::fs;
use std::path::PathBuf;

use crate::app_state::{CoreGame, GameMode, UiFonts};

/// Auto-save interval in seconds.
const AUTOSAVE_INTERVAL_SECS: f32 = 30.0;

/// Resource managing auto-save state.
#[derive(Resource, Debug)]
pub struct AutoSave {
    /// Whether auto-save is enabled.
    pub enabled: bool,
    /// Time since last save.
    pub time_since_save: f32,
    /// Last known move count (to detect changes).
    pub last_move_count: usize,
    /// Path where auto-saves are stored.
    pub save_dir: PathBuf,
    /// Whether there's an existing auto-save to resume.
    pub has_saved_game: bool,
}

impl Default for AutoSave {
    fn default() -> Self {
        let save_dir = crate::settings::save_dir();
        let autosave_path = save_dir.join("autosave.pgn");
        let has_saved_game = autosave_path.exists();

        Self {
            enabled: true,
            time_since_save: 0.0,
            last_move_count: 0,
            save_dir,
            has_saved_game,
        }
    }
}

impl AutoSave {
    /// Get the auto-save file path.
    pub fn autosave_path(&self) -> PathBuf {
        self.save_dir.join("autosave.pgn")
    }

    /// Get the auto-save metadata path.
    pub fn autosave_meta_path(&self) -> PathBuf {
        self.save_dir.join("autosave.meta")
    }

    /// Save the current game state.
    pub fn save_game(&mut self, core: &CoreGame) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        // Ensure save directory exists
        fs::create_dir_all(&self.save_dir)
            .map_err(|e| format!("Failed to create save dir: {}", e))?;

        // Save FEN for the current position
        let fen = core.game.board().to_fen();
        let moves = core
            .game
            .history()
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let notation = format!(
                    "{}{}{}{}",
                    (b'a' + m.mv().from.file()) as char,
                    m.mv().from.rank() + 1,
                    (b'a' + m.mv().to.file()) as char,
                    m.mv().to.rank() + 1
                );
                if i % 2 == 0 {
                    format!("{}. {}", i / 2 + 1, notation)
                } else {
                    notation
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Build PGN
        let mode_str = match core.mode {
            GameMode::VsAi => "VsAi",
            GameMode::LocalPvp => "Local",
            _ => "Other",
        };

        let pgn = format!(
            "[Event \"AutoSave\"]\n\
             [Mode \"{}\"]\n\
             [FEN \"{}\"]\n\
             [MoveCount \"{}\"]\n\n\
             {}",
            mode_str,
            fen,
            core.game.history_len(),
            moves
        );

        fs::write(self.autosave_path(), &pgn)
            .map_err(|e| format!("Failed to write autosave: {}", e))?;

        // Save metadata
        let meta = format!(
            "mode={}\nmoves={}\nfen={}",
            mode_str,
            core.game.history_len(),
            fen
        );
        fs::write(self.autosave_meta_path(), &meta)
            .map_err(|e| format!("Failed to write autosave meta: {}", e))?;

        self.last_move_count = core.game.history_len();
        self.time_since_save = 0.0;
        self.has_saved_game = true;

        Ok(())
    }

    /// Delete the auto-save file.
    pub fn clear_save(&mut self) {
        let _ = fs::remove_file(self.autosave_path());
        let _ = fs::remove_file(self.autosave_meta_path());
        self.has_saved_game = false;
        self.last_move_count = 0;
    }

    /// Load auto-save metadata.
    pub fn load_meta(&self) -> Option<AutoSaveMeta> {
        let path = self.autosave_meta_path();
        let content = fs::read_to_string(&path).ok()?;

        let mut mode = String::new();
        let mut moves = 0usize;
        let mut fen = String::new();

        for line in content.lines() {
            if let Some(v) = line.strip_prefix("mode=") {
                mode = v.to_string();
            } else if let Some(v) = line.strip_prefix("moves=") {
                moves = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("fen=") {
                fen = v.to_string();
            }
        }

        Some(AutoSaveMeta {
            mode,
            move_count: moves,
            fen,
        })
    }
}

/// Auto-save metadata.
#[derive(Debug, Clone)]
pub struct AutoSaveMeta {
    /// Game mode.
    pub mode: String,
    /// Number of moves.
    pub move_count: usize,
    /// FEN position.
    pub fen: String,
}

impl AutoSaveMeta {
    /// Get a display string.
    pub fn display(&self) -> String {
        let mode_cn = match self.mode.as_str() {
            "VsAi" => "人机",
            "Local" => "本地",
            _ => "其他",
        };
        format!("{}对局, {}步", mode_cn, self.move_count)
    }
}

/// System to periodically auto-save the game.
pub fn periodic_autosave(time: Res<Time>, core: Res<CoreGame>, mut autosave: ResMut<AutoSave>) {
    if !autosave.enabled {
        return;
    }

    // Only autosave in active game modes
    match core.mode {
        GameMode::VsAi | GameMode::LocalPvp => {}
        _ => return,
    }

    // Only save if there are moves to save
    let move_count = core.game.history_len();
    if move_count == 0 {
        return;
    }

    autosave.time_since_save += time.delta_secs();

    // Save every AUTOSAVE_INTERVAL_SECS or when moves change
    if autosave.time_since_save >= AUTOSAVE_INTERVAL_SECS || move_count != autosave.last_move_count
    {
        if autosave.time_since_save >= AUTOSAVE_INTERVAL_SECS {
            let _ = autosave.save_game(&core);
        }
    }
}

/// System to save on every move (lightweight — only writes if move count changed).
pub fn autosave_on_move(core: Res<CoreGame>, mut autosave: ResMut<AutoSave>) {
    if !autosave.enabled {
        return;
    }

    let move_count = core.game.history_len();
    if move_count > autosave.last_move_count && move_count > 0 {
        let _ = autosave.save_game(&core);
    }
}

/// System to clear autosave when game ends or player returns to menu.
pub fn clear_autosave_on_menu(core: Res<CoreGame>, mut autosave: ResMut<AutoSave>) {
    if core.game.history_len() == 0 && autosave.has_saved_game {
        autosave.clear_save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Board;

    #[test]
    fn test_autosave_default() {
        let autosave = AutoSave {
            enabled: true,
            time_since_save: 0.0,
            last_move_count: 0,
            save_dir: PathBuf::from("/tmp/test_chess_autosave"),
            has_saved_game: false,
        };
        assert!(autosave.enabled);
        assert_eq!(autosave.last_move_count, 0);
    }

    #[test]
    fn test_autosave_paths() {
        let autosave = AutoSave {
            enabled: true,
            time_since_save: 0.0,
            last_move_count: 0,
            save_dir: PathBuf::from("/tmp/test_chess"),
            has_saved_game: false,
        };
        assert_eq!(
            autosave.autosave_path(),
            PathBuf::from("/tmp/test_chess/autosave.pgn")
        );
        assert_eq!(
            autosave.autosave_meta_path(),
            PathBuf::from("/tmp/test_chess/autosave.meta")
        );
    }

    #[test]
    fn test_meta_display() {
        let meta = AutoSaveMeta {
            mode: "VsAi".to_string(),
            move_count: 42,
            fen: String::new(),
        };
        let display = meta.display();
        assert!(display.contains("人机"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_meta_display_local() {
        let meta = AutoSaveMeta {
            mode: "Local".to_string(),
            move_count: 10,
            fen: String::new(),
        };
        let display = meta.display();
        assert!(display.contains("本地"));
    }

    #[test]
    fn test_clear_save() {
        let dir = PathBuf::from("/tmp/test_chess_clear");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("autosave.pgn"), "test");
        let _ = fs::write(dir.join("autosave.meta"), "test");

        let mut autosave = AutoSave {
            enabled: true,
            time_since_save: 0.0,
            last_move_count: 5,
            save_dir: dir.clone(),
            has_saved_game: true,
        };

        autosave.clear_save();
        assert!(!autosave.has_saved_game);
        assert_eq!(autosave.last_move_count, 0);
        assert!(!dir.join("autosave.pgn").exists());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_meta_none() {
        let autosave = AutoSave {
            enabled: true,
            time_since_save: 0.0,
            last_move_count: 0,
            save_dir: PathBuf::from("/tmp/test_chess_nometa"),
            has_saved_game: false,
        };
        assert!(autosave.load_meta().is_none());
    }

    #[test]
    fn test_disabled_autosave() {
        let mut autosave = AutoSave {
            enabled: false,
            time_since_save: 0.0,
            last_move_count: 0,
            save_dir: PathBuf::from("/tmp/test_chess_disabled"),
            has_saved_game: false,
        };
        let core = CoreGame::default();
        let result = autosave.save_game(&core);
        assert!(result.is_ok()); // Should succeed but not write
    }
}
