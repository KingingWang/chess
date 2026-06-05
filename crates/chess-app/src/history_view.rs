//! History view mode for stepping through past positions.
//!
//! When [`HistoryView::viewing_ply`] is `Some(ply)`, the board renders the
//! position after `ply` half-moves (0 = starting position), input is blocked,
//! and the history panel highlights the viewed move. Arrow keys (←/→) step
//! through the history; pressing → past the last move returns to live view.

use bevy::prelude::*;

/// Resource tracking whether the player is reviewing a historical position.
///
/// - `None` — live view (normal play)
/// - `Some(ply)` — viewing the position after `ply` half-moves
#[derive(Resource, Default)]
pub struct HistoryView {
    pub viewing_ply: Option<usize>,
}

impl HistoryView {
    /// Are we in history review mode?
    #[inline]
    pub fn is_viewing(&self) -> bool {
        self.viewing_ply.is_some()
    }

    /// Return to live view.
    pub fn return_to_live(&mut self) {
        self.viewing_ply = None;
    }

    /// Set the viewing ply with bounds checking.
    ///
    /// If `ply >= max_ply` (i.e. we've reached or passed the last move),
    /// automatically returns to live view (`None`). Otherwise sets
    /// `viewing_ply = Some(ply)`.
    pub fn set_ply(&mut self, ply: usize, max_ply: usize) {
        if ply >= max_ply {
            self.viewing_ply = None;
        } else {
            self.viewing_ply = Some(ply);
        }
    }
}
