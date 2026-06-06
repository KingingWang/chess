//! Chess clock / time control for Xiangqi games.
//!
//! Supports three time control modes:
//! - **No limit**: unlimited thinking time (casual play).
//! - **Fixed per move**: each player gets a fixed amount per move (e.g., 30s).
//! - **Total + increment**: each player has a total budget with optional
//!   per-move increment (Fischer-style).
//!
//! The clock is decoupled from the game rules — it only tracks elapsed time
//! and reports whether a player has flagged (run out of time).

use crate::piece::Color;
use std::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Time control configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimeControl {
    /// No time limit.
    #[default]
    Unlimited,
    /// Fixed time per move (resets each turn).
    PerMove { seconds: u32 },
    /// Total time with optional increment per move (Fischer).
    Fischer {
        total_seconds: u32,
        increment_seconds: u32,
    },
}

impl TimeControl {
    /// Common presets.
    pub const BULLET_1_0: TimeControl = TimeControl::Fischer {
        total_seconds: 60,
        increment_seconds: 0,
    };
    pub const BULLET_1_1: TimeControl = TimeControl::Fischer {
        total_seconds: 60,
        increment_seconds: 1,
    };
    pub const BLITZ_3_0: TimeControl = TimeControl::Fischer {
        total_seconds: 180,
        increment_seconds: 0,
    };
    pub const BLITZ_3_2: TimeControl = TimeControl::Fischer {
        total_seconds: 180,
        increment_seconds: 2,
    };
    pub const BLITZ_5_3: TimeControl = TimeControl::Fischer {
        total_seconds: 300,
        increment_seconds: 3,
    };
    pub const RAPID_10_5: TimeControl = TimeControl::Fischer {
        total_seconds: 600,
        increment_seconds: 5,
    };
    pub const RAPID_15_10: TimeControl = TimeControl::Fischer {
        total_seconds: 900,
        increment_seconds: 10,
    };
    pub const CLASSICAL_30_0: TimeControl = TimeControl::Fischer {
        total_seconds: 1800,
        increment_seconds: 0,
    };
    pub const CLASSICAL_30_20: TimeControl = TimeControl::Fischer {
        total_seconds: 1800,
        increment_seconds: 20,
    };
    pub const PER_MOVE_30: TimeControl = TimeControl::PerMove { seconds: 30 };
    pub const PER_MOVE_60: TimeControl = TimeControl::PerMove { seconds: 60 };

    /// Get a human-readable label for this time control.
    pub fn label(&self) -> String {
        match self {
            TimeControl::Unlimited => "无限时".to_string(),
            TimeControl::PerMove { seconds } => format!("每步{}秒", seconds),
            TimeControl::Fischer {
                total_seconds,
                increment_seconds,
            } => {
                let mins = total_seconds / 60;
                if *increment_seconds > 0 {
                    format!("{}+{}", mins, increment_seconds)
                } else {
                    format!("{}分钟", mins)
                }
            }
        }
    }

    /// Get all available presets.
    pub fn all_presets() -> Vec<TimeControl> {
        vec![
            TimeControl::Unlimited,
            TimeControl::BULLET_1_0,
            TimeControl::BULLET_1_1,
            TimeControl::BLITZ_3_0,
            TimeControl::BLITZ_3_2,
            TimeControl::BLITZ_5_3,
            TimeControl::RAPID_10_5,
            TimeControl::RAPID_15_10,
            TimeControl::CLASSICAL_30_0,
            TimeControl::CLASSICAL_30_20,
            TimeControl::PER_MOVE_30,
            TimeControl::PER_MOVE_60,
        ]
    }
}

/// Live game clock state for both players.
#[derive(Debug, Clone)]
pub struct GameClock {
    /// Time control being used.
    pub time_control: TimeControl,
    /// Remaining time for Red.
    pub red_remaining: Duration,
    /// Remaining time for Black.
    pub black_remaining: Duration,
    /// Whose clock is currently ticking (None if paused).
    active_side: Option<Color>,
    /// When the currently active clock started ticking.
    tick_start: Option<Instant>,
}

impl GameClock {
    /// Create a new clock with the given time control.
    pub fn new(tc: TimeControl) -> Self {
        let initial = match tc {
            TimeControl::Unlimited => Duration::from_secs(u64::MAX / 2),
            TimeControl::PerMove { seconds } => Duration::from_secs(seconds as u64),
            TimeControl::Fischer {
                total_seconds,
                increment_seconds: _,
            } => Duration::from_secs(total_seconds as u64),
        };
        GameClock {
            time_control: tc,
            red_remaining: initial,
            black_remaining: initial,
            active_side: None,
            tick_start: None,
        }
    }

    /// Start the clock for a side (called when a game begins or after a move).
    pub fn start(&mut self, side: Color) {
        // If the clock was already running for a different side, stop it first.
        self.stop_current();
        self.active_side = Some(side);
        self.tick_start = Some(Instant::now());
    }

    /// Stop the current clock (e.g., game paused, ended).
    pub fn stop_current(&mut self) {
        if let (Some(side), Some(start)) = (self.active_side, self.tick_start) {
            let elapsed = start.elapsed();
            let remaining = self.remaining_mut(side);
            *remaining = remaining.saturating_sub(elapsed);
        }
        self.active_side = None;
        self.tick_start = None;
    }

