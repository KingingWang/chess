//! Achievement system for tracking milestones and encouraging play.
//!
//! Awards achievements for various accomplishments:
//! - Win streaks
//! - Opening mastery
//! - Puzzle solving
//! - Time records
//! - Special moves

use bevy::prelude::*;
use std::collections::HashSet;

/// Achievement definition.
#[derive(Debug, Clone)]
pub struct Achievement {
    /// Unique identifier.
    pub id: &'static str,
    /// Chinese name.
    pub name_cn: &'static str,
    /// Description.
    pub description: &'static str,
    /// Category.
    pub category: AchievementCategory,
}

/// Achievement categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementCategory {
    /// Combat achievements (wins, streaks).
    Combat,
    /// Opening knowledge.
    Opening,
    /// Puzzle solving.
    Puzzle,
    /// Time-based achievements.
    Speed,
    /// Exploration (trying different features).
    Exploration,
}

impl AchievementCategory {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Combat => "战斗",
            Self::Opening => "开局",
            Self::Puzzle => "残局",
            Self::Speed => "速度",
            Self::Exploration => "探索",
        }
    }
}

/// All available achievements.
static ACHIEVEMENTS: &[Achievement] = &[
    // Combat achievements
    Achievement {
        id: "first_win",
        name_cn: "初战告捷",
        description: "赢得第一场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_streak_3",
        name_cn: "三连胜",
        description: "连续赢得3场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_streak_5",
        name_cn: "五连胜",
        description: "连续赢得5场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_streak_10",
        name_cn: "十连胜",
        description: "连续赢得10场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_10_games",
        name_cn: "十胜将军",
        description: "总共赢得10场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_50_games",
        name_cn: "五十胜将军",
        description: "总共赢得50场对局",
        category: AchievementCategory::Combat,
    },
    Achievement {
        id: "win_as_black",
        name_cn: "后手制胜",
        description: "作为黑方赢得一场对局",
        category: AchievementCategory::Combat,
    },
    // Opening achievements
    Achievement {
        id: "opening_central_cannon",
        name_cn: "中炮大师",
        description: "使用中炮开局赢得5场对局",
        category: AchievementCategory::Opening,
    },
    Achievement {
        id: "opening_flying_elephant",
        name_cn: "飞相高手",
        description: "使用飞相局赢得5场对局",
        category: AchievementCategory::Opening,
    },
    Achievement {
        id: "opening_diversity",
        name_cn: "开局大师",
        description: "使用5种不同的开局各赢一场",
        category: AchievementCategory::Opening,
    },
    // Puzzle achievements
    Achievement {
        id: "puzzle_first",
        name_cn: "残局入门",
        description: "完成第一个残局题",
        category: AchievementCategory::Puzzle,
    },
    Achievement {
        id: "puzzle_10",
        name_cn: "残局达人",
        description: "完成10个残局题",
        category: AchievementCategory::Puzzle,
    },
    Achievement {
        id: "puzzle_50",
        name_cn: "残局大师",
        description: "完成50个残局题",
        category: AchievementCategory::Puzzle,
    },
    // Speed achievements
    Achievement {
        id: "speed_bullets",
        name_cn: "闪电战",
        description: "在60步以内赢得对局",
        category: AchievementCategory::Speed,
    },
    Achievement {
        id: "speed_marathon",
        name_cn: "持久战",
        description: "完成一场超过100步的对局",
        category: AchievementCategory::Speed,
    },
    Achievement {
        id: "speed_quick_think",
        name_cn: "快思",
        description: "平均每步思考不超过5秒赢得对局",
        category: AchievementCategory::Speed,
    },
    // Exploration achievements
    Achievement {
        id: "try_analysis",
        name_cn: "分析探索",
        description: "使用分析模式查看局面",
        category: AchievementCategory::Exploration,
    },
    Achievement {
        id: "try_replay",
        name_cn: "回放体验",
        description: "使用回放模式回顾对局",
        category: AchievementCategory::Exploration,
    },
    Achievement {
        id: "try_board_theme",
        name_cn: "主题收藏",
        description: "尝试所有棋盘主题",
        category: AchievementCategory::Exploration,
    },
    Achievement {
        id: "try_difficulty",
        name_cn: "难度挑战",
        description: "在最高难度下赢得对局",
        category: AchievementCategory::Exploration,
    },
];

