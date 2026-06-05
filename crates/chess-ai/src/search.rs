//! Alpha-beta search for the built-in engine.
//!
//! Negamax with principal variation search (PVS), late move reductions (LMR),
//! null move pruning, check extensions, transposition table, killer moves,
//! history heuristic, iterative deepening, and a wall-clock time budget.

use std::time::{Duration, Instant};

use chess_core::{Board, Color, Move, PieceKind};

use crate::eval::{evaluate, MATE};
use crate::tt::{Bound, TranspositionTable};

/// Maximum search depth for sizing arrays.
const MAX_PLY: usize = 128;

/// Number of killer move slots per ply.
const NUM_KILLERS: usize = 2;

/// Minimum depth at which null move pruning is applied.
const NULL_MOVE_MIN_DEPTH: u32 = 3;

/// Reduction amount for null move pruning.
const NULL_MOVE_REDUCTION: u32 = 2;

/// Minimum depth at which LMR is applied.
const LMR_MIN_DEPTH: u32 = 3;

/// Delta pruning margin for quiescence search. If stand-pat plus the value
/// of the best possible capture is still below alpha, skip this branch.
const DELTA_MARGIN: i32 = 1100; // slightly above chariot value

/// Futility margin per depth (centipawns). If a position is so far below alpha
/// that even a large positional gain cannot raise it above alpha, prune it.
const FUTILITY_MARGIN: [i32; 4] = [0, 200, 450, 750];

/// Minimum move index at which LMR kicks in (skip the first few moves).
const LMR_MIN_MOVE: usize = 3;

/// Late Move Pruning: at low depths, after searching this many quiet moves
/// without improvement, skip the rest.
const LMP_MOVE_COUNTS: [usize; 4] = [0, 5, 8, 12];

/// Adjust a mate score for TT storage: convert ply-relative to absolute.
///
/// Scores near ±MATE encode distance-from-root. To make them position-relative
/// (so the TT returns correct values regardless of which ply the position is
/// reached), we remove the ply component before storing.
#[inline]
fn score_to_tt(score: i32, ply: u32) -> i32 {
    if score > MATE - MAX_PLY as i32 {
        score + ply as i32
    } else if score < -MATE + MAX_PLY as i32 {
        score - ply as i32
    } else {
        score
    }
}

/// Reverse the TT adjustment: convert an absolute mate score back to
/// ply-relative for the current search path.
#[inline]
fn score_from_tt(score: i32, ply: u32) -> i32 {
    if score > MATE - MAX_PLY as i32 {
        score - ply as i32
    } else if score < -MATE + MAX_PLY as i32 {
        score + ply as i32
    } else {
        score
    }
}

/// Limits controlling a single search.
#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    /// Hard wall-clock budget.
    pub movetime: Duration,
    /// Maximum iterative-deepening depth (safety cap).
    pub max_depth: u32,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            movetime: Duration::from_secs(1),
            max_depth: 64,
        }
    }
}

/// Result of a search.
#[derive(Debug, Clone, Copy)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u32,
    pub nodes: u64,
}

/// History heuristic table.
struct HistoryTable {
    table: [[i32; 90]; 14],
}

impl HistoryTable {
    fn new() -> Self {
        HistoryTable {
            table: [[0; 90]; 14],
        }
    }

    #[inline]
    fn piece_index(piece: chess_core::Piece) -> usize {
        let c = match piece.color {
            Color::Red => 0,
            Color::Black => 7,
        };
        let k = match piece.kind {
            PieceKind::King => 0,
            PieceKind::Advisor => 1,
            PieceKind::Elephant => 2,
            PieceKind::Horse => 3,
            PieceKind::Chariot => 4,
            PieceKind::Cannon => 5,
            PieceKind::Pawn => 6,
        };
        c + k
    }

    #[inline]
    fn record_cutoff(&mut self, board: &Board, mv: Move, depth: u32) {
        if let Some(piece) = board.piece_at(mv.from) {
            let idx = Self::piece_index(piece);
            self.table[idx][mv.to.index()] += (depth * depth) as i32;
            if self.table[idx][mv.to.index()] > 1_000_000 {
                for sq in 0..90 {
                    self.table[idx][sq] /= 2;
                }
            }
        }
    }

    #[inline]
    fn score(&self, board: &Board, mv: Move) -> i32 {
        if let Some(piece) = board.piece_at(mv.from) {
            self.table[Self::piece_index(piece)][mv.to.index()]
        } else {
            0
        }
    }
}

