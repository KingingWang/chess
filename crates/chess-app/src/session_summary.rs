//! Session summary report generator.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct SessionSummary {
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub total_moves: u64,
    pub avg_game_length: f32,
    pub total_time_secs: f64,
    pub puzzles_solved: u32,
    pub achievements_unlocked: u32,
}

impl Default for SessionSummary {
    fn default() -> Self {
        Self {
            games_played: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            total_moves: 0,
            avg_game_length: 0.0,
            total_time_secs: 0.0,
            puzzles_solved: 0,
            achievements_unlocked: 0,
        }
    }
}

impl SessionSummary {
    pub fn record_game(&mut self, moves: u32, result: f32) {
        self.games_played += 1;
        self.total_moves += moves as u64;
        self.avg_game_length = self.total_moves as f32 / self.games_played as f32;
        if result > 0.5 {
            self.wins += 1;
        } else if result < 0.5 {
            self.losses += 1;
        } else {
            self.draws += 1;
        }
    }
    pub fn win_rate(&self) -> f32 {
        if self.games_played == 0 {
            0.0
        } else {
            self.wins as f32 / self.games_played as f32 * 100.0
        }
    }
    pub fn generate_report(&self) -> String {
        let hours = (self.total_time_secs / 3600.0) as u32;
        let mins = ((self.total_time_secs % 3600.0) / 60.0) as u32;
        format!(
            "=== 游戏会话总结 ===\n对局数: {}\n胜/负/和: {}/{}/{}\n胜率: {:.1}%\n平均步数: {:.1}\n游戏时间: {}时{}分\n解题数: {}\n成就解锁: {}",
            self.games_played, self.wins, self.losses, self.draws, self.win_rate(), self.avg_game_length, hours, mins, self.puzzles_solved, self.achievements_unlocked
        )
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn show_summary(
    keys: Res<ButtonInput<KeyCode>>,
    summary: Res<SessionSummary>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyR) {
        let report = summary.generate_report();
        let first_line = report.lines().next().unwrap_or("会话总结");
        crate::toast::spawn_toast(&mut commands, &fonts, first_line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut s = SessionSummary::default();
        s.record_game(40, 1.0);
        assert_eq!(s.games_played, 1);
        assert_eq!(s.wins, 1);
    }
    #[test]
    fn test_win_rate() {
        let mut s = SessionSummary::default();
        s.record_game(40, 1.0);
        s.record_game(30, 0.0);
        assert_eq!(s.win_rate(), 50.0);
    }
    #[test]
    fn test_report() {
        let s = SessionSummary::default();
        let r = s.generate_report();
        assert!(r.contains("游戏会话总结"));
    }
}
