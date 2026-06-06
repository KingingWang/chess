//! Game variations explorer for analyzing alternative lines.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct Variation {
    pub moves: Vec<String>,
    pub evaluation: i32,
    pub name: String,
}

#[derive(Resource, Debug, Clone)]
pub struct GameVariations {
    pub enabled: bool,
    pub variations: Vec<Variation>,
    pub max_depth: usize,
}

impl Default for GameVariations {
    fn default() -> Self {
        Self {
            enabled: false,
            variations: Vec::new(),
            max_depth: 10,
        }
    }
}

impl GameVariations {
    pub fn add_variation(&mut self, name: &str, moves: Vec<String>, eval: i32) {
        self.variations.push(Variation {
            moves,
            evaluation: eval,
            name: name.to_string(),
        });
    }
    pub fn best_variation(&self) -> Option<&Variation> {
        self.variations.iter().max_by_key(|v| v.evaluation)
    }
    pub fn clear(&mut self) {
        self.variations.clear();
    }
}

pub fn toggle_variations(
    keys: Res<ButtonInput<KeyCode>>,
    mut gv: ResMut<GameVariations>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyY) {
        gv.enabled = !gv.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gv.enabled {
                "变例浏览器已开启"
            } else {
                "变例浏览器已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut gv = GameVariations::default();
        gv.add_variation("test", vec!["h2e2".to_string()], 100);
        assert_eq!(gv.variations.len(), 1);
    }
    #[test]
    fn test_best() {
        let mut gv = GameVariations::default();
        gv.add_variation("a", vec![], 100);
        gv.add_variation("b", vec![], 200);
        assert_eq!(gv.best_variation().unwrap().evaluation, 200);
    }
}
