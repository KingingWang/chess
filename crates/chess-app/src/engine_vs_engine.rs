//! Engine vs Engine game mode.
//!
//! Allows two AI engines to play against each other automatically.

use bevy::prelude::*;
use chess_ai::{Ai, SearchLimits};
use chess_core::{Color as ChessColor, Move};
use std::time::{Duration, Instant};

/// Configuration for an engine vs engine game.
#[derive(Debug, Clone)]
pub struct EngineVsEngineConfig {
    /// Time limit per move in milliseconds.
    pub time_per_move_ms: u64,
    /// Maximum depth to search.
    pub max_depth: u32,
    /// Whether to use random opening moves.
    pub random_opening: bool,
    /// Number of random opening moves.
    pub random_moves: usize,
    /// Delay between moves in milliseconds (for viewing).
    pub move_delay_ms: u64,
}

impl Default for EngineVsEngineConfig {
    fn default() -> Self {
        Self {
            time_per_move_ms: 1000,
            max_depth: 10,
            random_opening: false,
            random_moves: 0,
            move_delay_ms: 500,
        }
    }
}

/// State of an engine vs engine game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineGameStatus {
    /// Game is not running.
    Idle,
    /// Game is running.
    Running,
    /// Game is paused.
    Paused,
    /// Game has finished.
    Finished,
}

/// Resource managing engine vs engine games.
#[derive(Resource)]
pub struct EngineVsEngine {
    /// Current status of the engine game.
    pub status: EngineGameStatus,
    /// Configuration for the game.
    pub config: EngineVsEngineConfig,
    /// Red engine instance.
    pub red_engine: Option<Ai>,
    /// Black engine instance.
    pub black_engine: Option<Ai>,
    /// Moves played in the game.
    pub moves: Vec<Move>,
    /// Current move number.
    pub move_number: usize,
    /// Time when the last move was made.
    pub last_move_time: Option<Instant>,
    /// Winner of the game (if finished).
    pub winner: Option<ChessColor>,
    /// Reason the game ended.
    pub end_reason: Option<String>,
}

impl Default for EngineVsEngine {
    fn default() -> Self {
        Self {
            status: EngineGameStatus::Idle,
            config: EngineVsEngineConfig::default(),
            red_engine: None,
            black_engine: None,
            moves: Vec::new(),
            move_number: 0,
            last_move_time: None,
            winner: None,
            end_reason: None,
        }
    }
}

impl EngineVsEngine {
    /// Start a new engine vs engine game.
    pub fn start_game(&mut self, config: EngineVsEngineConfig) {
        self.status = EngineGameStatus::Running;
        self.config = config;
        self.moves.clear();
        self.move_number = 0;
        self.last_move_time = Some(Instant::now());
        self.winner = None;
        self.end_reason = None;

        // Initialize engines
        self.red_engine = Some(Ai::builtin());
        self.black_engine = Some(Ai::builtin());
    }

    /// Pause the game.
    pub fn pause(&mut self) {
        if self.status == EngineGameStatus::Running {
            self.status = EngineGameStatus::Paused;
        }
    }

    /// Resume the game.
    pub fn resume(&mut self) {
        if self.status == EngineGameStatus::Paused {
            self.status = EngineGameStatus::Running;
            self.last_move_time = Some(Instant::now());
        }
    }

    /// Stop the game.
    pub fn stop(&mut self) {
        self.status = EngineGameStatus::Finished;
        self.end_reason = Some("Stopped by user".to_string());
    }

    /// Check if it's time to make the next move.
    pub fn should_make_move(&self) -> bool {
        if self.status != EngineGameStatus::Running {
            return false;
        }

        if let Some(last_time) = self.last_move_time {
            let elapsed = last_time.elapsed();
            elapsed >= Duration::from_millis(self.config.move_delay_ms)
        } else {
            true
        }
    }

    /// Get the search limits for the current move.
    pub fn get_search_limits(&self) -> SearchLimits {
        SearchLimits {
            movetime: Duration::from_millis(self.config.time_per_move_ms),
            max_depth: self.config.max_depth,
        }
    }

