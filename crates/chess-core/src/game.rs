//! High-level game state: move history, result determination, and the
//! repetition / perpetual-check ("长将") rules.

use crate::board::Board;
use crate::moves::{Move, UndoState};
use crate::piece::Color;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Outcome of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GameResult {
    /// `winner` won (by checkmate, stalemate-of-opponent, resignation, or a
    /// perpetual-check ruling against the loser).
    Win { winner: Color, reason: WinReason },
    /// Draw (agreement, or repetition where neither side is forcing).
    Draw(DrawReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WinReason {
    /// 将死 — opponent's king is checkmated.
    Checkmate,
    /// 困毙 — opponent has no legal move (counts as a loss in Xiangqi).
    Stalemate,
    /// Opponent resigned.
    Resignation,
    /// 长将 — opponent gave perpetual check and must yield.
    PerpetualCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DrawReason {
    /// Both players agreed.
    Agreement,
    /// Threefold repetition where neither side is the aggressor.
    Repetition,
    /// 60 moves with no capture (configurable inactivity rule).
    NoCapture,
}

/// Mutable game with full undo history and repetition tracking.
#[derive(Debug, Clone)]
pub struct Game {
    board: Board,
    history: Vec<HistoryEntry>,
    result: Option<GameResult>,
    /// Halfmoves since the last capture (for the inactivity draw rule).
    halfmove_clock: u32,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    undo: UndoState,
    /// FEN of the position *before* the move (placement + side only).
    fen_before: String,
    /// Did this move deliver check to the opponent?
    gave_check: bool,
    halfmove_clock_before: u32,
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

impl Game {
    pub fn new() -> Game {
        Game::from_board(Board::start_position())
    }

    pub fn from_board(board: Board) -> Game {
        Game {
            board,
            history: Vec::new(),
            result: None,
            halfmove_clock: 0,
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn side_to_move(&self) -> Color {
        self.board.side_to_move()
    }

    pub fn result(&self) -> Option<GameResult> {
        self.result
    }

    pub fn is_over(&self) -> bool {
        self.result.is_some()
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        if self.is_over() {
            Vec::new()
        } else {
            self.board.legal_moves()
        }
    }

    /// Apply a legal move, updating result/repetition state.
    ///
    /// Returns `Err` if the move is illegal or the game is already over.
    pub fn make_move(&mut self, mv: Move) -> Result<Option<GameResult>, IllegalMove> {
        if self.is_over() {
            return Err(IllegalMove::GameOver);
        }
        if !self.board.is_legal(mv) {
            return Err(IllegalMove::NotLegal(mv));
        }

        let fen_before = self.board.to_fen();
        let mover = self.board.side_to_move();
        let was_capture = self.board.piece_at(mv.to).is_some();
        let halfmove_clock_before = self.halfmove_clock;

        let undo = self.board.make_move(mv);
        let gave_check = self.board.is_in_check(mover.opponent());

        self.halfmove_clock = if was_capture { 0 } else { self.halfmove_clock + 1 };
        self.history.push(HistoryEntry {
            undo,
            fen_before,
            gave_check,
            halfmove_clock_before,
        });

        self.result = self.compute_result();
        Ok(self.result)
    }

    /// Undo the last move (also clears a result decided by that move).
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(entry) => {
                self.board.unmake_move(entry.undo);
                self.halfmove_clock = entry.halfmove_clock_before;
                self.result = None;
                true
            }
            None => false,
        }
    }

    /// Force a result (resignation or draw agreement).
    pub fn resign(&mut self, who: Color) {
        self.result = Some(GameResult::Win {
            winner: who.opponent(),
            reason: WinReason::Resignation,
        });
    }

    pub fn agree_draw(&mut self) {
        self.result = Some(GameResult::Draw(DrawReason::Agreement));
    }

    /// Recompute terminal status after the most recent move.
    fn compute_result(&self) -> Option<GameResult> {
        let side = self.board.side_to_move();
        // No legal reply -> the side to move loses (checkmate or stalemate).
        if self.board.legal_moves().is_empty() {
            let reason = if self.board.is_in_check(side) {
                WinReason::Checkmate
            } else {
                WinReason::Stalemate
            };
            return Some(GameResult::Win {
                winner: side.opponent(),
                reason,
            });
        }

        // Perpetual check: the side that has just moved delivered check, and
        // the current position has occurred three times with that side always
        // checking. That side must yield (loses).
        if let Some(loser) = self.perpetual_check_offender() {
            return Some(GameResult::Win {
                winner: loser.opponent(),
                reason: WinReason::PerpetualCheck,
            });
        }

        // Threefold repetition with no forcing side -> draw.
        if self.repetition_count() >= 3 {
            return Some(GameResult::Draw(DrawReason::Repetition));
        }

        if self.halfmove_clock >= 120 {
            // 60 full moves without a capture.
            return Some(GameResult::Draw(DrawReason::NoCapture));
        }

        None
    }

    /// How many times the current position has appeared (including now).
    fn repetition_count(&self) -> usize {
        let current = self.board.to_fen();
        let mut count = 1; // current occurrence
        for entry in &self.history {
            if entry.fen_before == current {
                count += 1;
            }
        }
        count
    }

    /// If the current (thrice-repeated) position was reached by one side always
    /// checking, that side is the perpetual-check offender and must yield.
    ///
    /// This implements the common, unambiguous "长将" case. The full official
    /// adjudication of 长捉 / 一将一杀 etc. is intentionally out of scope and
    /// documented as such.
    fn perpetual_check_offender(&self) -> Option<Color> {
        let current = self.board.to_fen();
        // Collect indices of history entries whose *resulting* position equals
        // the current one (i.e. entry.fen_before of the NEXT entry). Simpler:
        // we track, for each prior occurrence of `current`, who moved into it
        // and whether that move was a check.
        let mut occurrences: Vec<bool> = Vec::new(); // gave_check flags
        for i in 0..self.history.len() {
            // Position *after* history[i] equals fen_before of history[i+1],
            // or the live board for the last entry.
            let pos_after = if i + 1 < self.history.len() {
                &self.history[i + 1].fen_before
            } else {
                // after the last move == current live position
                // (we compare by value below)
                &current
            };
            if pos_after == &current {
                occurrences.push(self.history[i].gave_check);
            }
        }
        // Need the position to have occurred at least 3 times total and every
        // entry into it to have been a check by the same side.
        if occurrences.len() >= 2 && occurrences.iter().all(|&c| c) {
            // The side that just moved (delivered the repeated checks) is the
            // opponent of the side to move now.
            return Some(self.board.side_to_move().opponent());
        }
        None
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IllegalMove {
    #[error("the game is already over")]
    GameOver,
    #[error("move {0:?} is not legal in this position")]
    NotLegal(Move),
}
