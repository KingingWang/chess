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
    /// One side ran out of time on the clock.
    Timeout,
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Game {
    board: Board,
    history: Vec<HistoryEntry>,
    result: Option<GameResult>,
    /// Halfmoves since the last capture (for the inactivity draw rule).
    halfmove_clock: u32,
}

/// A single entry in the game's move history.
///
/// Provides read-only access to the move played, whether it was a capture,
/// whether it delivered check, and the board position before the move.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HistoryEntry {
    pub(crate) undo: UndoState,
    /// FEN of the position *before* the move (placement + side only).
    pub(crate) fen_before: String,
    /// Did this move deliver check to the opponent?
    pub(crate) gave_check: bool,
    pub(crate) halfmove_clock_before: u32,
}

impl HistoryEntry {
    /// The move that was played.
    #[inline]
    pub fn mv(&self) -> Move {
        self.undo.mv
    }

    /// The piece captured by this move, if any.
    #[inline]
    pub fn captured(&self) -> Option<crate::piece::Piece> {
        self.undo.captured
    }

    /// Whether this move delivered check to the opponent.
    #[inline]
    pub fn gave_check(&self) -> bool {
        self.gave_check
    }

    /// FEN string of the position *before* this move was played.
    ///
    /// This is essential for move notation: Chinese notation needs the board
    /// state before the move to determine the piece glyph and disambiguation.
    #[inline]
    pub fn fen_before(&self) -> &str {
        &self.fen_before
    }
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

        self.halfmove_clock = if was_capture {
            0
        } else {
            self.halfmove_clock + 1
        };
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

    /// Force a specific result (e.g., timeout, external adjudication).
    pub fn force_result(&mut self, result: GameResult) {
        self.result = Some(result);
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
    pub fn repetition_count(&self) -> usize {
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

    /// Read-only access to the full move history.
    ///
    /// Each entry records the move played, the board FEN before the move,
    /// whether the move was a capture, and whether it gave check. This is
    /// the data needed for move notation, game replay, and save/load.
    #[inline]
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Iterator over just the moves played, in order.
    pub fn played_moves(&self) -> impl Iterator<Item = Move> + '_ {
        self.history.iter().map(|e| e.undo.mv)
    }

    /// Reconstruct the board state at a given ply (0 = initial position,
    /// `history_len()` = current position). Returns `None` if `ply` is out
    /// of range.
    pub fn board_at_ply(&self, ply: usize) -> Option<Board> {
        if ply > self.history.len() {
            return None;
        }
        // Replay from the initial position recorded in history.
        // If ply == 0, parse the first entry's fen_before (which is the
        // start position). If there is no history, ply must be 0 and we
        // return the current board.
        if self.history.is_empty() {
            return if ply == 0 {
                Some(self.board.clone())
            } else {
                None
            };
        }
        let mut board: Board = self.history[0]
            .fen_before
            .parse()
            .expect("stored FEN is always valid");
        for entry in self.history.iter().take(ply) {
            board.make_move(entry.undo.mv);
        }
        Some(board)
    }

    /// The current half-move clock (halfmoves since the last capture).
    #[inline]
    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IllegalMove {
    #[error("the game is already over")]
    GameOver,
    #[error("move {0:?} is not legal in this position")]
    NotLegal(Move),
}

// ===== Enhanced Move Validation =====

/// Detailed move validation error with user-friendly messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveValidationError {
    /// Game is already over.
    GameOver { result: String },
    /// No piece at the source square.
    NoPieceAtSource { from: String },
    /// Piece at source doesn't belong to the current player.
    WrongColor {
        piece_color: String,
        current_player: String,
    },
    /// Move is not legal for this piece type.
    NotLegalForPiece {
        piece_type: String,
        from: String,
        to: String,
    },
    /// Move would leave king in check.
    LeavesKingInCheck { piece_type: String },
    /// Move is blocked by another piece.
    Blocked { piece_type: String, reason: String },
    /// Generic illegal move.
    IllegalMove { from: String, to: String },
}

impl MoveValidationError {
    /// Get a user-friendly error message in Chinese.
    pub fn message_chinese(&self) -> String {
        match self {
            MoveValidationError::GameOver { result } => {
                format!("对局已结束 ({})", result)
            }
            MoveValidationError::NoPieceAtSource { from } => {
                format!("{}位置没有棋子", from)
            }
            MoveValidationError::WrongColor {
                piece_color,
                current_player,
            } => {
                format!("该棋子是{}的，现在轮到{}走棋", piece_color, current_player)
            }
            MoveValidationError::NotLegalForPiece {
                piece_type,
                from,
                to,
            } => {
                format!("{}不能从{}走到{}", piece_type, from, to)
            }
            MoveValidationError::LeavesKingInCheck { piece_type } => {
                format!("走{}会导致将/帅被将军", piece_type)
            }
            MoveValidationError::Blocked { piece_type, reason } => {
                format!("{}被阻挡: {}", piece_type, reason)
            }
            MoveValidationError::IllegalMove { from, to } => {
                format!("非法着法: {}→{}", from, to)
            }
        }
    }

