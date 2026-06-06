//! Quick game mode with rapid time control.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickGameType {
    Blitz10,
    Bullet5,
    Bullet3,
}

impl QuickGameType {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Blitz10 => "10秒快棋",
            Self::Bullet5 => "5秒超快",
            Self::Bullet3 => "3秒闪电",
        }
    }
    pub fn seconds_per_move(&self) -> f32 {
        match self {
            Self::Blitz10 => 10.0,
            Self::Bullet5 => 5.0,
            Self::Bullet3 => 3.0,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct QuickGame {
    pub active: bool,
    pub game_type: QuickGameType,
    pub red_time: f32,
    pub black_time: f32,
}

impl Default for QuickGame {
    fn default() -> Self {
        Self {
            active: false,
            game_type: QuickGameType::Blitz10,
            red_time: 10.0,
            black_time: 10.0,
        }
    }
}

impl QuickGame {
    pub fn start(&mut self, game_type: QuickGameType) {
        self.active = true;
        self.game_type = game_type;
        self.red_time = game_type.seconds_per_move();
        self.black_time = game_type.seconds_per_move();
    }
    pub fn stop(&mut self) {
        self.active = false;
    }
    pub fn tick(&mut self, delta: f32, is_red_turn: bool) {
        if !self.active {
            return;
        }
        if is_red_turn {
            self.red_time = (self.red_time - delta).max(0.0);
        } else {
            self.black_time = (self.black_time - delta).max(0.0);
        }
    }
    pub fn flag_fallen(&self) -> Option<bool> {
        if self.red_time <= 0.0 {
            Some(false)
        } else if self.black_time <= 0.0 {
            Some(true)
        } else {
            None
        }
    }
}

pub fn toggle_quick_game(
    keys: Res<ButtonInput<KeyCode>>,
    mut qg: ResMut<QuickGame>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyX) {
        if qg.active {
            qg.stop();
            crate::toast::spawn_toast(&mut commands, &fonts, "快棋模式已关闭");
        } else {
            qg.start(QuickGameType::Blitz10);
            crate::toast::spawn_toast(
                &mut commands,
                &fonts,
                &format!("快棋模式: {}", qg.game_type.label_cn()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tick() {
        let mut qg = QuickGame::default();
        qg.start(QuickGameType::Blitz10);
        qg.tick(1.0, true);
        assert!(qg.red_time < 10.0);
    }
    #[test]
    fn test_flag() {
        let mut qg = QuickGame::default();
        qg.start(QuickGameType::Bullet3);
        qg.red_time = 0.0;
        assert_eq!(qg.flag_fallen(), Some(false));
    }
}