/// Resource tracking achievement progress.
#[derive(Resource, Debug, Clone)]
pub struct AchievementTracker {
    /// Set of unlocked achievement IDs.
    pub unlocked: HashSet<String>,
    /// Total wins.
    pub total_wins: u32,
    /// Current win streak.
    pub current_streak: u32,
    /// Longest win streak ever.
    pub longest_streak: u32,
    /// Puzzles solved.
    pub puzzles_solved: u32,
    /// Opening win counts.
    pub opening_wins: std::collections::HashMap<String, u32>,
    /// Unique openings won with.
    pub unique_opening_wins: HashSet<String>,
    /// Whether to show achievement notifications.
    pub notifications_enabled: bool,
}

impl Default for AchievementTracker {
    fn default() -> Self {
        Self {
            unlocked: HashSet::new(),
            total_wins: 0,
            current_streak: 0,
            longest_streak: 0,
            puzzles_solved: 0,
            opening_wins: std::collections::HashMap::new(),
            unique_opening_wins: HashSet::new(),
            notifications_enabled: true,
        }
    }
}

impl AchievementTracker {
    /// Check if an achievement is unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }

    /// Try to unlock an achievement, returns true if newly unlocked.
    pub fn try_unlock(&mut self, id: &str) -> bool {
        if !self.unlocked.contains(id) {
            self.unlocked.insert(id.to_string());
            true
        } else {
            false
        }
    }

    /// Record a win.
    pub fn record_win(&mut self, opening: Option<&str>) {
        self.total_wins += 1;
        self.current_streak += 1;
        if self.current_streak > self.longest_streak {
            self.longest_streak = self.current_streak;
        }
        if let Some(name) = opening {
            *self.opening_wins.entry(name.to_string()).or_insert(0) += 1;
            self.unique_opening_wins.insert(name.to_string());
        }
    }

    /// Record a loss.
    pub fn record_loss(&mut self) {
        self.current_streak = 0;
    }

    /// Record a puzzle solved.
    pub fn record_puzzle(&mut self) {
        self.puzzles_solved += 1;
    }

    /// Check and unlock achievements based on current state.
    pub fn check_achievements(&mut self) -> Vec<&'static Achievement> {
        let mut newly_unlocked = Vec::new();

        for achievement in ACHIEVEMENTS.iter() {
            if self.is_unlocked(achievement.id) {
                continue;
            }

            let should_unlock = match achievement.id {
                "first_win" => self.total_wins >= 1,
                "win_streak_3" => self.current_streak >= 3,
                "win_streak_5" => self.current_streak >= 5,
                "win_streak_10" => self.current_streak >= 10,
                "win_10_games" => self.total_wins >= 10,
                "win_50_games" => self.total_wins >= 50,
                "puzzle_first" => self.puzzles_solved >= 1,
                "puzzle_10" => self.puzzles_solved >= 10,
                "puzzle_50" => self.puzzles_solved >= 50,
                "opening_diversity" => self.unique_opening_wins.len() >= 5,
                _ => false,
            };

            if should_unlock && self.try_unlock(achievement.id) {
                newly_unlocked.push(achievement);
            }
        }