    /// Called after a move is made. Stops the mover's clock, applies increment
    /// (if Fischer), resets (if per-move), and starts the opponent's clock.
    pub fn move_made(&mut self, mover: Color) {
        self.stop_current();

        // Apply time control rules.
        match self.time_control {
            TimeControl::Unlimited => {}
            TimeControl::PerMove { seconds } => {
                // Reset the mover's clock for their next turn.
                *self.remaining_mut(mover) = Duration::from_secs(seconds as u64);
            }
            TimeControl::Fischer {
                increment_seconds, ..
            } => {
                // Add increment after the move.
                *self.remaining_mut(mover) += Duration::from_secs(increment_seconds as u64);
            }
        }

        // Start the opponent's clock.
        self.start(mover.opponent());
    }

    /// Get the remaining time for a side, accounting for currently elapsed time.
    pub fn remaining(&self, side: Color) -> Duration {
        let base = match side {
            Color::Red => self.red_remaining,
            Color::Black => self.black_remaining,
        };
        if self.active_side == Some(side) {
            if let Some(start) = self.tick_start {
                return base.saturating_sub(start.elapsed());
            }
        }
        base
    }

    /// Has the given side run out of time?
    pub fn is_flagged(&self, side: Color) -> bool {
        self.remaining(side) == Duration::ZERO
    }

    /// Is the clock currently running?
    pub fn is_running(&self) -> bool {
        self.active_side.is_some()
    }

    /// Which side's clock is ticking?
    pub fn active_side(&self) -> Option<Color> {
        self.active_side
    }

    /// Format remaining time as "MM:SS" string.
    pub fn format_time(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if total_secs >= 3600 {
            let hours = mins / 60;
            let mins = mins % 60;
            format!("{hours}:{mins:02}:{secs:02}")
        } else {
            format!("{mins}:{secs:02}")
        }
    }

    fn remaining_mut(&mut self, side: Color) -> &mut Duration {
        match side {
            Color::Red => &mut self.red_remaining,
            Color::Black => &mut self.black_remaining,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn unlimited_never_flags() {
        let clock = GameClock::new(TimeControl::Unlimited);
        assert!(!clock.is_flagged(Color::Red));
        assert!(!clock.is_flagged(Color::Black));
    }

    #[test]
    fn fischer_starts_with_total_time() {
        let clock = GameClock::new(TimeControl::Fischer {
            total_seconds: 300,
            increment_seconds: 5,
        });
        assert_eq!(clock.remaining(Color::Red), Duration::from_secs(300));
        assert_eq!(clock.remaining(Color::Black), Duration::from_secs(300));
    }

    #[test]
    fn per_move_resets_after_move() {
        let mut clock = GameClock::new(TimeControl::PerMove { seconds: 30 });
        clock.start(Color::Red);
        sleep(Duration::from_millis(10));
        clock.move_made(Color::Red);
        // Red's clock should be reset to 30s.
        let remaining = clock.remaining(Color::Red).as_secs();
        assert_eq!(remaining, 30);
    }

    #[test]
    fn fischer_adds_increment() {
        let mut clock = GameClock::new(TimeControl::Fischer {
            total_seconds: 60,
            increment_seconds: 5,
        });
        clock.start(Color::Red);
        sleep(Duration::from_millis(10));
        clock.move_made(Color::Red);
        // Red should have ~60 + 5 - elapsed ≈ 64-65 seconds.
        let remaining = clock.remaining(Color::Red);
        assert!(remaining > Duration::from_secs(64));
        assert!(remaining <= Duration::from_secs(65));
    }

    #[test]
    fn clock_ticks_down() {
        let mut clock = GameClock::new(TimeControl::Fischer {
            total_seconds: 60,
            increment_seconds: 0,
        });
        clock.start(Color::Red);
        sleep(Duration::from_millis(50));
        let remaining = clock.remaining(Color::Red);
        assert!(remaining < Duration::from_secs(60));
    }

    #[test]
    fn format_time_display() {
        assert_eq!(GameClock::format_time(Duration::from_secs(0)), "0:00");
        assert_eq!(GameClock::format_time(Duration::from_secs(65)), "1:05");
        assert_eq!(GameClock::format_time(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn test_time_control_labels() {
        assert_eq!(TimeControl::Unlimited.label(), "无限时");
        assert_eq!(TimeControl::PER_MOVE_30.label(), "每步30秒");
        assert_eq!(TimeControl::BLITZ_3_2.label(), "3+2");
        assert_eq!(TimeControl::RAPID_15_10.label(), "15+10");
        assert_eq!(TimeControl::CLASSICAL_30_0.label(), "30分钟");
    }

    #[test]
    fn test_all_presets() {
        let presets = TimeControl::all_presets();
        assert_eq!(presets.len(), 12);
        assert_eq!(presets[0], TimeControl::Unlimited);
    }

    #[test]
    fn bullet_time_control() {
        let clock = GameClock::new(TimeControl::BULLET_1_0);
        assert_eq!(clock.remaining(Color::Red), Duration::from_secs(60));
        assert_eq!(clock.remaining(Color::Black), Duration::from_secs(60));
    }
}