struct Searcher {
    deadline: Instant,
    nodes: u64,
    stop: bool,
    tt: TranspositionTable,
    killers: [[Option<Move>; NUM_KILLERS]; MAX_PLY],
    history: HistoryTable,
}

impl Searcher {
    #[inline]
    fn time_up(&mut self) -> bool {
        if self.nodes & 0x3FF == 0 && Instant::now() >= self.deadline {
            self.stop = true;
        }
        self.stop
    }

    #[inline]
    fn store_killer(&mut self, ply: usize, mv: Move) {
        if ply >= MAX_PLY {
            return;
        }
        if self.killers[ply][0] == Some(mv) {
            return;
        }
        self.killers[ply][1] = self.killers[ply][0];
        self.killers[ply][0] = Some(mv);
    }

    fn order_moves(&self, board: &Board, moves: &mut [Move], tt_move: Option<Move>, ply: usize) {
        fn mvv_lva_val(k: PieceKind) -> i32 {
            match k {
                PieceKind::King => 6000,
                PieceKind::Chariot => 600,
                PieceKind::Cannon => 300,
                PieceKind::Horse => 270,
                PieceKind::Elephant | PieceKind::Advisor => 120,
                PieceKind::Pawn => 60,
            }
        }

        let killers = if ply < MAX_PLY {
            self.killers[ply]
        } else {
            [None; NUM_KILLERS]
        };

        moves.sort_by_cached_key(|m| {
            if tt_move == Some(*m) {
                return -10_000_000i32;
            }
            let is_capture = board.piece_at(m.to).is_some();
            if is_capture {
                let victim = board
                    .piece_at(m.to)
                    .map(|p| mvv_lva_val(p.kind))
                    .unwrap_or(0);
                let attacker = board
                    .piece_at(m.from)
                    .map(|p| mvv_lva_val(p.kind))
                    .unwrap_or(0);
                return -(5_000_000 + victim * 16 - attacker);
            }
            if killers[0] == Some(*m) {
                return -4_000_000;
            }
            if killers[1] == Some(*m) {
                return -3_900_000;
            }
            -self.history.score(board, *m)
        });
    }

    /// Compute the LMR reduction based on depth and move index.
    #[inline]
    fn lmr_reduction(depth: u32, move_index: usize) -> u32 {
        // Simple logarithmic reduction formula.
        if depth >= 6 && move_index >= 6 {
            2
        } else {
            1
        }
    }