        newly_unlocked
    }

    /// Get the number of unlocked achievements.
    pub fn unlocked_count(&self) -> usize {
        self.unlocked.len()
    }

    /// Get the total number of achievements.
    pub fn total_count(&self) -> usize {
        ACHIEVEMENTS.len()
    }

    /// Get progress as a percentage.
    pub fn progress_percentage(&self) -> f32 {
        if ACHIEVEMENTS.is_empty() {
            return 0.0;
        }
        (self.unlocked.len() as f32 / ACHIEVEMENTS.len() as f32) * 100.0
    }

    /// Get all achievements.
    pub fn all_achievements() -> &'static [Achievement] {
        ACHIEVEMENTS
    }
}

/// System to check achievements after game events.
pub fn check_achievements_system(
    mut tracker: ResMut<AchievementTracker>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let newly_unlocked = tracker.check_achievements();

    if tracker.notifications_enabled {
        for achievement in newly_unlocked {
            let msg = format!(
                "🏆 成就解锁: {} - {}",
                achievement.name_cn, achievement.description
            );
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let tracker = AchievementTracker::default();
        assert_eq!(tracker.unlocked_count(), 0);
        assert_eq!(tracker.total_wins, 0);
    }

    #[test]
    fn test_try_unlock() {
        let mut tracker = AchievementTracker::default();
        assert!(tracker.try_unlock("first_win"));
        assert!(!tracker.try_unlock("first_win")); // Already unlocked
        assert!(tracker.is_unlocked("first_win"));
    }

    #[test]
    fn test_record_win() {
        let mut tracker = AchievementTracker::default();
        tracker.record_win(Some("中炮"));
        assert_eq!(tracker.total_wins, 1);
        assert_eq!(tracker.current_streak, 1);
        assert_eq!(tracker.unique_opening_wins.len(), 1);
    }

    #[test]
    fn test_record_loss() {
        let mut tracker = AchievementTracker::default();
        tracker.record_win(None);
        tracker.record_win(None);
        assert_eq!(tracker.current_streak, 2);
        tracker.record_loss();
        assert_eq!(tracker.current_streak, 0);
    }

    #[test]
    fn test_check_first_win() {
        let mut tracker = AchievementTracker::default();
        tracker.record_win(None);
        let unlocked = tracker.check_achievements();
        assert!(unlocked.iter().any(|a| a.id == "first_win"));
    }

    #[test]
    fn test_check_win_streak() {
        let mut tracker = AchievementTracker::default();
        for _ in 0..3 {
            tracker.record_win(None);
        }
        let unlocked = tracker.check_achievements();
        assert!(unlocked.iter().any(|a| a.id == "win_streak_3"));
    }

    #[test]
    fn test_check_puzzles() {
        let mut tracker = AchievementTracker::default();
        tracker.record_puzzle();
        let unlocked = tracker.check_achievements();
        assert!(unlocked.iter().any(|a| a.id == "puzzle_first"));
    }

    #[test]
    fn test_progress_percentage() {
        let mut tracker = AchievementTracker::default();
        assert_eq!(tracker.progress_percentage(), 0.0);
        tracker.try_unlock("first_win");
        assert!(tracker.progress_percentage() > 0.0);
    }

    #[test]
    fn test_all_achievements_have_data() {
        for a in ACHIEVEMENTS {
            assert!(!a.id.is_empty());
            assert!(!a.name_cn.is_empty());
            assert!(!a.description.is_empty());
        }
    }

    #[test]
    fn test_longest_streak() {
        let mut tracker = AchievementTracker::default();
        for _ in 0..5 {
            tracker.record_win(None);
        }
        assert_eq!(tracker.longest_streak, 5);
        tracker.record_loss();
        for _ in 0..3 {
            tracker.record_win(None);
        }
        assert_eq!(tracker.longest_streak, 5); // Still 5
    }

    #[test]
    fn test_opening_tracking() {
        let mut tracker = AchievementTracker::default();
        tracker.record_win(Some("中炮"));
        tracker.record_win(Some("中炮"));
        tracker.record_win(Some("飞相"));
        assert_eq!(tracker.opening_wins.get("中炮"), Some(&2));
        assert_eq!(tracker.unique_opening_wins.len(), 2);
    }
}
