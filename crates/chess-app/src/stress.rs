//! Headless soak-test driver ("many games, no human").
//!
//! ```bash
//! CHESS_STRESS=1 CHESS_STRESS_SECS=10800 ./target/debug/chess
//! ```
//!
//! Plays full games automatically: a random-legal-move Red against the AI
//! Black, analysis mode toggling on/off between games, occasional undoes,
//! automatic restarts after every finished game. Prints a heartbeat once a
//! minute and a summary on exit, so a crash is unmistakable in the log
//! (the process dies with a panic instead of exiting cleanly).
//!
//! Env:
//! - `CHESS_STRESS`        enable the driver
//! - `CHESS_STRESS_SECS`   wall-clock budget (default 60)
//! - `CHESS_STRESS_GAMES`  optional game cap

use bevy::prelude::*;

use crate::ai_bridge::AiTask;
use crate::analysis_mode::AnalysisMode;
use crate::app_state::{AiSettings, CoreGame, GameMode};
use crate::board_view::RenderDirty;
use crate::history_view::HistoryView;

/// Ply cap per game so a random-walk Red cannot wander forever.
const MOVE_CAP: usize = 300;

#[derive(Resource)]
pub struct Stress {
    deadline: std::time::Instant,
    max_games: Option<u32>,
    games: u32,
    moves: u32,
    undoes: u32,
    frames_until_move: u32,
    frames_until_heartbeat: u32,
    next_undo_at_move: u32,
}

/// `None` means normal (interactive) startup.
pub fn from_env() -> Option<Stress> {
    std::env::var("CHESS_STRESS").ok()?;
    let secs = std::env::var("CHESS_STRESS_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let max_games = std::env::var("CHESS_STRESS_GAMES")
        .ok()
        .and_then(|v| v.parse().ok());
    Some(Stress {
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(secs),
        max_games,
        games: 0,
        moves: 0,
        undoes: 0,
        frames_until_move: 30,
        frames_until_heartbeat: 60 * 60,
        next_undo_at_move: 40,
    })
}

/// Jump into the first stress game on startup.
pub fn enter(
    stress: Option<Res<Stress>>,
    mut next: ResMut<NextState<crate::app_state::AppState>>,
    mut core: ResMut<CoreGame>,
    mut settings: ResMut<AiSettings>,
    mut analysis: ResMut<AnalysisMode>,
) {
    if stress.is_none() {
        return;
    }
    core.restart();
    core.mode = GameMode::VsAi;
    core.local_color = chess_core::Color::Red;
    settings.difficulty = chess_ai::Difficulty::Easy;
    analysis.active = true;
    next.set(crate::app_state::AppState::InGame);
    info!("stress: starting soak run");
}

/// Drive games until the wall-clock budget (or game cap) runs out.
#[allow(clippy::too_many_arguments)]
pub fn tick(
    stress: Option<ResMut<Stress>>,
    mut core: ResMut<CoreGame>,
    mut settings: ResMut<AiSettings>,
    mut analysis: ResMut<AnalysisMode>,
    mut ai_task: ResMut<AiTask>,
    mut dirty: ResMut<RenderDirty>,
    mut history_view: ResMut<HistoryView>,
    mut exit: MessageWriter<AppExit>,
) {
    // The resource only exists when launched with CHESS_STRESS=1; a normal
    // game must never run the soak driver.
    let Some(mut stress) = stress else { return };

    // Heartbeat once a minute.
    stress.frames_until_heartbeat = stress.frames_until_heartbeat.saturating_sub(1);
    if stress.frames_until_heartbeat == 0 {
        stress.frames_until_heartbeat = 60 * 60;
        info!(
            games = stress.games,
            moves = stress.moves,
            undoes = stress.undoes,
            history = core.game.history_len(),
            "stress: heartbeat"
        );
    }

    if std::time::Instant::now() >= stress.deadline
        || stress.max_games.is_some_and(|m| stress.games >= m)
    {
        info!(
            games = stress.games,
            moves = stress.moves,
            undoes = stress.undoes,
            "stress: budget reached, exiting cleanly"
        );
        exit.write(AppExit::Success);
        return;
    }

    // Finished game: log the result, vary the settings, start the next one.
    if core.game.is_over() {
        stress.games += 1;
        info!(
            game = stress.games,
            result = ?core.game.result(),
            plies = core.game.history_len(),
            "stress: game over"
        );
        core.restart();
        dirty.0 = true;
        // Alternate difficulty and analysis coverage between games.
        settings.difficulty = if stress.games.is_multiple_of(2) {
            chess_ai::Difficulty::Easy
        } else {
            chess_ai::Difficulty::Medium
        };
        analysis.active = stress.games % 3 != 2;
        stress.frames_until_move = 30;
        return;
    }

    if core.game.history_len() >= MOVE_CAP {
        stress.games += 1;
        info!(game = stress.games, "stress: move cap reached, restarting");
        core.restart();
        dirty.0 = true;
        return;
    }

    // Occasionally rewind (exercises undo + analysis FEN invalidation).
    if stress.moves >= stress.next_undo_at_move
        && core.game.history_len() >= 4
        && !core.game.is_over()
    {
        stress.next_undo_at_move = stress.moves + 137;
        stress.undoes += 1;
        // Cancel any in-flight AI task first, exactly like the UI does.
        ai_task.rx = None;
        ai_task.info_rx = None;
        core.game.undo();
        core.game.undo();
        history_view.return_to_live();
        dirty.0 = true;
        info!("stress: undid two plies");
        return;
    }

    // Auto-play Red with random legal moves.
    if core.local_to_move() {
        if stress.frames_until_move > 0 {
            stress.frames_until_move -= 1;
            return;
        }
        stress.frames_until_move = 20;
        let moves = core.game.legal_moves();
        if moves.is_empty() {
            return;
        }
        let mut rng = chess_ai::rng::SmallRng::from_entropy();
        let mv = moves[rng.below(moves.len() as u64) as usize];
        crate::moves::apply_local_move(&mut core, mv);
        history_view.return_to_live();
        dirty.0 = true;
        stress.moves += 1;
    }
}
