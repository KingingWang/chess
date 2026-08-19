//! Analysis mode: a live engine panel for the current position.
//!
//! Ctrl+A toggles it. While active, a background engine search continuously
//! evaluates the **current** board (restarting after every move), and a card
//! in the left sidebar shows evaluation, depth, nodes, best move and PV.
//! During the AI's own thinking turn the panel instead mirrors the game
//! engine's stream. Scores are displayed from Red's perspective.

use bevy::prelude::*;
use chess_ai::SearchInfo;
use crossbeam_channel::{Receiver, TryRecvError};

use crate::app_state::{AiSettings, CoreGame, UiFonts};
use crate::ui_theme::{CARD, GOLD_BRIGHT, HAIRLINE, JADE, TEXT, TEXT_DIM, TEXT_FAINT};

/// Resource tracking analysis mode state.
#[derive(Resource, Debug, Clone, Default)]
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
    /// FEN of the position the current eval/best-move/PV belong to.
    /// Display code must only trust the data when this matches the board on
    /// screen — anything else is stale output from an earlier position.
    pub info_fen: String,
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
        self.clear_position_data();
        // Keep eval_history for graphing
    }

    /// Forget position-specific data (eval, best move, PV) — called whenever
    /// the board no longer matches the analysed position.
    pub fn clear_position_data(&mut self) {
        self.eval_score = 0;
        self.best_move = None;
        self.principal_variation.clear();
        self.depth = 0;
        self.nodes = 0;
        self.info_fen.clear();
    }

    /// Update from search info.
    pub fn update_from_search_info(&mut self, info: &SearchInfo) {
        self.eval_score = info.red_score();
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

/// In-flight background analysis search, if any.
#[derive(Resource, Default)]
pub struct AnalysisTask {
    info_rx: Option<Receiver<SearchInfo>>,
    done_rx: Option<Receiver<()>>,
    /// FEN of the position being analysed; a change aborts and relaunches.
    analyzed_fen: String,
}

/// Movetime per background analysis pass. Long enough to reach a useful
/// depth, short enough to react quickly to a new position (the search is
/// aborted on the next move anyway).
const ANALYSIS_MOVETIME: std::time::Duration = std::time::Duration::from_secs(10);

/// Launch / feed / restart the background analysis search.
///
/// The game AI's own search has priority: while it is thinking we do not
/// run a second engine, and simply mirror its info stream into the panel.
pub fn analysis_engine_tick(
    mut analysis: ResMut<AnalysisMode>,
    mut task: ResMut<AnalysisTask>,
    core: Res<CoreGame>,
    settings: Res<AiSettings>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
    mut search_info: ResMut<crate::ai_bridge::SearchInfoResource>,
) {
    if !analysis.active || core.game.is_over() {
        // Drop the receivers; the engine notices the disconnected channel
        // and stops early instead of finishing an unwatched search.
        task.info_rx = None;
        task.done_rx = None;
        return;
    }

    let current_fen = core.game.board().to_fen();

    // Invalidate stale display data the moment the board no longer matches
    // the analysed position (a stale PV move may start from a now-empty
    // square and must never be rendered against the new board).
    if !analysis.info_fen.is_empty() && analysis.info_fen != current_fen {
        analysis.clear_position_data();
    }

    // Record eval history as the game progresses.
    if core.game.history_len() > analysis.eval_history.len() {
        analysis.record_eval();
    }

    // While the game engine is thinking, its stream already feeds
    // `search_info.latest` — mirror it and don't launch a second engine.
    // The mirrored info describes the current position, so tag it as such.
    if search_info.thinking {
        if let Some(info) = &search_info.latest {
            analysis.update_from_search_info(&info.clone());
            analysis.info_fen = current_fen.clone();
        }
        return;
    }

    // A move was played mid-search: abort immediately. Dropping the
    // receivers disconnects the channel, and the engine side stops at the
    // next info line instead of finishing a stale 10 s search.
    let position_changed = task.analyzed_fen != current_fen;
    if task.done_rx.is_some() && position_changed {
        task.info_rx = None;
        task.done_rx = None;
    }

    // Drain the background stream (tagged with the analysed FEN).
    if let Some(rx) = &task.info_rx {
        while let Ok(info) = rx.try_recv() {
            analysis.update_from_search_info(&info);
            analysis.info_fen = task.analyzed_fen.clone();
            // Also feed the eval bar with the live analysis.
            search_info.latest = Some(info);
        }
    }

    let finished = match &task.done_rx {
        None => false,
        Some(rx) => !matches!(rx.try_recv(), Err(TryRecvError::Empty)),
    };
    if finished {
        task.info_rx = None;
        task.done_rx = None;
    }

    // (Re)launch when idle and the position has moved on since the last pass.
    let idle = task.done_rx.is_none();
    if idle && position_changed {
        let board = core.game.board().clone();
        let limits = chess_ai::SearchLimits {
            movetime: ANALYSIS_MOVETIME,
            max_depth: 64,
            variety_window: 0,
        };
        let (info_tx, info_rx) = crossbeam_channel::bounded(8);
        let (done_tx, done_rx) = crossbeam_channel::bounded(1);
        let rt = runtime.0.clone();
        match crate::ai_bridge::engine_config(&settings) {
            Some(cfg) => {
                rt.spawn(async move {
                    match chess_ai::UciEngine::launch(&cfg).await {
                        Ok(mut engine) => {
                            let _ = engine
                                .best_move_with_info(&board, &[], limits.movetime, Some(info_tx))
                                .await;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "analysis engine failed to launch");
                        }
                    }
                    let _ = done_tx.send(());
                });
            }
            None => {
                // No external engine: analyse with the built-in search.
                rt.spawn(async move {
                    tokio::task::spawn_blocking(move || {
                        chess_ai::search::search_with_info(&board, limits, Some(info_tx))
                    })
                    .await
                    .ok();
                    let _ = done_tx.send(());
                });
            }
        }
        task.analyzed_fen = current_fen.clone();
        task.info_rx = Some(info_rx);
        task.done_rx = Some(done_rx);
    }
}

