//! Centralised move application so every source (local input, AI, network)
//! funnels through the same validation and side-effect path.

use chess_core::{GameResult, Move};

use crate::app_state::CoreGame;

/// Flag set when a move was applied this frame; the clock system reads it.
pub static MOVE_APPLIED_THIS_FRAME: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
/// Flag set when a capture occurred this frame; triggers particle effects.
pub static CAPTURE_THIS_FRAME: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Flag set when a check was delivered this frame; triggers screen shake.
pub static CHECK_THIS_FRAME: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Flag set when checkmate occurred this frame; triggers celebration.
pub static CHECKMATE_THIS_FRAME: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Apply a move to the authoritative game, returning the result if the game
/// ended. Illegal moves are rejected (logged) and ignored.
pub fn apply_local_move(core: &mut CoreGame, mv: Move) -> Option<GameResult> {
    // Check if this is a capture before making the move
    let is_capture = core.game.board().piece_at(mv.to).is_some();

    match core.game.make_move(mv) {
        Ok(result) => {
            core.last_move = Some((mv.from, mv.to));
            MOVE_APPLIED_THIS_FRAME.store(true, std::sync::atomic::Ordering::Relaxed);

            // Set animation flags based on game state after move
            if is_capture {
                CAPTURE_THIS_FRAME.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            // Check if opponent is in check (but not checkmate yet)
            let opponent_color = core.game.board().side_to_move();
            let in_check = core.game.board().is_in_check(opponent_color);

            match result {
                Some(GameResult::Win {
                    reason: chess_core::WinReason::Checkmate,
                    ..
                }) => {
                    CHECKMATE_THIS_FRAME.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ if in_check => {
                    CHECK_THIS_FRAME.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }

            bevy::log::info!(mv = %mv.to_iccs(), "move applied");
            result
        }
        Err(e) => {
            bevy::log::warn!(error = %e, mv = %mv.to_iccs(), "rejected illegal move");
            None
        }
    }
}

/// Set by `keyboard_shortcuts` after a successful undo; consumed by
/// `undo_sound_trigger` one frame later.
pub static UNDO_PERFORMED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Set by standalone systems when Escape closes an overlay panel,
/// consumed by `keyboard_shortcuts` to prevent same-frame menu navigation.
pub static ESCAPE_CONSUMED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Set by any restart/load/rematch path; consumed by `undo_sound_trigger`
/// to reset `UndoCount`.
pub static GAME_RESTARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