    /// Get a user-friendly error message in English.
    pub fn message_english(&self) -> String {
        match self {
            MoveValidationError::GameOver { result } => {
                format!("Game over ({})", result)
            }
            MoveValidationError::NoPieceAtSource { from } => {
                format!("No piece at {}", from)
            }
            MoveValidationError::WrongColor {
                piece_color,
                current_player,
            } => {
                format!(
                    "Piece is {}, but it's {}'s turn",
                    piece_color, current_player
                )
            }
            MoveValidationError::NotLegalForPiece {
                piece_type,
                from,
                to,
            } => {
                format!("{} cannot move from {} to {}", piece_type, from, to)
            }
            MoveValidationError::LeavesKingInCheck { piece_type } => {
                format!("Moving {} would leave king in check", piece_type)
            }
            MoveValidationError::Blocked { piece_type, reason } => {
                format!("{} is blocked: {}", piece_type, reason)
            }
            MoveValidationError::IllegalMove { from, to } => {
                format!("Illegal move: {}→{}", from, to)
            }
        }
    }
}

impl Game {
    /// Validate a move and return detailed error information.
    pub fn validate_move(&self, mv: Move) -> Result<(), MoveValidationError> {
        use crate::piece::Color;

        // Check if game is over
        if let Some(result) = &self.result {
            let result_str = match result {
                GameResult::Win { winner, reason } => {
                    let winner_str = if *winner == Color::Red {
                        "红方"
                    } else {
                        "黑方"
                    };
                    let reason_str = match reason {
                        WinReason::Checkmate => "将死",
                        WinReason::Stalemate => "困毙",
                        WinReason::Resignation => "认输",
                        WinReason::PerpetualCheck => "长将",
                        WinReason::Timeout => "超时",
                    };
                    format!("{}{} ({})", winner_str, reason_str, reason_str)
                }
                GameResult::Draw(reason) => {
                    let reason_str = match reason {
                        DrawReason::Agreement => "协议和棋",
                        DrawReason::Repetition => "三次重复",
                        DrawReason::NoCapture => "60步无吃子",
                    };
                    format!("和棋 ({})", reason_str)
                }
            };
            return Err(MoveValidationError::GameOver { result: result_str });
        }

        // Check if there's a piece at the source
        let piece = match self.board.piece_at(mv.from) {
            Some(p) => p,
            None => {
                let from = format!("({}, {})", mv.from.file(), mv.from.rank());
                return Err(MoveValidationError::NoPieceAtSource { from });
            }
        };

        // Check if the piece belongs to the current player
        if piece.color != self.board.side_to_move() {
            let piece_color = if piece.color == Color::Red {
                "红方"
            } else {
                "黑方"
            };
            let current_player = if self.board.side_to_move() == Color::Red {
                "红方"
            } else {
                "黑方"
            };
            return Err(MoveValidationError::WrongColor {
                piece_color: piece_color.to_string(),
                current_player: current_player.to_string(),
            });
        }

        // Check if the move is legal
        if !self.board.is_legal(mv) {
            let piece_type = piece_glyph_chinese(piece.kind);
            let from = format!("({}, {})", mv.from.file(), mv.from.rank());
            let to = format!("({}, {})", mv.to.file(), mv.to.rank());
            return Err(MoveValidationError::NotLegalForPiece {
                piece_type: piece_type.to_string(),
                from,
                to,
            });
        }

        Ok(())
    }
}

/// Get Chinese name for a piece type.
fn piece_glyph_chinese(kind: crate::piece::PieceKind) -> &'static str {
    use crate::piece::PieceKind;
    match kind {
        PieceKind::King => "将/帅",
        PieceKind::Advisor => "士/仕",
        PieceKind::Elephant => "象/相",
        PieceKind::Horse => "马",
        PieceKind::Chariot => "车",
        PieceKind::Cannon => "炮",
        PieceKind::Pawn => "卒/兵",
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::{Board, Move, Square};

    #[test]
    fn test_validate_no_piece() {
        let game = Game::new();
        let mv = Move {
            from: Square::new(0, 5).unwrap(), // Empty square
            to: Square::new(0, 6).unwrap(),
        };
        let result = game.validate_move(mv);
        assert!(matches!(
            result,
            Err(MoveValidationError::NoPieceAtSource { .. })
        ));
    }

    #[test]
    fn test_validate_wrong_color() {
        let game = Game::new();
        // Red's turn, but try to move a black piece
        let mv = Move {
            from: Square::new(0, 9).unwrap(), // Black rook
            to: Square::new(0, 8).unwrap(),
        };
        let result = game.validate_move(mv);
        assert!(matches!(
            result,
            Err(MoveValidationError::WrongColor { .. })
        ));
    }

    #[test]
    fn test_validate_legal_move() {
        let game = Game::new();
        // Red pawn forward (legal move)
        let mv = Move {
            from: Square::new(0, 3).unwrap(), // Red pawn
            to: Square::new(0, 4).unwrap(),
        };
        let result = game.validate_move(mv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_illegal_move() {
        let game = Game::new();
        // Try to move rook through pieces (illegal)
        let mv = Move {
            from: Square::new(0, 0).unwrap(), // Red rook
            to: Square::new(0, 5).unwrap(),   // Blocked by pawn
        };
        let result = game.validate_move(mv);
        assert!(matches!(
            result,
            Err(MoveValidationError::NotLegalForPiece { .. })
        ));
    }

    #[test]
    fn test_error_messages() {
        let error = MoveValidationError::WrongColor {
            piece_color: "黑方".to_string(),
            current_player: "红方".to_string(),
        };
        let chinese = error.message_chinese();
        let english = error.message_english();
        assert!(chinese.contains("黑方"));
        assert!(chinese.contains("红方"));
        // English message uses the same Chinese terms for clarity
        assert!(english.contains("黑方"));
        assert!(english.contains("红方"));
    }

    #[test]
    fn test_game_over_validation() {
        let mut game = Game::new();
        game.result = Some(GameResult::Draw(DrawReason::Agreement));

        let mv = Move {
            from: Square::new(0, 0).unwrap(),
            to: Square::new(0, 1).unwrap(),
        };
        let result = game.validate_move(mv);
        assert!(matches!(result, Err(MoveValidationError::GameOver { .. })));
    }
}