    /// Check if null move pruning is safe (not in check, has non-pawn material).
    #[inline]
    fn can_do_null_move(board: &Board) -> bool {
        let side = board.side_to_move();
        // Must have at least one piece other than king and pawns (avoid
        // zugzwang in king+pawn endgames).
        board.pieces().any(|(_, p)| {
            p.color == side
                && !matches!(
                    p.kind,
                    PieceKind::King | PieceKind::Pawn | PieceKind::Advisor | PieceKind::Elephant
                )
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        mut depth: u32,
        ply: u32,
        in_check: bool,
        allow_null: bool,
    ) -> i32 {
        if self.time_up() {
            return 0;
        }
        self.nodes += 1;

        // Check extension: extend by 1 ply when in check.
        if in_check {
            depth += 1;
        }

        let hash = chess_core::hash_board(board);

        // TT probe.
        let mut tt_move: Option<Move> = None;
        if let Some(entry) = self.tt.probe(hash) {
            tt_move = entry.best_move;
            if entry.depth >= depth as u16 {
                let tt_score = score_from_tt(entry.score, ply);
                match entry.bound {
                    Bound::Exact => return tt_score,
                    Bound::Lower => {
                        if tt_score >= beta {
                            return tt_score;
                        }
                        if tt_score > alpha {
                            alpha = tt_score;
                        }
                    }
                    Bound::Upper => {
                        if tt_score <= alpha {
                            return tt_score;
                        }
                    }
                }
            }
        }

        let mut moves = board.legal_moves();

        if moves.is_empty() {
            return -MATE + ply as i32;
        }

        if depth == 0 {
            return self.quiescence(board, alpha, beta);
        }

        // Internal Iterative Deepening (IID): if no TT move is available at
        // a high-depth node, do a reduced search first to find a good move
        // for ordering. This improves the first-move guess significantly.
        if tt_move.is_none() && depth >= 5 && !in_check {
            let iid_score = self.negamax(board, alpha, beta, depth - 3, ply, in_check, false);
            if !self.stop {
                if let Some(entry) = self.tt.probe(hash) {
                    tt_move = entry.best_move;
                }
            }
            let _ = iid_score; // only care about the TT side-effect
        }

        // Null move pruning: if we can "pass" and still beat beta, the
        // position is likely so good we can prune.
        if allow_null && !in_check && depth >= NULL_MOVE_MIN_DEPTH && Self::can_do_null_move(board)
        {
            // Make a "null move" by just flipping the side to move.
            let mut null_board = board.clone();
            null_board.set_side_to_move(board.side_to_move().opponent());
            let null_score = -self.negamax(
                &null_board,
                -beta,
                -beta + 1,
                depth.saturating_sub(NULL_MOVE_REDUCTION + 1),
                ply + 1,
                false,
                false, // no consecutive null moves
            );
            if null_score >= beta {
                return beta;
            }
        }

        self.order_moves(board, &mut moves, tt_move, ply as usize);

        // Futility pruning: at shallow depths, if the static eval is far
        // below alpha, skip quiet moves (they're unlikely to raise alpha).
        let futile = !in_check && depth <= 3 && (depth as usize) < FUTILITY_MARGIN.len() && {
            let static_eval = evaluate(board, board.side_to_move());
            static_eval + FUTILITY_MARGIN[depth as usize] <= alpha
        };

        let mut best = -MATE * 2;
        let mut best_move = moves[0];
        let mut b = board.clone();
        let orig_alpha = alpha;

        for (i, mv) in moves.iter().enumerate() {
            let is_capture = b.piece_at(mv.to).is_some();

            // Futility pruning: skip quiet moves at frontier nodes.
            if futile && i > 0 && !is_capture {
                continue;
            }

            // Late Move Pruning: at low depths, skip late quiet moves.
            if !in_check
                && !is_capture
                && depth <= 3
                && (depth as usize) < LMP_MOVE_COUNTS.len()
                && i >= LMP_MOVE_COUNTS[depth as usize]
            {
                continue;
            }

            let undo = b.make_move(*mv);
            let gives_check = b.is_in_check(b.side_to_move());

            let score;
            if i == 0 {
                // First move (expected PV): full window search.
                score = -self.negamax(&b, -beta, -alpha, depth - 1, ply + 1, gives_check, true);
            } else {
                // Late Move Reduction for quiet moves.
                let mut reduction = 0u32;
                if !is_capture
                    && !gives_check
                    && !in_check
                    && depth >= LMR_MIN_DEPTH
                    && i >= LMR_MIN_MOVE
                {
                    reduction = Self::lmr_reduction(depth, i);
                }

                // PVS: search with null window first.
                let reduced_depth = (depth - 1).saturating_sub(reduction);
                let mut s = -self.negamax(
                    &b,
                    -alpha - 1,
                    -alpha,
                    reduced_depth,
                    ply + 1,
                    gives_check,
                    true,
                );

                // Re-search at full depth if the reduced search raised alpha.
                if s > alpha && reduction > 0 {
                    s = -self.negamax(
                        &b,
                        -alpha - 1,
                        -alpha,
                        depth - 1,
                        ply + 1,
                        gives_check,
                        true,
                    );
                }

                // Re-search with full window if still above alpha (PV node).
                if s > alpha && s < beta {
                    s = -self.negamax(&b, -beta, -alpha, depth - 1, ply + 1, gives_check, true);
                }
                score = s;
            }

            b.unmake_move(undo);

            if self.stop {
                return best.max(alpha);
            }
            if score > best {
                best = score;
                best_move = *mv;
            }
            if best > alpha {
                alpha = best;
            }
            if alpha >= beta {
                if !is_capture {
                    self.store_killer(ply as usize, *mv);
                    self.history.record_cutoff(board, *mv, depth);
                }
                break;
            }
        }

        // Store in TT.
        if !self.stop {
            let bound = if best <= orig_alpha {
                Bound::Upper
            } else if best >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };
            self.tt.store(
                hash,
                Some(best_move),
                score_to_tt(best, ply),
                depth as u16,
                bound,
            );
        }

        best
    }

    /// Quiescence search: captures only.
    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        if self.time_up() {
            return 0;
        }
        self.nodes += 1;

        let stand_pat = evaluate(board, board.side_to_move());
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Delta pruning: if even capturing the most valuable piece cannot
        // raise the score above alpha, bail out early.
        if stand_pat + DELTA_MARGIN < alpha {
            return alpha;
        }

        let mut captures: Vec<Move> = board
            .legal_moves()
            .into_iter()
            .filter(|m| board.piece_at(m.to).is_some())
            .collect();
        self.order_moves(board, &mut captures, None, MAX_PLY - 1);

        let mut b = board.clone();
        for mv in captures {
            let undo = b.make_move(mv);
            let score = -self.quiescence(&b, -beta, -alpha);
            b.unmake_move(undo);
            if self.stop {
                return alpha;
            }
            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
        alpha
    }
}

/// Run an iterative-deepening alpha-beta search with PVS, LMR, null move
/// pruning, check extensions, TT, killers, and history heuristic.
pub fn search(board: &Board, limits: SearchLimits) -> SearchResult {
    let mut searcher = Searcher {
        deadline: Instant::now() + limits.movetime,
        nodes: 0,
        stop: false,
        tt: TranspositionTable::new(16),
        killers: [[None; NUM_KILLERS]; MAX_PLY],
        history: HistoryTable::new(),
    };

    let root_moves = board.legal_moves();
    let mut result = SearchResult {
        best_move: root_moves.first().copied(),
        score: 0,
        depth: 0,
        nodes: 0,
    };
    if root_moves.is_empty() {
        return result;
    }

    let mut b = board.clone();
    for depth in 1..=limits.max_depth {
        // Aspiration windows: use a narrow window around the previous score
        // at deeper iterations for faster cutoffs.
        let (mut alpha, mut beta) = if depth >= 4 && result.score.abs() < MATE - 1000 {
            (result.score - 50, result.score + 50)
        } else {
            (-MATE * 2, MATE * 2)
        };
        let aspiration_alpha = alpha;
        let aspiration_beta = beta;
        let mut best_move = result.best_move;
        let mut best_score = -MATE * 2;

        let mut moves = root_moves.clone();
        searcher.order_moves(&b, &mut moves, result.best_move, 0);
        if let Some(pv) = result.best_move {
            if let Some(pos) = moves.iter().position(|&m| m == pv) {
                moves.swap(0, pos);
            }
        }

        for (i, mv) in moves.iter().enumerate() {
            let undo = b.make_move(*mv);
            let gives_check = b.is_in_check(b.side_to_move());

            let score = if i == 0 {
                -searcher.negamax(&b, -beta, -alpha, depth - 1, 1, gives_check, true)
            } else {
                let mut s =
                    -searcher.negamax(&b, -alpha - 1, -alpha, depth - 1, 1, gives_check, true);
                if s > alpha && s < beta {
                    s = -searcher.negamax(&b, -beta, -alpha, depth - 1, 1, gives_check, true);
                }
                s
            };

            b.unmake_move(undo);

            if searcher.stop {
                break;
            }
            if i == 0 || score > best_score {
                best_score = score;
                best_move = Some(*mv);
            }
            if best_score > alpha {
                alpha = best_score;
            }
            if alpha >= beta {
                break;
            }
        }

        if !searcher.stop {
            // If aspiration window failed (score outside window), re-search
            // with full window.
            if best_score <= aspiration_alpha || best_score >= aspiration_beta {
                alpha = -MATE * 2;
                beta = MATE * 2;
                best_score = -MATE * 2;
                best_move = result.best_move;

                let mut moves = root_moves.clone();
                searcher.order_moves(&b, &mut moves, result.best_move, 0);
                if let Some(pv) = result.best_move {
                    if let Some(pos) = moves.iter().position(|&m| m == pv) {
                        moves.swap(0, pos);
                    }
                }

                for (i, mv) in moves.iter().enumerate() {
                    let undo = b.make_move(*mv);
                    let gives_check = b.is_in_check(b.side_to_move());
                    let score =
                        -searcher.negamax(&b, -beta, -alpha, depth - 1, 1, gives_check, true);
                    b.unmake_move(undo);
                    if searcher.stop {
                        break;
                    }
                    if i == 0 || score > best_score {
                        best_score = score;
                        best_move = Some(*mv);
                    }
                    if best_score > alpha {
                        alpha = best_score;
                    }
                    if alpha >= beta {
                        break;
                    }
                }
            }
            if !searcher.stop {
                result.best_move = best_move;
                result.score = best_score;
                result.depth = depth;
            }
        }
        result.nodes = searcher.nodes;

        if searcher.stop || best_score.abs() >= MATE - 1000 {
            break;
        }
    }

    result
}
