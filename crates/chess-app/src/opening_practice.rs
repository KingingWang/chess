//! Opening practice mode for training specific openings.
//!
//! Allows the player to select an opening line and practice playing
//! against the AI from the resulting position.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};

/// A practice opening definition.
#[derive(Debug, Clone)]
pub struct PracticeOpening {
    /// Chinese name.
    pub name_cn: String,
    /// English name.
    pub name_en: String,
    /// Opening moves in ICCS notation.
    pub moves: Vec<String>,
    /// Difficulty rating (1-5 stars).
    pub difficulty: u8,
    /// Description.
    pub description: String,
}

/// Resource managing opening practice mode.
#[derive(Resource, Debug)]
pub struct OpeningPractice {
    /// Whether practice mode is active.
    pub active: bool,
    /// Available openings for practice.
    pub openings: Vec<PracticeOpening>,
    /// Currently selected opening index.
    pub selected: Option<usize>,
    /// Number of successful practices per opening.
    pub success_counts: Vec<u32>,
    /// Number of attempts per opening.
    pub attempt_counts: Vec<u32>,
}

impl Default for OpeningPractice {
    fn default() -> Self {
        let openings = vec![
            PracticeOpening {
                name_cn: "中炮开局".to_string(),
                name_en: "Central Cannon".to_string(),
                moves: vec!["h2e2".to_string()],
                difficulty: 1,
                description: "最经典的开局，将炮移至中路，直指对方将帅".to_string(),
            },
            PracticeOpening {
                name_cn: "飞相局".to_string(),
                name_en: "Flying Elephant".to_string(),
                moves: vec!["c0e2".to_string()],
                difficulty: 2,
                description: "稳健的开局，先巩固防守再伺机进攻".to_string(),
            },
            PracticeOpening {
                name_cn: "仙人指路".to_string(),
                name_en: "Pawn Opening".to_string(),
                moves: vec!["c3c4".to_string()],
                difficulty: 2,
                description: "灵活的开局，试探对方应手后再决定策略".to_string(),
            },
            PracticeOpening {
                name_cn: "起马局".to_string(),
                name_en: "Horse Opening".to_string(),
                moves: vec!["b0c2".to_string()],
                difficulty: 2,
                description: "以马开局，变化多端，注重子力协调".to_string(),
            },
            PracticeOpening {
                name_cn: "仕角炮".to_string(),
                name_en: "Palace Corner Cannon".to_string(),
                moves: vec!["h2f2".to_string()],
                difficulty: 3,
                description: "将炮移至仕角，攻守兼备的布局".to_string(),
            },
            PracticeOpening {
                name_cn: "过宫炮".to_string(),
                name_en: "Cross Palace Cannon".to_string(),
                moves: vec!["h2a2".to_string()],
                difficulty: 3,
                description: "炮过宫至另一侧，侧重侧翼进攻".to_string(),
            },
            PracticeOpening {
                name_cn: "中炮对屏风马".to_string(),
                name_en: "Central Cannon vs Screen Horse".to_string(),
                moves: vec![
                    "h2e2".to_string(),
                    "h8g7".to_string(),
                    "h0g2".to_string(),
                    "i9h9".to_string(),
                    "b0a2".to_string(),
                ],
                difficulty: 3,
                description: "经典的中炮对屏风马体系，变化极其丰富".to_string(),
            },
            PracticeOpening {
                name_cn: "中炮对反宫马".to_string(),
                name_en: "Central Cannon vs Reverse Palace".to_string(),
                moves: vec![
                    "h2e2".to_string(),
                    "b7c7".to_string(),
                    "h0g2".to_string(),
                    "b9c7".to_string(),
                    "b0a2".to_string(),
                ],
                difficulty: 4,
                description: "反宫马是对抗中炮的重要体系，攻防转换激烈".to_string(),
            },
        ];

        let count = openings.len();
        Self {
            active: false,
            openings,
            selected: None,
            success_counts: vec![0; count],
            attempt_counts: vec![0; count],
        }
    }
}

impl OpeningPractice {
    /// Get the selected opening.
    pub fn selected_opening(&self) -> Option<&PracticeOpening> {
        self.selected.and_then(|i| self.openings.get(i))
    }

    /// Select an opening by index.
    pub fn select(&mut self, idx: usize) {
        if idx < self.openings.len() {
            self.selected = Some(idx);
        }
    }