/// Keyboard shortcut to toggle analysis mode.
pub fn toggle_analysis_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut analysis: ResMut<AnalysisMode>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
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

/// Marker for the analysis panel card.
#[derive(Component)]
pub struct AnalysisPanel;
/// Marker for the big evaluation value.
#[derive(Component)]
pub struct AnalysisEvalText;
/// Marker for the depth / nodes line.
#[derive(Component)]
pub struct AnalysisMetaText;
/// Marker for the best-move line.
#[derive(Component)]
pub struct AnalysisBestText;
/// Marker for the principal-variation line.
#[derive(Component)]
pub struct AnalysisPvText;

/// Spawn or remove the sidebar analysis card as the mode is toggled.
pub fn manage_analysis_panel(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    analysis: Res<AnalysisMode>,
    slot_q: Query<Entity, With<crate::ui::AnalysisSlot>>,
    panel_q: Query<Entity, With<AnalysisPanel>>,
) {
    let exists = !panel_q.is_empty();
    if analysis.active && !exists {
        let Ok(slot) = slot_q.single() else { return };
        commands.entity(slot).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(CARD),
                    BorderColor::all(HAIRLINE),
                    AnalysisPanel,
                ))
                .with_children(|card| {
                    // Header: dot + title.
                    card.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(7.0),
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(6.0),
                                    height: Val::Px(6.0),
                                    border_radius: BorderRadius::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(JADE),
                            ),
                            (
                                Text::new("分析模式"),
                                TextFont {
                                    font: fonts.bold.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(GOLD_BRIGHT),
                            )
                        ],
                    ));
                    // Big evaluation value.
                    card.spawn((
                        Text::new("…"),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(TEXT),
                        AnalysisEvalText,
                    ));
                    // Depth / nodes.
                    card.spawn((
                        Text::new(""),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
                        AnalysisMetaText,
                    ));
                    // Best move.
                    card.spawn((
                        Text::new(""),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.5,
                            ..default()
                        },
                        TextColor(TEXT),
                        AnalysisBestText,
                    ));
                    // Principal variation.
                    card.spawn((
                        Text::new(""),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 11.5,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                        AnalysisPvText,
                    ));
                });
        });
    } else if !analysis.active && exists {
        for e in &panel_q {
            commands.entity(e).despawn();
        }
    }
}

