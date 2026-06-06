//! Opening repertoire management for personal opening preparation.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RepertoireOpening {
    pub name: String,
    pub moves: Vec<String>,
    pub win_rate: f32,
    pub games_played: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct OpeningRepertoire {
    pub enabled: bool,
    pub openings: HashMap<String, RepertoireOpening>,
    pub current_side: bool, // true = red, false = black
}

impl Default for OpeningRepertoire {
    fn default() -> Self {
        Self {
            enabled: false,
            openings: HashMap::new(),
            current_side: true,
        }
    }
}

impl OpeningRepertoire {
    pub fn add_opening(&mut self, name: &str, moves: Vec<String>) {
        self.openings.insert(
            name.to_string(),
            RepertoireOpening {
                name: name.to_string(),
                moves,
                win_rate: 0.0,
                games_played: 0,
            },
        );
    }

    pub fn record_game(&mut self, name: &str, won: bool) {
        if let Some(opening) = self.openings.get_mut(name) {
            opening.games_played += 1;
            let old_rate = opening.win_rate;
            let n = opening.games_played as f32;
            opening.win_rate = ((old_rate * (n - 1.0)) + if won { 1.0 } else { 0.0 }) / n;
        }
    }

    pub fn best_openings(&self) -> Vec<(&str, f32)> {
        let mut sorted: Vec<_> = self
            .openings
            .values()
            .filter(|o| o.games_played >= 3)
            .map(|o| (o.name.as_str(), o.win_rate))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sorted.into_iter().take(5).collect()
    }
}

pub fn toggle_repertoire(
    keys: Res<ButtonInput<KeyCode>>,
    mut rep: ResMut<OpeningRepertoire>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyR) {
        rep.enabled = !rep.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if rep.enabled {
                "开局库管理已开启"
            } else {
                "开局库管理已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_opening() {
        let mut rep = OpeningRepertoire::default();
        rep.add_opening("中炮", vec!["h2e2".to_string()]);
        assert_eq!(rep.openings.len(), 1);
    }
    #[test]
    fn test_record_game() {
        let mut rep = OpeningRepertoire::default();
        rep.add_opening("中炮", vec!["h2e2".to_string()]);
        rep.record_game("中炮", true);
        rep.record_game("中炮", false);
        assert_eq!(rep.openings["中炮"].games_played, 2);
        assert_eq!(rep.openings["中炮"].win_rate, 0.5);
    }
}
