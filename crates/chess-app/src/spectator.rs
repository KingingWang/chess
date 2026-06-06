//! Spectator mode for watching ongoing games.
//!
//! Allows players to watch games in progress without interfering.

use bevy::prelude::*;

/// Spectator mode state.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpectatorMode {
    pub active: bool,
    pub game_id: Option<String>,
    pub player_red: String,
    pub player_black: String,
    pub move_count: usize,
}

impl SpectatorMode {
    pub fn start(&mut self, game_id: &str, red: &str, black: &str) {
        self.active = true;
        self.game_id = Some(game_id.to_string());
        self.player_red = red.to_string();
        self.player_black = black.to_string();
        self.move_count = 0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.game_id = None;
        self.player_red.clear();
        self.player_black.clear();
        self.move_count = 0;
    }

    pub fn update_move_count(&mut self, count: usize) {
        self.move_count = count;
    }

    pub fn player_label(&self) -> String {
        if self.active {
            format!("{} vs {}", self.player_red, self.player_black)
        } else {
            String::new()
        }
    }
}

pub fn toggle_spectator(
    keys: Res<ButtonInput<KeyCode>>,
    mut spec: ResMut<SpectatorMode>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyW) {
        if spec.active {
            spec.stop();
            crate::toast::spawn_toast(&mut commands, &fonts, "观战模式已关闭");
        } else {
            crate::toast::spawn_toast(&mut commands, &fonts, "观战模式: 等待加入对局...");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let s = SpectatorMode::default();
        assert!(!s.active);
    }

    #[test]
    fn test_start_stop() {
        let mut s = SpectatorMode::default();
        s.start("game123", "红方选手", "黑方选手");
        assert!(s.active);
        assert_eq!(s.game_id, Some("game123".to_string()));
        s.stop();
        assert!(!s.active);
    }

    #[test]
    fn test_player_label() {
        let mut s = SpectatorMode::default();
        s.start("g1", "张三", "李四");
        assert!(s.player_label().contains("张三"));
        assert!(s.player_label().contains("李四"));
    }

    #[test]
    fn test_update_move_count() {
        let mut s = SpectatorMode::default();
        s.update_move_count(42);
        assert_eq!(s.move_count, 42);
    }
}