    /// Record a move in the game.
    pub fn record_move(&mut self, mv: Move) {
        self.moves.push(mv);
        self.move_number += 1;
        self.last_move_time = Some(Instant::now());
    }

    /// End the game with a winner.
    pub fn end_game(&mut self, winner: Option<ChessColor>, reason: String) {
        self.status = EngineGameStatus::Finished;
        self.winner = winner;
        self.end_reason = Some(reason);
    }

    /// Get the current side to move.
    pub fn side_to_move(&self) -> ChessColor {
        if self.move_number % 2 == 0 {
            ChessColor::Red
        } else {
            ChessColor::Black
        }
    }

    /// Get the engine for the current side to move.
    pub fn current_engine(&self) -> Option<&Ai> {
        match self.side_to_move() {
            ChessColor::Red => self.red_engine.as_ref(),
            ChessColor::Black => self.black_engine.as_ref(),
        }
    }

    /// Get a mutable reference to the engine for the current side to move.
    pub fn current_engine_mut(&mut self) -> Option<&mut Ai> {
        match self.side_to_move() {
            ChessColor::Red => self.red_engine.as_mut(),
            ChessColor::Black => self.black_engine.as_mut(),
        }
    }

    /// Get a summary of the game.
    pub fn summary(&self) -> String {
        let status_str = match self.status {
            EngineGameStatus::Idle => "Not started",
            EngineGameStatus::Running => "Running",
            EngineGameStatus::Paused => "Paused",
            EngineGameStatus::Finished => "Finished",
        };

        let result_str = if let Some(winner) = self.winner {
            match winner {
                ChessColor::Red => "Red wins",
                ChessColor::Black => "Black wins",
            }
        } else if self.status == EngineGameStatus::Finished {
            "Draw"
        } else {
            "In progress"
        };

        format!(
            "Engine vs Engine - {} - Move {} - {}",
            status_str, self.move_number, result_str
        )
    }
}

/// Component for engine vs engine UI elements.
#[derive(Component)]
pub struct EngineVsEngineUI;

/// System to toggle engine vs engine mode.
pub fn toggle_engine_vs_engine(
    mut engine_game: ResMut<EngineVsEngine>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyE) {
        match engine_game.status {
            EngineGameStatus::Idle => {
                engine_game.start_game(EngineVsEngineConfig::default());
            }
            EngineGameStatus::Running => {
                engine_game.pause();
            }
            EngineGameStatus::Paused => {
                engine_game.resume();
            }
            EngineGameStatus::Finished => {
                engine_game.start_game(EngineVsEngineConfig::default());
            }
        }
    }
}

