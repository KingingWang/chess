//! Move time tracking and display.
//!
//! Records the time spent on each move and provides data for
//! the history panel to show per-move time information.

use bevy::prelude::*;
use std::time::Instant;

use crate::app_state::{CoreGame, MoveTimeHistory};

/// Resource tracking per-move timing data.
#[derive(Resource, Debug)]
pub struct MoveTimeDisplay {
    /// Whether the time display is enabled.
    pub enabled: bool,
    /// Timestamp when the current turn started.
    pub turn_start: Option<Instant>,
    /// Time spent on each move (in seconds).
    pub move_times: Vec<f32>,
    /// Total time for Red (seconds).
    pub red_total_time: f32,
    /// Total time for Black (seconds).
    pub black_total_time: f32,
    /// Average time per move for Red.
    pub red_avg_time: f32,
    /// Average time per move for Black.
    pub black_avg_time: f32,
    /// Longest time spent on a single move.
    pub max_time: f32,
    /// Index of the move that took the longest.
    pub max_time_move_idx: usize,
}

impl Default for MoveTimeDisplay {
    fn default() -> Self {
        Self {
            enabled: true,
            turn_start: Some(Instant::now()),
            move_times: Vec::new(),
            red_total_time: 0.0,
            black_total_time: 0.0,
            red_avg_time: 0.0,
            black_avg_time: 0.0,
            max_time: 0.0,
            max_time_move_idx: 0,
        }
    }
}

impl MoveTimeDisplay {
    /// Record the time for the current move.
    pub fn record_move_time(&mut self, time_secs: f32) {
        self.move_times.push(time_secs);

        let move_idx = self.move_times.len() - 1;

        // Update totals
        if move_idx.is_multiple_of(2) {
            // Red's move
            self.red_total_time += time_secs;
            let red_moves = (move_idx / 2 + 1) as f32;
            self.red_avg_time = self.red_total_time / red_moves;
        } else {
            // Black's move
            self.black_total_time += time_secs;
            let black_moves = move_idx.div_ceil(2) as f32;
            self.black_avg_time = self.black_total_time / black_moves;
        }

        // Update max time
        if time_secs > self.max_time {
            self.max_time = time_secs;
            self.max_time_move_idx = move_idx;
        }
    }

    /// Get the time for a specific move.
    pub fn time_at(&self, idx: usize) -> Option<f32> {
        self.move_times.get(idx).copied()
    }

    /// Format a time in seconds as a human-readable string.
    pub fn format_time(secs: f32) -> String {
        if secs < 60.0 {
            format!("{:.1}s", secs)
        } else if secs < 3600.0 {
            let mins = (secs / 60.0).floor() as u32;
            let remaining = secs - (mins as f32 * 60.0);
            format!("{}:{:04.1}", mins, remaining)
        } else {
            let hours = (secs / 3600.0).floor() as u32;
            let mins = ((secs % 3600.0) / 60.0).floor() as u32;
            format!("{}:{:02}h", hours, mins)
        }
    }

    /// Get a summary of time statistics.
    pub fn summary(&self) -> String {
        format!(
            "红方用时: {} | 黑方用时: {} | 最长思考: 第{}步 ({})",
            Self::format_time(self.red_total_time),
            Self::format_time(self.black_total_time),
            self.max_time_move_idx + 1,
            Self::format_time(self.max_time),
        )
    }

    /// Reset all timing data.
    pub fn reset(&mut self) {
        self.move_times.clear();
        self.red_total_time = 0.0;
        self.black_total_time = 0.0;
        self.red_avg_time = 0.0;
        self.black_avg_time = 0.0;
        self.max_time = 0.0;
        self.max_time_move_idx = 0;
        self.turn_start = Some(Instant::now());
    }

    /// Start timing a new turn.
    pub fn start_turn(&mut self) {
        self.turn_start = Some(Instant::now());
    }

    /// Get elapsed time for the current turn.
    pub fn elapsed(&self) -> f32 {
        self.turn_start
            .map(|start| start.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }
}

/// System to track move timing when moves are made.
pub fn track_move_time(
    core: Res<CoreGame>,
    mut display: ResMut<MoveTimeDisplay>,
    mut move_times: ResMut<MoveTimeHistory>,
) {
    let history_len = core.game.history_len();

    // If a new move was made
    if history_len > display.move_times.len() {
        let elapsed = display.elapsed();
        display.record_move_time(elapsed);
        move_times.0.push(elapsed);
        display.start_turn();
    }
}

/// System to reset timing when game is reset.
pub fn reset_move_times(
    core: Res<CoreGame>,
    mut display: ResMut<MoveTimeDisplay>,
    mut move_times: ResMut<MoveTimeHistory>,
) {
    if core.game.history_len() == 0 && !display.move_times.is_empty() {
        display.reset();
        move_times.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_seconds() {
        assert_eq!(MoveTimeDisplay::format_time(5.3), "5.3s");
        assert_eq!(MoveTimeDisplay::format_time(0.1), "0.1s");
        assert_eq!(MoveTimeDisplay::format_time(59.9), "59.9s");
    }

    #[test]
    fn test_format_time_minutes() {
        assert_eq!(MoveTimeDisplay::format_time(90.0), "1:30.0");
        assert_eq!(MoveTimeDisplay::format_time(125.5), "2:05.5");
    }

    #[test]
    fn test_format_time_hours() {
        assert_eq!(MoveTimeDisplay::format_time(3661.0), "1:01h");
    }

    #[test]
    fn test_record_red_move() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(5.0); // Red's first move
        assert_eq!(display.move_times.len(), 1);
        assert_eq!(display.red_total_time, 5.0);
        assert_eq!(display.red_avg_time, 5.0);
        assert_eq!(display.black_total_time, 0.0);
    }

    #[test]
    fn test_record_both_sides() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(5.0); // Red
        display.record_move_time(3.0); // Black
        display.record_move_time(8.0); // Red
        display.record_move_time(2.0); // Black

        assert_eq!(display.red_total_time, 13.0); // 5 + 8
        assert_eq!(display.black_total_time, 5.0); // 3 + 2
        assert_eq!(display.red_avg_time, 6.5); // 13 / 2
        assert_eq!(display.black_avg_time, 2.5); // 5 / 2
    }

    #[test]
    fn test_max_time_tracking() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(5.0);
        display.record_move_time(3.0);
        display.record_move_time(15.0); // This is the longest
        display.record_move_time(2.0);

        assert_eq!(display.max_time, 15.0);
        assert_eq!(display.max_time_move_idx, 2);
    }

    #[test]
    fn test_time_at() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(5.0);
        display.record_move_time(3.0);

        assert_eq!(display.time_at(0), Some(5.0));
        assert_eq!(display.time_at(1), Some(3.0));
        assert_eq!(display.time_at(2), None);
    }

    #[test]
    fn test_summary() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(10.0);
        display.record_move_time(5.0);

        let summary = display.summary();
        assert!(summary.contains("红方"));
        assert!(summary.contains("黑方"));
    }

    #[test]
    fn test_reset() {
        let mut display = MoveTimeDisplay::default();
        display.record_move_time(5.0);
        display.record_move_time(3.0);

        display.reset();
        assert!(display.move_times.is_empty());
        assert_eq!(display.red_total_time, 0.0);
        assert_eq!(display.black_total_time, 0.0);
    }

    #[test]
    fn test_elapsed() {
        let mut display = MoveTimeDisplay::default();
        display.turn_start = Some(Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(50));
        let elapsed = display.elapsed();
        assert!(elapsed >= 0.04); // Allow some slack
        assert!(elapsed < 1.0);
    }
}
