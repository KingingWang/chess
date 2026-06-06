//! Comprehensive game statistics panel.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct GameStatisticsPanel {
    pub visible: bool,
    pub total_games: u32,
    pub total_wins: u32,
    pub total_losses: u32,
    pub total_draws: u32,
    pub avg_moves_per_game: f32,
    pub longest_game: u32,
    pub shortest_game: u32,
}

impl Default for GameStatisticsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            total_games: 0,
            total_wins: 0,
            total_losses: 0,
            total_draws: 0,
            avg_moves_per_game: 0.0,
            longest_game: 0,
            shortest_game: u32::MAX,
        }
    }
}

impl GameStatisticsPanel {
    pub fn win_rate(&self) -> f32 {
        if self.total_games == 0 {
            0.0
        } else {
            self.total_wins as f32 / self.total_games as f32 * 100.0
        }
    }
    pub fn record_game(&mut self, moves: u32, won: bool) {
        self.total_games += 1;
        if won {
            self.total_wins += 1;
        } else {
            self.total_losses += 1;
        }
        self.longest_game = self.longest_game.max(moves);
        self.shortest_game = self.shortest_game.min(moves);
        self.avg_moves_per_game = ((self.avg_moves_per_game * (self.total_games - 1) as f32)
            + moves as f32)
            / self.total_games as f32;
    }
    pub fn summary(&self) -> String {
        format!(
            "总对局: {} | 胜率: {:.1}% | 平均步数: {:.1}",
            self.total_games,
            self.win_rate(),
            self.avg_moves_per_game
        )
    }
}

pub fn toggle_stats_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut sp: ResMut<GameStatisticsPanel>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyP) {
        sp.visible = !sp.visible;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if sp.visible {
                "统计面板已打开"
            } else {
                "统计面板已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut sp = GameStatisticsPanel::default();
        sp.record_game(40, true);
        assert_eq!(sp.total_games, 1);
        assert_eq!(sp.win_rate(), 100.0);
    }
    #[test]
    fn test_summary() {
        let sp = GameStatisticsPanel::default();
        let s = sp.summary();
        assert!(s.contains("总对局"));
    }
}
