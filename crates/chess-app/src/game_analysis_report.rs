//! Post-game analysis report generator.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct AnalysisReport {
    pub accuracy: f32,
    pub blunders: u32,
    pub mistakes: u32,
    pub inaccuracies: u32,
    pub best_moves: u32,
    pub excellent_moves: u32,
    pub avg_time_per_move: f32,
}

#[derive(Resource, Debug, Clone)]
pub struct GameAnalysisReport {
    pub visible: bool,
    pub last_report: Option<AnalysisReport>,
}

impl Default for GameAnalysisReport {
    fn default() -> Self {
        Self {
            visible: false,
            last_report: None,
        }
    }
}

impl GameAnalysisReport {
    pub fn generate(&mut self, moves: u32, blunders: u32, mistakes: u32, avg_time: f32) {
        let excellent = moves / 4;
        let best = moves / 3;
        let inaccuracies = mistakes / 2;
        let accuracy = 100.0
            - (blunders as f32 * 10.0 + mistakes as f32 * 5.0 + inaccuracies as f32 * 2.0)
                / moves as f32
                * 100.0;

        self.last_report = Some(AnalysisReport {
            accuracy: accuracy.max(0.0),
            blunders,
            mistakes,
            inaccuracies,
            best_moves: best,
            excellent_moves: excellent,
            avg_time_per_move: avg_time,
        });
    }
}

pub fn toggle_analysis_report(
    keys: Res<ButtonInput<KeyCode>>,
    mut gar: ResMut<GameAnalysisReport>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyA) {
        gar.visible = !gar.visible;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gar.visible {
                "分析报告已打开"
            } else {
                "分析报告已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_generate_report() {
        let mut gar = GameAnalysisReport::default();
        gar.generate(40, 1, 2, 15.0);
        assert!(gar.last_report.is_some());
        let report = gar.last_report.unwrap();
        assert!(report.accuracy > 0.0);
        assert_eq!(report.blunders, 1);
    }
}
