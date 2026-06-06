//! Move quality classification based on engine evaluation.
//!
//! After each move, compares the evaluation before and after to classify
//! the move as: brilliant, good, inaccuracy, mistake, or blunder.

use bevy::prelude::*;

use crate::ai_bridge::SearchInfoResource;
use crate::app_state::{CoreGame, UiFonts};

/// Classification of a move's quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveQuality {
    /// Excellent move, significantly improves position.
    Brilliant,
    /// Good move, improves or maintains position.
    Good,
    /// Slightly suboptimal move.
    Inaccuracy,
    /// Clearly suboptimal move.
    Mistake,
    /// Severely worsens position.
    Blunder,
}

impl MoveQuality {
    /// Get the symbol for this quality.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Brilliant => "!!",
            Self::Good => "!",
            Self::Inaccuracy => "?!",
            Self::Mistake => "?",
            Self::Blunder => "??",
        }
    }

    /// Get the Chinese label.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Brilliant => "妙手",
            Self::Good => "好棋",
            Self::Inaccuracy => "疑问",
            Self::Mistake => "错着",
            Self::Blunder => "败着",
        }
    }

    /// Get the color for displaying this quality.
    pub fn color(&self) -> Color {
        match self {
            Self::Brilliant => Color::srgb(0.0, 0.9, 1.0), // Cyan
            Self::Good => Color::srgb(0.4, 0.85, 0.4),     // Green
            Self::Inaccuracy => Color::srgb(0.95, 0.85, 0.3), // Yellow
            Self::Mistake => Color::srgb(0.95, 0.55, 0.2), // Orange
            Self::Blunder => Color::srgb(0.95, 0.2, 0.2),  // Red
        }
    }

    /// Classify a move based on the evaluation difference.
    ///
    /// `eval_before` and `eval_after` are in centipawns from the same
    /// perspective (positive = Red advantage).
    pub fn classify(eval_before: i32, eval_after: i32, side_to_move_is_red: bool) -> Self {
        // Normalize: we want the change from the mover's perspective.
        // If Red moved, a positive delta is good; if Black moved, a negative delta is good.
        let delta = if side_to_move_is_red {
            eval_after - eval_before
        } else {
            eval_before - eval_after
        };

        // Thresholds in centipawns
        match delta {
            d if d >= 100 => Self::Brilliant,
            d if d >= -30 => Self::Good,
            d if d >= -80 => Self::Inaccuracy,
            d if d >= -200 => Self::Mistake,
            _ => Self::Blunder,
        }
    }
}

/// Resource tracking move quality classifications.
#[derive(Resource, Debug, Clone)]
pub struct MoveQualityTracker {
    /// Quality of each move in the game.
    pub qualities: Vec<MoveQuality>,
    /// Evaluation before each move (centipawns, Red perspective).
    pub evals_before: Vec<i32>,
    /// Evaluation after each move.
    pub evals_after: Vec<i32>,
    /// Last known evaluation.
    pub last_eval: i32,
    /// Whether the last move was by Red.
    pub last_side_was_red: bool,
    /// Count of each quality.
    pub counts: [u32; 5],
}

impl Default for MoveQualityTracker {
    fn default() -> Self {
        Self {
            qualities: Vec::new(),
            evals_before: Vec::new(),
            evals_after: Vec::new(),
            last_eval: 0,
            last_side_was_red: true,
            counts: [0; 5],
        }
    }
}

impl MoveQualityTracker {
    /// Record a new move with its evaluation.
    pub fn record_move(&mut self, eval_after: i32, side_to_move_is_red: bool) {
        let quality = MoveQuality::classify(self.last_eval, eval_after, side_to_move_is_red);

        self.evals_before.push(self.last_eval);
        self.evals_after.push(eval_after);
        self.qualities.push(quality);

        // Update counts
        let idx = match quality {
            MoveQuality::Brilliant => 0,
            MoveQuality::Good => 1,
            MoveQuality::Inaccuracy => 2,
            MoveQuality::Mistake => 3,
            MoveQuality::Blunder => 4,
        };
        self.counts[idx] += 1;

        // Update last eval for next move
        self.last_eval = eval_after;
        self.last_side_was_red = !side_to_move_is_red;
    }

    /// Get the quality of the last move.
    pub fn last_quality(&self) -> Option<MoveQuality> {
        self.qualities.last().copied()
    }

    /// Get quality by move index.
    pub fn quality_at(&self, idx: usize) -> Option<MoveQuality> {
        self.qualities.get(idx).copied()
    }

    /// Get total move count.
    pub fn move_count(&self) -> usize {
        self.qualities.len()
    }

    /// Reset all tracking.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        format!(
            "妙:{} 好:{} 疑:{} 错:{} 败:{}",
            self.counts[0], self.counts[1], self.counts[2], self.counts[3], self.counts[4]
        )
    }
}

