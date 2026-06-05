//! Non-blocking AI move computation.
//!
//! When it becomes the AI's turn, [`request_ai_move`] launches a task on the
//! shared Tokio runtime and stores a receiver. [`poll_ai_move`] checks the
//! receiver each frame and applies the move when ready — all without ever
//! blocking the Bevy schedule.

use bevy::prelude::*;
use chess_ai::{Ai, Difficulty, SearchLimits, UciConfig};
use chess_core::Move;
use crossbeam_channel::{Receiver, TryRecvError};

use crate::app_state::{AiSettings, CoreGame, GameMode};
use crate::history_view::HistoryView;
use crate::moves::apply_local_move;
use crate::sound::{MoveSound, PendingSound};

/// Holds the in-flight AI computation, if any.
#[derive(Resource, Default)]
pub struct AiTask {
    pub rx: Option<Receiver<Option<Move>>>,
}

/// If it is the AI's turn and no task is running, start one.
pub fn request_ai_move(
    core: Res<CoreGame>,
    settings: Res<AiSettings>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
    mut task: ResMut<AiTask>,
) {
    if core.mode != GameMode::VsAi || core.game.is_over() {
        return;
    }
    if core.local_to_move() || task.rx.is_some() {
        return; // human's turn, or a search is already running
    }

    let board = core.game.board().clone();
    let limits: SearchLimits = settings.difficulty.limits();
    let use_book = settings.difficulty != Difficulty::Easy;
    let engine_path = settings.engine_path.clone();
    let eval_file = settings.eval_file.clone();

    let (tx, rx) = crossbeam_channel::bounded(1);
    task.rx = Some(rx);

    let rt = runtime.0.clone();
    rt.spawn(async move {
        let mut ai = match engine_path {
            Some(path) => {
                let mut cfg = UciConfig::new(path);
                if let Some(ev) = eval_file {
                    cfg = cfg.with_option("EvalFile", ev.to_string_lossy().to_string());
                }
                Ai::pikafish(&cfg).await
            }
            None => Ai::builtin(),
        };
        let mv = ai.best_move(&board, &[], limits, use_book).await;
        let _ = tx.send(mv);
    });
}

/// Apply the AI's move once the task finishes.
pub fn poll_ai_move(
    mut task: ResMut<AiTask>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut pending_sound: ResMut<PendingSound>,
    mut history_view: ResMut<HistoryView>,
) {
    let Some(rx) = task.rx.as_ref() else {
        return;
    };
    match rx.try_recv() {
        Ok(Some(mv)) => {
            // Detect capture before the move.
            let is_capture = core.game.board().piece_at(mv.to).is_some();

            apply_local_move(&mut core, mv);
            history_view.return_to_live();

            // Detect check after the move.
            let is_check = core.game.board().is_in_check(core.game.side_to_move());

            task.rx = None;
            dirty.0 = true;

            let moved_piece = core.game.board().piece_at(mv.to).map(|p| p.kind);
            pending_sound.sound = Some(if is_check {
                MoveSound::Check
            } else if is_capture {
                MoveSound::Capture
            } else {
                MoveSound::Normal
            });
            pending_sound.piece = moved_piece;
        }
        Ok(None) => {
            warn!("AI produced no move");
            task.rx = None;
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            task.rx = None;
        }
    }
}
