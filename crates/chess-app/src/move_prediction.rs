//! Move prediction based on game context.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct MovePrediction {
    pub enabled: bool,
    pub predicted_move: Option<String>,
    pub confidence: f32,
    pub history: Vec<String>,
}

impl Default for MovePrediction {
    fn default() -> Self {
        Self {
            enabled: false,
            predicted_move: None,
            confidence: 0.0,
            history: Vec::new(),
        }
    }
}

impl MovePrediction {
    pub fn predict(&mut self, predicted: &str, confidence: f32) {
        self.predicted_move = Some(predicted.to_string());
        self.confidence = confidence;
        self.history.push(predicted.to_string());
        if self.history.len() > 50 {
            self.history.remove(0);
        }
    }
    pub fn accuracy(&self, actual: &[String]) -> f32 {
        if actual.is_empty() || self.history.is_empty() {
            return 0.0;
        }
        let matches = self
            .history
            .iter()
            .zip(actual.iter())
            .filter(|(a, b)| *a == *b)
            .count();
        matches as f32 / actual.len() as f32 * 100.0
    }
    pub fn clear(&mut self) {
        self.predicted_move = None;
        self.confidence = 0.0;
        self.history.clear();
    }
}

pub fn toggle_prediction(
    keys: Res<ButtonInput<KeyCode>>,
    mut mp: ResMut<MovePrediction>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyR) {
        mp.enabled = !mp.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if mp.enabled {
                "着法预测已开启"
            } else {
                "着法预测已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_predict() {
        let mut mp = MovePrediction::default();
        mp.predict("h2e2", 0.8);
        assert_eq!(mp.predicted_move, Some("h2e2".to_string()));
    }
    #[test]
    fn test_accuracy() {
        let mut mp = MovePrediction::default();
        mp.predict("h2e2", 0.8);
        let acc = mp.accuracy(&["h2e2".to_string()]);
        assert_eq!(acc, 100.0);
    }
}