/// System to classify moves when new search info is available.
pub fn classify_moves(
    core: Res<CoreGame>,
    search_info: Res<SearchInfoResource>,
    mut tracker: ResMut<MoveQualityTracker>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let history_len = core.game.history_len();

    // Only classify if there's a new move and search info is available
    if history_len > tracker.move_count() {
        if let Some(info) = &search_info.latest {
            let eval = info.score;
            // Determine whose move it was
            let side_is_red = history_len % 2 == 1; // Odd ply = Red's move
            tracker.record_move(eval, side_is_red);

            // Show toast for notable moves
            if let Some(q) = tracker.last_quality() {
                match q {
                    MoveQuality::Brilliant => {
                        let msg = format!("{} {}!", q.label_cn(), q.symbol());
                        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    }
                    MoveQuality::Blunder => {
                        let msg = format!("{} {}", q.label_cn(), q.symbol());
                        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// System to reset move quality tracking when a new game starts.
pub fn reset_move_quality(core: Res<CoreGame>, mut tracker: ResMut<MoveQualityTracker>) {
    if core.game.history_len() == 0 && tracker.move_count() > 0 {
        tracker.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_brilliant() {
        let q = MoveQuality::classify(0, 150, true);
        assert_eq!(q, MoveQuality::Brilliant);
    }

    #[test]
    fn test_classify_good() {
        let q = MoveQuality::classify(0, 10, true);
        assert_eq!(q, MoveQuality::Good);
    }

    #[test]
    fn test_classify_inaccuracy() {
        let q = MoveQuality::classify(0, -50, true);
        assert_eq!(q, MoveQuality::Inaccuracy);
    }

    #[test]
    fn test_classify_mistake() {
        let q = MoveQuality::classify(0, -150, true);
        assert_eq!(q, MoveQuality::Mistake);
    }

    #[test]
    fn test_classify_blunder() {
        let q = MoveQuality::classify(0, -300, true);
        assert_eq!(q, MoveQuality::Blunder);
    }

    #[test]
    fn test_classify_black_perspective() {
        // For Black, a positive eval change from Black's perspective is good
        let q = MoveQuality::classify(100, -50, false);
        assert_eq!(q, MoveQuality::Brilliant);
    }

    #[test]
    fn test_symbols() {
        assert_eq!(MoveQuality::Brilliant.symbol(), "!!");
        assert_eq!(MoveQuality::Good.symbol(), "!");
        assert_eq!(MoveQuality::Inaccuracy.symbol(), "?!");
        assert_eq!(MoveQuality::Mistake.symbol(), "?");
        assert_eq!(MoveQuality::Blunder.symbol(), "??");
    }

    #[test]
    fn test_labels_cn() {
        assert_eq!(MoveQuality::Brilliant.label_cn(), "妙手");
        assert_eq!(MoveQuality::Blunder.label_cn(), "败着");
    }

    #[test]
    fn test_tracker_record() {
        let mut tracker = MoveQualityTracker::default();
        tracker.record_move(100, true);
        assert_eq!(tracker.move_count(), 1);
        assert_eq!(tracker.last_quality(), Some(MoveQuality::Brilliant));
    }

    #[test]
    fn test_tracker_counts() {
        let mut tracker = MoveQualityTracker::default();
        tracker.record_move(100, true); // Brilliant
        tracker.record_move(10, false); // Good
        tracker.record_move(-50, true); // Inaccuracy
        assert_eq!(tracker.counts[0], 1); // Brilliant
        assert_eq!(tracker.counts[1], 1); // Good
        assert_eq!(tracker.counts[2], 1); // Inaccuracy
    }

    #[test]
    fn test_tracker_reset() {
        let mut tracker = MoveQualityTracker::default();
        tracker.record_move(100, true);
        assert!(tracker.move_count() > 0);
        tracker.reset();
        assert_eq!(tracker.move_count(), 0);
    }

    #[test]
    fn test_tracker_summary() {
        let mut tracker = MoveQualityTracker::default();
        tracker.record_move(100, true);
        tracker.record_move(10, false);
        let summary = tracker.summary();
        assert!(summary.contains("妙:1"));
        assert!(summary.contains("好:1"));
    }

    #[test]
    fn test_quality_at() {
        let mut tracker = MoveQualityTracker::default();
        tracker.record_move(100, true); // Move 0: Brilliant
        tracker.record_move(10, false); // Move 1: Good
        assert_eq!(tracker.quality_at(0), Some(MoveQuality::Brilliant));
        assert_eq!(tracker.quality_at(1), Some(MoveQuality::Good));
        assert_eq!(tracker.quality_at(5), None);
    }
}
