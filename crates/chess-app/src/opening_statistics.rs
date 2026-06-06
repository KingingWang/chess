//! Opening statistics tracker.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OpeningStat {
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub avg_rating: Option<u32>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct OpeningStatistics {
    pub stats: HashMap<String, OpeningStat>,
    pub visible: bool,
}

impl OpeningStatistics {
    pub fn record(&mut self, opening: &str, won: bool) {
        let entry = self
            .stats
            .entry(opening.to_string())
            .or_insert(OpeningStat {
                games: 0,
                wins: 0,
                losses: 0,
                draws: 0,
                avg_rating: None,
            });
        entry.games += 1;
        if won {
            entry.wins += 1;
        } else {
            entry.losses += 1;
        }
    }
    pub fn win_rate(&self, opening: &str) -> f32 {
        self.stats
            .get(opening)
            .map(|s| {
                if s.games == 0 {
                    0.0
                } else {
                    s.wins as f32 / s.games as f32 * 100.0
                }
            })
            .unwrap_or(0.0)
    }
    pub fn top_openings(&self, n: usize) -> Vec<(&str, f32)> {
        let mut sorted: Vec<_> = self
            .stats
            .iter()
            .map(|(name, s)| {
                (
                    name.as_str(),
                    if s.games == 0 {
                        0.0
                    } else {
                        s.wins as f32 / s.games as f32 * 100.0
                    },
                )
            })
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(n).collect()
    }
}

pub fn toggle_opening_stats(
    keys: Res<ButtonInput<KeyCode>>,
    mut os: ResMut<OpeningStatistics>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyU) {
        os.visible = !os.visible;
        let msg = if os.visible {
            format!("开局统计: {}种开局", os.stats.len())
        } else {
            "开局统计已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let mut os = OpeningStatistics::default();
        os.record("中炮", true);
        os.record("中炮", false);
        assert_eq!(os.stats["中炮"].games, 2);
        assert_eq!(os.win_rate("中炮"), 50.0);
    }
    #[test]
    fn test_top() {
        let mut os = OpeningStatistics::default();
        for _ in 0..5 {
            os.record("中炮", true);
        }
        for _ in 0..2 {
            os.record("飞相", true);
        }
        for _ in 0..3 {
            os.record("飞相", false);
        }
        let top = os.top_openings(2);
        assert_eq!(top.len(), 2);
    }
}
