//! Performance analytics dashboard.
//!
//! Provides detailed statistics about player performance:
//! - Win/loss ratios by time control
//! - Average game length trends
//! - Opening success rates
//! - Move quality distribution
//! - Time management analysis

use bevy::prelude::*;
use std::collections::HashMap;

/// Time control category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeControlCategory {
    Bullet,    // < 3 min
    Blitz,     // 3-10 min
    Rapid,     // 10-30 min
    Classical, // > 30 min
    Unlimited, // No time control
}

impl TimeControlCategory {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Bullet => "子弹棋",
            Self::Blitz => "快棋",
            Self::Rapid => "快速",
            Self::Classical => "慢棋",
            Self::Unlimited => "无限",
        }
    }

    pub fn from_minutes(minutes: u32) -> Self {
        match minutes {
            0 => Self::Unlimited,
            1..=2 => Self::Bullet,
            3..=9 => Self::Blitz,
            10..=29 => Self::Rapid,
            _ => Self::Classical,
        }
    }
}

/// Game outcome for analytics.
#[derive(Debug, Clone, Copy)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}

/// Analytics resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct Analytics {
    /// Win/loss by time control.
    pub by_time_control: HashMap<TimeControlCategory, (u32, u32, u32)>, // (wins, losses, draws)
    /// Game lengths (move counts).
    pub game_lengths: Vec<u32>,
    /// Average thinking time per game (seconds).
    pub avg_think_times: Vec<f32>,
    /// Opening success: (opening_name, wins, total).
    pub opening_stats: HashMap<String, (u32, u32)>,
    /// Rating history (if applicable).
    pub rating_history: Vec<i32>,
}

impl Analytics {
    /// Record a game result.
    pub fn record_game(
        &mut self,
        time_minutes: u32,
        outcome: GameOutcome,
        move_count: u32,
        avg_time: f32,
        opening: Option<&str>,
    ) {
        let tc = TimeControlCategory::from_minutes(time_minutes);
        let entry = self.by_time_control.entry(tc).or_insert((0, 0, 0));
        match outcome {
            GameOutcome::Win => entry.0 += 1,
            GameOutcome::Loss => entry.1 += 1,
            GameOutcome::Draw => entry.2 += 1,
        }

        self.game_lengths.push(move_count);
        self.avg_think_times.push(avg_time);

        if let Some(name) = opening {
            let stats = self.opening_stats.entry(name.to_string()).or_insert((0, 0));
            stats.1 += 1;
            if matches!(outcome, GameOutcome::Win) {
                stats.0 += 1;
            }
        }
    }

    /// Get overall win rate.
    pub fn overall_win_rate(&self) -> f32 {
        let mut total_wins = 0u32;
        let mut total_games = 0u32;
        for (w, l, d) in self.by_time_control.values() {
            total_wins += w;
            total_games += w + l + d;
        }
        if total_games == 0 {
            return 0.0;
        }
        (total_wins as f32 / total_games as f32) * 100.0
    }

    /// Get average game length.
    pub fn avg_game_length(&self) -> f32 {
        if self.game_lengths.is_empty() {
            return 0.0;
        }
        self.game_lengths.iter().sum::<u32>() as f32 / self.game_lengths.len() as f32
    }

    /// Get average thinking time.
    pub fn avg_thinking_time(&self) -> f32 {
        if self.avg_think_times.is_empty() {
            return 0.0;
        }
        self.avg_think_times.iter().sum::<f32>() / self.avg_think_times.len() as f32
    }

    /// Get best opening by win rate (minimum 3 games).
    pub fn best_opening(&self) -> Option<(&String, f32)> {
        self.opening_stats
            .iter()
            .filter(|(_, (_, total))| *total >= 3)
            .map(|(name, (wins, total))| (name, *wins as f32 / *total as f32 * 100.0))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    /// Get total games played.
    pub fn total_games(&self) -> u32 {
        self.by_time_control
            .values()
            .map(|(w, l, d)| w + l + d)
            .sum()
    }

    /// Reset all analytics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let a = Analytics::default();
        assert_eq!(a.total_games(), 0);
        assert_eq!(a.overall_win_rate(), 0.0);
    }

    #[test]
    fn test_record_game() {
        let mut a = Analytics::default();
        a.record_game(10, GameOutcome::Win, 40, 5.0, Some("中炮"));
        assert_eq!(a.total_games(), 1);
        assert_eq!(a.overall_win_rate(), 100.0);
    }

    #[test]
    fn test_time_control_categories() {
        assert_eq!(
            TimeControlCategory::from_minutes(0),
            TimeControlCategory::Unlimited
        );
        assert_eq!(
            TimeControlCategory::from_minutes(2),
            TimeControlCategory::Bullet
        );
        assert_eq!(
            TimeControlCategory::from_minutes(5),
            TimeControlCategory::Blitz
        );
        assert_eq!(
            TimeControlCategory::from_minutes(15),
            TimeControlCategory::Rapid
        );
        assert_eq!(
            TimeControlCategory::from_minutes(60),
            TimeControlCategory::Classical
        );
    }

    #[test]
    fn test_win_rate_by_category() {
        let mut a = Analytics::default();
        a.record_game(5, GameOutcome::Win, 30, 4.0, None);
        a.record_game(5, GameOutcome::Loss, 25, 3.5, None);
        a.record_game(15, GameOutcome::Win, 50, 8.0, None);

        let blitz = a.by_time_control.get(&TimeControlCategory::Blitz).unwrap();
        assert_eq!(blitz.0, 1); // wins
        assert_eq!(blitz.1, 1); // losses
    }

    #[test]
    fn test_avg_game_length() {
        let mut a = Analytics::default();
        a.record_game(5, GameOutcome::Win, 30, 4.0, None);
        a.record_game(5, GameOutcome::Loss, 50, 3.5, None);
        assert_eq!(a.avg_game_length(), 40.0);
    }

    #[test]
    fn test_opening_stats() {
        let mut a = Analytics::default();
        a.record_game(10, GameOutcome::Win, 40, 5.0, Some("中炮"));
        a.record_game(10, GameOutcome::Win, 35, 5.0, Some("中炮"));
        a.record_game(10, GameOutcome::Loss, 30, 5.0, Some("中炮"));
        a.record_game(10, GameOutcome::Win, 45, 5.0, Some("飞相"));

        let (wins, total) = a.opening_stats.get("中炮").unwrap();
        assert_eq!(*wins, 2);
        assert_eq!(*total, 3);
    }

    #[test]
    fn test_best_opening() {
        let mut a = Analytics::default();
        for _ in 0..5 {
            a.record_game(10, GameOutcome::Win, 40, 5.0, Some("中炮"));
        }
        for _ in 0..3 {
            a.record_game(10, GameOutcome::Loss, 40, 5.0, Some("中炮"));
        }
        let best = a.best_opening();
        assert!(best.is_some());
        let (name, rate) = best.unwrap();
        assert_eq!(name, "中炮");
        assert!((rate - 62.5).abs() < 0.1);
    }

    #[test]
    fn test_reset() {
        let mut a = Analytics::default();
        a.record_game(10, GameOutcome::Win, 40, 5.0, Some("中炮"));
        a.reset();
        assert_eq!(a.total_games(), 0);
    }

    #[test]
    fn test_time_control_labels() {
        assert_eq!(TimeControlCategory::Bullet.label_cn(), "子弹棋");
        assert_eq!(TimeControlCategory::Blitz.label_cn(), "快棋");
    }
}
