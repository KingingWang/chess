//! Centralised move application so every source (local input, AI, network)
//! funnels through the same validation and side-effect path.

use chess_core::{GameResult, Move};

use crate::app_state::CoreGame;

/// Apply a move to the authoritative game, returning the result if the game
/// ended. Illegal moves are rejected (logged) and ignored.
pub fn apply_local_move(core: &mut CoreGame, mv: Move) -> Option<GameResult> {
    match core.game.make_move(mv) {
        Ok(result) => {
            bevy::log::info!(mv = %mv.to_iccs(), "move applied");
            result
        }
        Err(e) => {
            bevy::log::warn!(error = %e, mv = %mv.to_iccs(), "rejected illegal move");
            None
        }
    }
}
