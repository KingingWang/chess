//! Analysis mode for game evaluation and move suggestions.
//!
//! When enabled, displays real-time engine evaluation, best move suggestions,
//! and evaluation history.

use bevy::prelude::*;
use chess_ai::SearchInfo;

/// Resource tracking analysis mode state.
#[derive(Resource, Debug, Clone)]
pub struct AnalysisMode {
    /// Whether analysis mode is active.
    pub active: bool,
    /// Current evaluation score (centipawns, positive = Red advantage).
    pub eval_score: i32,
    /// Best move suggested by the engine.
    pub best_move: Option<chess_core::Move>,
    /// Principal variation (sequence of best moves).
    pub principal_variation: Vec<chess_core::Move>,
    /// Search depth reached.
    pub depth: u32,
    /// Number of nodes searched.
    pub nodes: u64,
    /// Evaluation history for graphing.
    pub eval_history: Vec<i32>,
}

impl Default for AnalysisMode {
    fn default() -> Self {
        Self {
            active: false,
            eval_score: 0,
            best_move: None,
            principal_variation: Vec::new(),
            depth: 0,
            nodes: 0,
            eval_history: Vec::new(),
        }
    }
}

impl AnalysisMode {
    /// Toggle analysis mode on/off.
    pub fn toggle(&mut self) {
        self.active = !self.active;
        if !self.active {
            self.clear();
        }
    }

    /// Clear analysis data.
    pub fn clear(&mut self) {
        self.eval_score = 0;
        self.best_move = None;
        self.principal_variation.clear();
        self.depth = 0;
        self.nodes = 0;
        // Keep eval_history for graphing
    }

    /// Update from search info.
    pub fn update_from_search_info(&mut self, info: &SearchInfo) {
        self.eval_score = info.score;
        self.depth = info.depth;
        self.nodes = info.nodes;
        self.principal_variation = info.pv.clone();
        if let Some(mv) = info.pv.first() {
            self.best_move = Some(*mv);
        }
    }

    /// Record current evaluation to history.
    pub fn record_eval(&mut self) {
        self.eval_history.push(self.eval_score);
        // Keep last 100 evaluations
        if self.eval_history.len() > 100 {
            self.eval_history.remove(0);
        }
    }

    /// Get evaluation as a human-readable string.
    pub fn eval_string(&self) -> String {
        if self.eval_score.abs() >= 9900 {
            // Mate score
            let moves_to_mate = (10000 - self.eval_score.abs()) / 2;
            if self.eval_score > 0 {
                format!("M{} (红胜)", moves_to_mate)
            } else {
                format!("M{} (黑胜)", moves_to_mate)
            }
        } else {
            let pawns = self.eval_score as f32 / 100.0;
            format!("{:+.2}", pawns)
        }
    }

    /// Get evaluation as percentage for display (50% = equal).
    pub fn eval_percentage(&self) -> f32 {
        // Clamp to reasonable range (-1000 to +1000 centipawns)
        let clamped = self.eval_score.clamp(-1000, 1000);
        // Convert to percentage (50% = equal)
        (50.0 + (clamped as f32 / 20.0)).clamp(0.0, 100.0)
    }
}

/// System to update analysis mode from search info.
pub fn update_analysis_from_search_info(
    mut analysis: ResMut<AnalysisMode>,
    search_info: Res<crate::ai_bridge::SearchInfoResource>,
    core: Res<crate::app_state::CoreGame>,
) {
    if !analysis.active {
        return;
    }

    // Update from search info if available
    if let Some(info) = &search_info.latest {
        analysis.update_from_search_info(info);
    }

    // Record evaluation when a move is made
    if core.game.history_len() > analysis.eval_history.len() {
        analysis.record_eval();
    }
}

/// Keyboard shortcut to toggle analysis mode.
pub fn toggle_analysis_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut analysis: ResMut<AnalysisMode>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    // Ctrl+A to toggle analysis mode
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyA) {
        analysis.toggle();
        let status = if analysis.active {
            "已开启"
        } else {
            "已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &format!("分析模式{}", status));
    }
}

/// Marker for analysis UI elements.
#[derive(Component)]
pub struct AnalysisUI;

/// Spawn analysis mode UI overlay.
pub fn spawn_analysis_ui(
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    analysis: Res<AnalysisMode>,
) {
    if !analysis.active {
        return;
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                width: Val::Px(300.0),
                height: Val::Px(150.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            AnalysisUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("分析模式"),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
            ));

            parent.spawn((
                Text::new("评估: 0.00"),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent.spawn((
                Text::new("深度: 0"),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            parent.spawn((
                Text::new("最佳着法: -"),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

/// Update analysis UI with current data.
pub fn update_analysis_ui(
    analysis: Res<AnalysisMode>,
    mut ui_query: Query<&mut Text, With<AnalysisUI>>,
) {
    if !analysis.active {
        return;
    }

    // Update UI elements (simplified - in real implementation would update specific text nodes)
    // This is a placeholder for the actual UI update logic
}

/// Despawn analysis UI when mode is disabled.
pub fn despawn_analysis_ui(
    mut commands: Commands,
    analysis: Res<AnalysisMode>,
    ui_query: Query<Entity, With<AnalysisUI>>,
) {
    if !analysis.active {
        for entity in ui_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_mode_toggle() {
        let mut analysis = AnalysisMode::default();
        assert!(!analysis.active);

        analysis.toggle();
        assert!(analysis.active);

        analysis.toggle();
        assert!(!analysis.active);
    }

    #[test]
    fn test_eval_string_normal() {
        let mut analysis = AnalysisMode::default();
        analysis.eval_score = 150;
        assert_eq!(analysis.eval_string(), "+1.50");

        analysis.eval_score = -250;
        assert_eq!(analysis.eval_string(), "-2.50");
    }

    #[test]
    fn test_eval_string_mate() {
        let mut analysis = AnalysisMode::default();
        analysis.eval_score = 9990; // Mate in 5
        assert_eq!(analysis.eval_string(), "M5 (红胜)");

        analysis.eval_score = -9980; // Mate in 10
        assert_eq!(analysis.eval_string(), "M10 (黑胜)");
    }

    #[test]
    fn test_eval_percentage() {
        let mut analysis = AnalysisMode::default();
        analysis.eval_score = 0;
        assert_eq!(analysis.eval_percentage(), 50.0);

        analysis.eval_score = 500;
        assert_eq!(analysis.eval_percentage(), 75.0);

        analysis.eval_score = -500;
        assert_eq!(analysis.eval_percentage(), 25.0);
    }

    #[test]
    fn test_record_eval() {
        let mut analysis = AnalysisMode::default();
        analysis.eval_score = 100;
        analysis.record_eval();
        assert_eq!(analysis.eval_history.len(), 1);
        assert_eq!(analysis.eval_history[0], 100);

        analysis.eval_score = 200;
        analysis.record_eval();
        assert_eq!(analysis.eval_history.len(), 2);
        assert_eq!(analysis.eval_history[1], 200);
    }

    #[test]
    fn test_clear() {
        let mut analysis = AnalysisMode::default();
        analysis.eval_score = 100;
        analysis.best_move = Some(chess_core::Move {
            from: chess_core::Square::new(0, 0).unwrap(),
            to: chess_core::Square::new(0, 1).unwrap(),
        });
        analysis.depth = 10;
        analysis.nodes = 1000;

        analysis.clear();

        assert_eq!(analysis.eval_score, 0);
        assert!(analysis.best_move.is_none());
        assert_eq!(analysis.depth, 0);
        assert_eq!(analysis.nodes, 0);
    }
}
