//! Game result tracking with streaks.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameResult {
    Win,
    Loss,
    Draw,
}

#[derive(Resource, Debug, Clone)]
pub struct GameResultTracker {
    pub results: Vec<GameResult>,
    pub current_streak: i32,
    pub longest_streak: i32,
}

impl Default for GameResultTracker {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            current_streak: 0,
            longest_streak: 0,
        }
    }
}

impl GameResultTracker {
    pub fn record(&mut self, result: GameResult) {
        self.results.push(result);
        match result {
            GameResult::Win => {
                self.current_streak = if self.current_streak > 0 {
                    self.current_streak + 1
                } else {
                    1
                };
                self.longest_streak = self.longest_streak.max(self.current_streak);
            }
            GameResult::Loss => {
                self.current_streak = if self.current_streak < 0 {
                    self.current_streak - 1
                } else {
                    -1
                };
            }
            GameResult::Draw => {
                self.current_streak = 0;
            }
        }
    }
    pub fn win_streak(&self) -> u32 {
        if self.current_streak > 0 {
            self.current_streak as u32
        } else {
            0
        }
    }
    pub fn loss_streak(&self) -> u32 {
        if self.current_streak < 0 {
            (-self.current_streak) as u32
        } else {
            0
        }
    }
}

pub fn show_streak(
    keys: Res<ButtonInput<KeyCode>>,
    grt: Res<GameResultTracker>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyX) {
        let msg = if grt.current_streak > 0 {
            format!("当前连胜: {}局", grt.win_streak())
        } else if grt.current_streak < 0 {
            format!("当前连败: {}局", grt.loss_streak())
        } else {
            "当前无连胜/连败".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_streak() {
        let mut grt = GameResultTracker::default();
        grt.record(GameResult::Win);
        grt.record(GameResult::Win);
        grt.record(GameResult::Win);
        assert_eq!(grt.win_streak(), 3);
        assert_eq!(grt.longest_streak, 3);
    }
    #[test]
    fn test_break() {
        let mut grt = GameResultTracker::default();
        grt.record(GameResult::Win);
        grt.record(GameResult::Loss);
        assert_eq!(grt.win_streak(), 0);
        assert_eq!(grt.loss_streak(), 1);
    }
}
