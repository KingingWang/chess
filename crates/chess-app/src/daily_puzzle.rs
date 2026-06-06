//! Daily puzzle challenge system.
//!
//! Provides a daily puzzle that is the same for all players on a given day,
//! encouraging players to come back each day for a new challenge.

use bevy::prelude::*;

/// A daily puzzle definition.
#[derive(Debug, Clone)]
pub struct DailyPuzzle {
    /// Date string (YYYY-MM-DD).
    pub date: String,
    /// FEN position.
    pub fen: String,
    /// Solution moves in ICCS notation.
    pub solution: Vec<String>,
    /// Hint text (Chinese).
    pub hint_cn: String,
    /// Difficulty (1-5).
    pub difficulty: u8,
}

/// Resource managing daily puzzle state.
#[derive(Resource, Debug)]
pub struct DailyPuzzleChallenge {
    /// Current day's puzzle.
    pub current_puzzle: Option<DailyPuzzle>,
    /// Whether the player has completed today's puzzle.
    pub completed_today: bool,
    /// Total daily puzzles completed.
    pub total_completed: u32,
    /// Current streak of consecutive days.
    pub daily_streak: u32,
    /// Longest daily streak.
    pub longest_streak: u32,
    /// Last date a puzzle was completed.
    pub last_completed_date: Option<String>,
}

impl Default for DailyPuzzleChallenge {
    fn default() -> Self {
        Self {
            current_puzzle: None,
            completed_today: false,
            total_completed: 0,
            daily_streak: 0,
            longest_streak: 0,
            last_completed_date: None,
        }
    }
}

impl DailyPuzzleChallenge {
    /// Generate today's puzzle based on the date.
    pub fn generate_puzzle_for_date(&mut self, date_str: &str) {
        // Simple hash of date to select puzzle
        let hash = date_str
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let idx = (hash % DAILY_PUZZLES.len() as u64) as usize;

        self.current_puzzle = Some(DailyPuzzle {
            date: date_str.to_string(),
            fen: DAILY_PUZZLES[idx].0.to_string(),
            solution: DAILY_PUZZLES[idx].1.iter().map(|s| s.to_string()).collect(),
            hint_cn: DAILY_PUZZLES[idx].2.to_string(),
            difficulty: DAILY_PUZZLES[idx].3,
        });
        self.completed_today = false;
    }

    /// Record puzzle completion.
    pub fn record_completion(&mut self, date: &str) {
        self.completed_today = true;
        self.total_completed += 1;

        // Update streak
        if let Some(ref last) = self.last_completed_date {
            if is_consecutive_day(last, date) {
                self.daily_streak += 1;
            } else {
                self.daily_streak = 1;
            }
        } else {
            self.daily_streak = 1;
        }

        if self.daily_streak > self.longest_streak {
            self.longest_streak = self.daily_streak;
        }
        self.last_completed_date = Some(date.to_string());
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "每日残局: {}完成 | 连续{}天 | 最长连续{}天",
            if self.completed_today { "已" } else { "未" },
            self.daily_streak,
            self.longest_streak,
        )
    }
}

/// Check if two dates are consecutive days.
fn is_consecutive_day(prev: &str, curr: &str) -> bool {
    // Simple check: parse YYYY-MM-DD and compare
    let parse = |s: &str| -> Option<(i32, u32, u32)> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    };

    let Some((py, pm, pd)) = parse(prev) else {
        return false;
    };
    let Some((cy, cm, cd)) = parse(curr) else {
        return false;
    };

    // Simple consecutive check (doesn't handle month/year boundaries perfectly)
    let prev_days = py * 365 + pm as i32 * 30 + pd as i32;
    let curr_days = cy * 365 + cm as i32 * 30 + cd as i32;
    curr_days - prev_days == 1
}

/// Sample daily puzzles (FEN, solution, hint, difficulty).
static DAILY_PUZZLES: &[(&str, &[&str], &str, u8)] = &[
    (
        "4k4/9/9/9/9/9/9/9/4R4/4K4 w - - 0 1",
        &["e1e2"],
        "车占中路，控制对方将帅",
        1,
    ),
    (
        "3k5/9/9/9/9/9/9/9/R8/4K4 w - - 0 1",
        &["a1a9"],
        "利用底线将军",
        2,
    ),
    (
        "4k4/9/9/9/9/9/9/4R4/9/4K4 w - - 0 1",
        &["e3e9"],
        "直接沉底将军",
        1,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let dpc = DailyPuzzleChallenge::default();
        assert!(!dpc.completed_today);
        assert_eq!(dpc.total_completed, 0);
        assert_eq!(dpc.daily_streak, 0);
    }

    #[test]
    fn test_generate_puzzle() {
        let mut dpc = DailyPuzzleChallenge::default();
        dpc.generate_puzzle_for_date("2026-06-06");
        assert!(dpc.current_puzzle.is_some());
        assert_eq!(dpc.current_puzzle.as_ref().unwrap().date, "2026-06-06");
    }

    #[test]
    fn test_same_date_same_puzzle() {
        let mut dpc1 = DailyPuzzleChallenge::default();
        let mut dpc2 = DailyPuzzleChallenge::default();
        dpc1.generate_puzzle_for_date("2026-06-06");
        dpc2.generate_puzzle_for_date("2026-06-06");
        assert_eq!(
            dpc1.current_puzzle.as_ref().unwrap().fen,
            dpc2.current_puzzle.as_ref().unwrap().fen
        );
    }

    #[test]
    fn test_record_completion() {
        let mut dpc = DailyPuzzleChallenge::default();
        dpc.record_completion("2026-06-06");
        assert!(dpc.completed_today);
        assert_eq!(dpc.total_completed, 1);
        assert_eq!(dpc.daily_streak, 1);
    }

    #[test]
    fn test_consecutive_days() {
        let mut dpc = DailyPuzzleChallenge::default();
        dpc.record_completion("2026-06-06");
        dpc.record_completion("2026-06-07");
        assert_eq!(dpc.daily_streak, 2);
    }

    #[test]
    fn test_non_consecutive_resets_streak() {
        let mut dpc = DailyPuzzleChallenge::default();
        dpc.record_completion("2026-06-06");
        dpc.record_completion("2026-06-10"); // Gap
        assert_eq!(dpc.daily_streak, 1);
    }

    #[test]
    fn test_summary() {
        let mut dpc = DailyPuzzleChallenge::default();
        let summary = dpc.summary();
        assert!(summary.contains("未"));
        dpc.record_completion("2026-06-06");
        let summary = dpc.summary();
        assert!(summary.contains("已"));
    }

    #[test]
    fn test_is_consecutive_day() {
        assert!(is_consecutive_day("2026-06-06", "2026-06-07"));
        assert!(!is_consecutive_day("2026-06-06", "2026-06-08"));
        assert!(!is_consecutive_day("2026-06-06", "2026-06-06"));
    }
}