/// Keep the panel texts in sync with the latest search info.
#[allow(clippy::too_many_arguments)]
pub fn update_analysis_panel(
    analysis: Res<AnalysisMode>,
    core: Res<CoreGame>,
    mut texts: Query<(
        &mut Text,
        Option<&AnalysisEvalText>,
        Option<&AnalysisMetaText>,
        Option<&AnalysisBestText>,
        Option<&AnalysisPvText>,
    )>,
) {
    if !analysis.active {
        return;
    }
    for (mut text, is_eval, is_meta, is_best, is_pv) in &mut texts {
        if is_eval.is_some() {
            **text = analysis.eval_string();
        } else if is_meta.is_some() {
            if analysis.depth > 0 {
                **text = format!(
                    "深度 {} · {} 节点",
                    analysis.depth,
                    format_nodes(analysis.nodes)
                );
            } else {
                **text = "引擎启动中…".to_string();
            }
        } else if is_best.is_some() {
            **text = match analysis.best_move {
                Some(mv) => {
                    // Chinese notation only when the info belongs to the
                    // board on screen AND the moving piece is still there
                    // (belt and braces: move_to_chinese panics otherwise).
                    let fresh = !analysis.info_fen.is_empty()
                        && analysis.info_fen == core.game.board().to_fen();
                    if fresh && core.game.board().piece_at(mv.from).is_some() {
                        format!(
                            "最佳：{} ({})",
                            chess_core::notation::move_to_chinese(mv, core.game.board()),
                            mv.to_iccs()
                        )
                    } else {
                        format!("最佳：{}", mv.to_iccs())
                    }
                }
                None => String::new(),
            };
        } else if is_pv.is_some() {
            if analysis.principal_variation.len() > 1 {
                let pv: Vec<String> = analysis
                    .principal_variation
                    .iter()
                    .take(6)
                    .map(|m| m.to_iccs())
                    .collect();
                **text = format!("变化：{}", pv.join(" "));
            } else {
                **text = String::new();
            }
        }
    }
}

/// Compact node count: 12.3万 / 1.2M style.
fn format_nodes(n: u64) -> String {
    if n >= 100_000_000 {
        format!("{:.1}亿", n as f64 / 1e8)
    } else if n >= 10_000 {
        format!("{:.1}万", n as f64 / 1e4)
    } else {
        n.to_string()
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

    #[test]
    fn update_from_search_info_uses_red_perspective() {
        let mut analysis = AnalysisMode::default();
        let info = chess_ai::SearchInfo {
            depth: 12,
            score: -100, // black-to-move sees -1.00 ...
            side_to_move: chess_core::Color::Black,
            pv: vec![chess_core::Move::new(
                chess_core::Square::new(4, 2).unwrap(),
                chess_core::Square::new(4, 5).unwrap(),
            )],
            nodes: 5000,
            elapsed: std::time::Duration::ZERO,
            is_final: false,
        };
        analysis.update_from_search_info(&info);
        assert_eq!(analysis.eval_score, 100, "...stored as Red advantage");
        assert_eq!(analysis.depth, 12);
        assert!(analysis.best_move.is_some());
    }

    #[test]
    fn stale_position_data_is_cleared_on_fen_mismatch() {
        let mut analysis = AnalysisMode::default();
        analysis.active = true;
        analysis.info_fen = "some-position".to_string();
        analysis.eval_score = 150;
        analysis.best_move = Some(chess_core::Move::new(
            chess_core::Square::new(0, 0).unwrap(),
            chess_core::Square::new(0, 1).unwrap(),
        ));
        // Simulates what the tick does when the board no longer matches.
        if analysis.info_fen != "a-different-position" {
            analysis.clear_position_data();
        }
        assert!(analysis.best_move.is_none());
        assert_eq!(analysis.eval_score, 0);
        assert!(analysis.info_fen.is_empty());
    }

    #[test]
    fn test_format_nodes() {
        assert_eq!(format_nodes(999), "999");
        assert_eq!(format_nodes(12_345), "1.2万");
        assert_eq!(format_nodes(230_000_000), "2.3亿");
    }
}