/// System to make moves in engine vs engine games.
pub fn make_engine_moves(
    mut engine_game: ResMut<EngineVsEngine>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
) {
    if !engine_game.should_make_move() {
        return;
    }

    let side = engine_game.side_to_move();
    let limits = engine_game.get_search_limits();
    let board = core.game.board().clone();
    let history: Vec<chess_core::Move> = core.game.played_moves().collect();

    // Get the best move from the engine
    if let Some(engine) = engine_game.current_engine_mut() {
        // Use the async runtime to call best_move
        let rt = runtime.0.clone();
        let result = rt.block_on(async { engine.best_move(&board, &history, limits, false).await });

        if let Some(best_move) = result {
            // Make the move
            if core.game.make_move(best_move).is_ok() {
                engine_game.record_move(best_move);
                dirty.0 = true;

                // Check if the game is over
                if let Some(result) = core.game.result() {
                    let (winner, reason) = match result {
                        chess_core::GameResult::Win { winner, reason } => {
                            let reason_str = match reason {
                                chess_core::WinReason::Checkmate => "Checkmate",
                                chess_core::WinReason::Stalemate => "Stalemate",
                                chess_core::WinReason::Resignation => "Resignation",
                                chess_core::WinReason::PerpetualCheck => "Perpetual check",
                                chess_core::WinReason::Timeout => "Timeout",
                            };
                            (Some(winner), reason_str.to_string())
                        }
                        chess_core::GameResult::Draw(reason) => {
                            let reason_str = match reason {
                                chess_core::DrawReason::Agreement => "Agreement",
                                chess_core::DrawReason::Repetition => "Repetition",
                                chess_core::DrawReason::NoCapture => "60 moves no capture",
                            };
                            (None, reason_str.to_string())
                        }
                    };
                    engine_game.end_game(winner, reason);
                }
            }
        } else {
            // Engine couldn't find a move - end the game
            engine_game.end_game(
                Some(if side == ChessColor::Red {
                    ChessColor::Black
                } else {
                    ChessColor::Red
                }),
                "Engine failed to find a move".to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn create_test_move() -> Move {
        let from = Square::new(4, 0).unwrap();
        let to = Square::new(4, 1).unwrap();
        Move { from, to }
    }

    #[test]
    fn test_engine_vs_engine_default() {
        let evs = EngineVsEngine::default();
        assert_eq!(evs.status, EngineGameStatus::Idle);
        assert_eq!(evs.move_number, 0);
        assert!(evs.moves.is_empty());
    }

    #[test]
    fn test_start_game() {
        let mut evs = EngineVsEngine::default();
        let config = EngineVsEngineConfig::default();

        evs.start_game(config);
        assert_eq!(evs.status, EngineGameStatus::Running);
        assert!(evs.red_engine.is_some());
        assert!(evs.black_engine.is_some());
    }

    #[test]
    fn test_pause_resume() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        evs.pause();
        assert_eq!(evs.status, EngineGameStatus::Paused);

        evs.resume();
        assert_eq!(evs.status, EngineGameStatus::Running);
    }

    #[test]
    fn test_stop_game() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        evs.stop();
        assert_eq!(evs.status, EngineGameStatus::Finished);
        assert!(evs.end_reason.is_some());
    }

    #[test]
    fn test_record_move() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        let mv = create_test_move();
        evs.record_move(mv);

        assert_eq!(evs.moves.len(), 1);
        assert_eq!(evs.move_number, 1);
    }

    #[test]
    fn test_side_to_move() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        assert_eq!(evs.side_to_move(), ChessColor::Red);

        evs.record_move(create_test_move());
        assert_eq!(evs.side_to_move(), ChessColor::Black);

        evs.record_move(create_test_move());
        assert_eq!(evs.side_to_move(), ChessColor::Red);
    }

    #[test]
    fn test_end_game() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        evs.end_game(Some(ChessColor::Red), "Checkmate".to_string());

        assert_eq!(evs.status, EngineGameStatus::Finished);
        assert_eq!(evs.winner, Some(ChessColor::Red));
        assert_eq!(evs.end_reason, Some("Checkmate".to_string()));
    }

    #[test]
    fn test_summary() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        let summary = evs.summary();
        assert!(summary.contains("Running"));
        assert!(summary.contains("Move 0"));
    }

    #[test]
    fn test_should_make_move() {
        let mut evs = EngineVsEngine::default();
        let mut config = EngineVsEngineConfig::default();
        config.move_delay_ms = 0;

        evs.start_game(config);
        assert!(evs.should_make_move());

        evs.pause();
        assert!(!evs.should_make_move());
    }

    #[test]
    fn test_get_search_limits() {
        let mut evs = EngineVsEngine::default();
        let mut config = EngineVsEngineConfig::default();
        config.time_per_move_ms = 2000;
        config.max_depth = 15;

        evs.start_game(config);
        let limits = evs.get_search_limits();

        assert_eq!(limits.movetime, Duration::from_millis(2000));
        assert_eq!(limits.max_depth, 15);
    }

    #[test]
    fn test_config_default() {
        let config = EngineVsEngineConfig::default();
        assert_eq!(config.time_per_move_ms, 1000);
        assert_eq!(config.max_depth, 10);
        assert_eq!(config.move_delay_ms, 500);
        assert!(!config.random_opening);
    }

    #[test]
    fn test_current_engine() {
        let mut evs = EngineVsEngine::default();
        evs.start_game(EngineVsEngineConfig::default());

        assert!(evs.current_engine().is_some());

        evs.record_move(create_test_move());
        assert!(evs.current_engine().is_some());
    }
}
