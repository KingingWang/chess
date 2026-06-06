//! Comprehensive game statistics dashboard.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone)]
pub struct GameStatisticsDashboard {
    pub visible: bool,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub by_opening: HashMap<String, (u32, u32, u32)>,
    pub avg_game_length: f32,
    pub total_time_played: f64,
}

impl Default for GameStatisticsDashboard {
    fn default() -> Self {
        Self {
            visible: false,
            games_played: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            by_opening: HashMap::new(),
            avg_game_length: 0.0,
            total_time_played: 0.0,
        }
    }
}

impl GameStatisticsDashboard {
    pub fn record_game(&mut self, won: bool, opening: &str, moves: u32, time: f64) {
        self.games_played += 1;
        if won {
            self.wins += 1;
        } else {
            self.losses += 1;
        }
        let entry = self
            .by_opening
            .entry(opening.to_string())
            .or_insert((0, 0, 0));
        if won {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
        entry.2 += 1;
        self.avg_game_length = ((self.avg_game_length * (self.games_played - 1) as f32)
            + moves as f32)
            / self.games_played as f32;
        self.total_time_played += time;
    }
    pub fn win_rate(&self) -> f32 {
        if self.games_played == 0 {
            0.0
        } else {
            self.wins as f32 / self.games_played as f32 * 100.0
        }
    }
    pub fn best_opening(&self) -> Option<(&str, f32)> {
        self.by_opening
            .iter()
            .filter(|(_, (_, _, games))| *games >= 3)
            .map(|(name, (wins, _, games))| (name.as_str(), *wins as f32 / *games as f32 * 100.0))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}

pub fn toggle_stats_dashboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut gsd: ResMut<GameStatisticsDashboard>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyT) {
        gsd.visible = !gsd.visible;
        let msg = if gsd.visible {
            format!(
                "统计仪表板: {}局, 胜率{:.0}%",
                gsd.games_played,
                gsd.win_rate()
            )
        } else {
            "统计仪表板已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut gsd = GameStatisticsDashboard::default();
        gsd.record_game(true, "中炮", 40, 600.0);
        assert_eq!(gsd.games_played, 1);
        assert_eq!(gsd.win_rate(), 100.0);
    }
    #[test]
    fn test_best_opening() {
        let mut gsd = GameStatisticsDashboard::default();
        for _ in 0..5 {
            gsd.record_game(true, "中炮", 40, 600.0);
        }
        for _ in 0..2 {
            gsd.record_game(false, "飞相", 35, 500.0);
        }
        let best = gsd.best_opening();
        assert!(best.is_some());
        assert_eq!(best.unwrap().0, "中炮");
    }
}
