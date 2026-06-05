//! Opening move suggestion toast for beginners.
//!
//! Shows a one-time hint about common opening moves when the board is at
//! the starting position. The hint auto-resets when moves are made so it
//! re-arms for the next game.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};

/// Tracks whether the opening hint has been shown for the current game.
#[derive(Resource, Default)]
pub struct OpeningHintShown(pub bool);

/// Show a suggestion toast at the starting position (ply 0) for the
/// local player. Self-resets: when `history_len() > 0` the flag is
/// cleared so the next fresh game re-triggers the hint.
pub fn show_opening_hint(
    mut commands: Commands,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    mut shown: ResMut<OpeningHintShown>,
) {
    let history_len = core.game.history_len();

    // Self-reset: clear the flag once moves have been made so a new
    // game (restart / rematch) re-arms the hint.
    if history_len > 0 {
        shown.0 = false;
        return;
    }

    // Only show at the starting position for the local player.
    if shown.0 || !core.local_to_move() || core.game.is_over() {
        return;
    }

    shown.0 = true;
    crate::toast::spawn_toast_long(
        &mut commands,
        &fonts,
        "推荐开局 (红方先行): 炮二平五 或 马二进三",
    );
}