    /// Record a practice result.
    pub fn record_result(&mut self, success: bool) {
        if let Some(idx) = self.selected {
            self.attempt_counts[idx] += 1;
            if success {
                self.success_counts[idx] += 1;
            }
        }
    }

    /// Get success rate for an opening.
    pub fn success_rate(&self, idx: usize) -> f32 {
        if idx >= self.openings.len() || self.attempt_counts[idx] == 0 {
            return 0.0;
        }
        (self.success_counts[idx] as f32 / self.attempt_counts[idx] as f32) * 100.0
    }

    /// Get the overall statistics.
    pub fn total_stats(&self) -> (u32, u32) {
        let total_attempts: u32 = self.attempt_counts.iter().sum();
        let total_successes: u32 = self.success_counts.iter().sum();
        (total_attempts, total_successes)
    }

    /// Reset all statistics.
    pub fn reset_stats(&mut self) {
        for c in self.success_counts.iter_mut() {
            *c = 0;
        }
        for c in self.attempt_counts.iter_mut() {
            *c = 0;
        }
    }

    /// Start practice mode.
    pub fn start(&mut self) {
        self.active = true;
    }

    /// Stop practice mode.
    pub fn stop(&mut self) {
        self.active = false;
        self.selected = None;
    }
}

/// Toggle opening practice mode.
pub fn toggle_opening_practice(
    keys: Res<ButtonInput<KeyCode>>,
    mut practice: ResMut<OpeningPractice>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        if practice.active {
            practice.stop();
            crate::toast::spawn_toast(&mut commands, &fonts, "开局练习已关闭");
        } else {
            practice.start();
            let count = practice.openings.len();
            crate::toast::spawn_toast(
                &mut commands,
                &fonts,
                &format!("开局练习: {}个开局可选", count),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_openings() {
        let practice = OpeningPractice::default();
        assert!(!practice.openings.is_empty());
        assert!(practice.openings.len() >= 5);
    }

    #[test]
    fn test_select_opening() {
        let mut practice = OpeningPractice::default();
        practice.select(0);
        assert_eq!(practice.selected, Some(0));
        assert!(practice.selected_opening().is_some());
    }

    #[test]
    fn test_select_invalid() {
        let mut practice = OpeningPractice::default();
        practice.select(999);
        assert_eq!(practice.selected, None);
    }

    #[test]
    fn test_record_result() {
        let mut practice = OpeningPractice::default();
        practice.select(0);

        practice.record_result(true);
        practice.record_result(true);
        practice.record_result(false);

        assert_eq!(practice.attempt_counts[0], 3);
        assert_eq!(practice.success_counts[0], 2);
    }

    #[test]
    fn test_success_rate() {
        let mut practice = OpeningPractice::default();
        practice.select(0);

        assert_eq!(practice.success_rate(0), 0.0); // No attempts

        practice.record_result(true);
        assert_eq!(practice.success_rate(0), 100.0);

        practice.record_result(false);
        assert_eq!(practice.success_rate(0), 50.0);
    }

    #[test]
    fn test_total_stats() {
        let mut practice = OpeningPractice::default();
        practice.select(0);
        practice.record_result(true);
        practice.select(1);
        practice.record_result(false);

        let (attempts, successes) = practice.total_stats();
        assert_eq!(attempts, 2);
        assert_eq!(successes, 1);
    }

    #[test]
    fn test_reset_stats() {
        let mut practice = OpeningPractice::default();
        practice.select(0);
        practice.record_result(true);

        practice.reset_stats();
        let (attempts, successes) = practice.total_stats();
        assert_eq!(attempts, 0);
        assert_eq!(successes, 0);
    }

    #[test]
    fn test_start_stop() {
        let mut practice = OpeningPractice::default();
        practice.start();
        assert!(practice.active);

        practice.stop();
        assert!(!practice.active);
        assert!(practice.selected.is_none());
    }

    #[test]
    fn test_opening_data() {
        let practice = OpeningPractice::default();
        for opening in &practice.openings {
            assert!(!opening.name_cn.is_empty());
            assert!(!opening.name_en.is_empty());
            assert!(!opening.moves.is_empty());
            assert!(opening.difficulty >= 1 && opening.difficulty <= 5);
            assert!(!opening.description.is_empty());
        }
    }
}
