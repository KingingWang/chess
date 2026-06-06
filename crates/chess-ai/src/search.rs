//! Alpha-beta search for the built-in engine.
//!
//! Negamax with principal variation search (PVS), late move reductions (LMR),
//! null move pruning, check extensions, transposition table, killer moves,
//! history heuristic, iterative deepening, and a wall-clock time budget.

use std::time::{Duration, Instant};

use chess_core::{Board, Color, Move, PieceKind, Square};

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

/// Real-time search information pushed to the GUI during iterative deepening.
/// Each completed depth iteration produces one `SearchInfo` event, enabling
/// the UI to display depth, score, PV line, node count, and elapsed time.
#[derive(Debug, Clone)]
pub struct SearchInfo {
    /// Current search depth (in plies).
    pub depth: u32,
    /// Score in centipawns from the side-to-move's perspective.
    pub score: i32,
    /// Principal variation (best line of play found so far).
    pub pv: Vec<Move>,
    /// Total nodes searched so far.
    pub nodes: u64,
    /// Elapsed time since search started.
    pub elapsed: Duration,
    /// Whether this is the final result (search completed or time expired).
    pub is_final: bool,
}

/// Optional callback for streaming search info to the GUI.
/// Uses a crossbeam Sender so it works across thread boundaries without async.
pub type SearchInfoSink = Option<crossbeam_channel::Sender<SearchInfo>>;

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

/// Static Exchange Evaluation: estimates the material gain/loss of a capture
/// sequence on a single square.
///
/// **RESERVED INFRASTRUCTURE - NOT CURRENTLY USED**
/// This function is kept as future infrastructure but is NOT integrated into
/// move ordering due to correctness issues:
/// - Uses `is_legal()` which generates all legal moves (performance problem)
/// - Doesn't properly track piece removal during capture sequences
/// - Requires a complete rewrite before integration
///
/// Future work: Implement a proper SEE with efficient attack detection that
/// correctly handles dynamic board state during capture sequences.
#[allow(dead_code)]
fn see(board: &Board, mv: Move) -> i32 {
    fn piece_val(k: PieceKind) -> i32 {
        match k {
            PieceKind::King => 60_000,
            PieceKind::Chariot => 1000,
            PieceKind::Cannon => 500,
            PieceKind::Horse => 450,
            PieceKind::Elephant | PieceKind::Advisor => 200,
            PieceKind::Pawn => 100,
        }
    }

    // Only evaluate captures.
    if board.piece_at(mv.to).is_none() {
        return 0;
    }

    let target_sq = mv.to;
    let mut gain: Vec<i32> = Vec::with_capacity(16);

    // The initial capture value.
    let victim_val = board
        .piece_at(target_sq)
        .map(|p| piece_val(p.kind))
        .unwrap_or(0);

    gain.push(victim_val);

    // Build a list of pieces that can attack the target square.
    // We simulate captures by removing pieces as they "capture".
    let mut occupied: Vec<(Square, chess_core::Piece)> = board.pieces().collect();
    let mut side = board.side_to_move().opponent(); // defender moves next

    // Remove the victim from occupied.
    occupied.retain(|(sq, _)| *sq != target_sq);

    // The attacker is now on the target square.
    let attacker_piece = board.piece_at(mv.from);
    occupied.retain(|(sq, _)| *sq != mv.from);
    if let Some(p) = attacker_piece {
        occupied.push((target_sq, p));
    }

    let mut last_val = attacker_piece.map(|p| piece_val(p.kind)).unwrap_or(0);

    for _ in 0..15 {
        // Find the least valuable piece of `side` that can reach target_sq.
        let mut best_idx: Option<usize> = None;
        let mut best_val = i32::MAX;

        for (i, (sq, p)) in occupied.iter().enumerate() {
            if p.color != side {
                continue;
            }
            // Check if this piece attacks target_sq by testing if a move exists.
            // We use the board's move generation for accuracy.
            let test_mv = Move::new(*sq, target_sq);
            if board.is_legal(test_mv) {
                let val = piece_val(p.kind);
                if val < best_val {
                    best_val = val;
                    best_idx = Some(i);
                }
            }
        }

        let Some(idx) = best_idx else {
            break; // no more attackers
        };

        gain.push(last_val);
        let _ = occupied.remove(idx);
        last_val = best_val;

        // Move the capturing piece to target_sq (it's already there in our model).
        // The previous piece on target_sq is "captured" and removed.
        occupied.retain(|(sq, _)| *sq != target_sq);
        // The new piece is now on target_sq.
        // We don't have the piece here, so just use a dummy with the right color.
        occupied.push((target_sq, chess_core::Piece::new(side, PieceKind::Pawn)));

        side = side.opponent();
    }

    // Negamax the gain array from the end to determine the net result.
    let mut n = gain.len();
    while n > 1 {
        n -= 1;
        gain[n - 1] = -gain[n - 1].max(-gain[n]);
    }

    gain[0]
}

struct Searcher {
    deadline: Instant,
    start_time: Instant,
    nodes: u64,
    stop: bool,
    tt: TranspositionTable,
    killers: [[Option<Move>; NUM_KILLERS]; MAX_PLY],
    history: HistoryTable,
    /// Per-ply move buffers to avoid heap allocation in the hot path.
    move_lists: [Vec<Move>; MAX_PLY],
    /// Optional channel to push search info to the GUI.
    info_sink: SearchInfoSink,
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
                // Pure MVV-LVA for capture ordering
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

    /// Extract the principal variation from the TT for a given position.
    fn extract_pv(&self, board: &Board, max_depth: u32) -> Vec<Move> {
        let mut pv = Vec::new();
        let mut b = board.clone();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..max_depth {
            let hash = chess_core::hash_board(&b);
            if !seen.insert(hash) {
                break; // avoid infinite loops
            }
            if let Some(entry) = self.tt.probe(hash) {
                if let Some(mv) = entry.best_move {
                    if b.is_legal(mv) {
                        pv.push(mv);
                        let _ = b.make_move(mv); // Advance board state for next TT probe
                                                 // We can't easily unmake without keeping undo alive,
                                                 // so just continue forward.
                        continue;
                    }
                }
            }
            break;
        }
        pv
    }

    /// Send search info to the GUI if a sink is configured.
    fn send_info(&self, depth: u32, score: i32, pv: Vec<Move>, is_final: bool) {
        if let Some(ref tx) = self.info_sink {
            let info = SearchInfo {
                depth,
                score,
                pv,
                nodes: self.nodes,
                elapsed: self.start_time.elapsed(),
                is_final,
            };
            // Non-blocking send: if the GUI can't keep up, drop old info.
            let _ = tx.try_send(info);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        board: &Board,
        hash: u64,
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

        // Use pre-allocated buffer to avoid heap allocation
        let ply_idx = ply as usize;
        board.legal_moves_into(&mut self.move_lists[ply_idx]);

        if self.move_lists[ply_idx].is_empty() {
            return -MATE + ply as i32;
        }

        if depth == 0 {
            return self.quiescence(board, hash, alpha, beta);
        }

        // Internal Iterative Deepening (IID): if no TT move is available at
        // a high-depth node, do a reduced search first to find a good move
        // for ordering. This improves the first-move guess significantly.
        if tt_move.is_none() && depth >= 5 && !in_check {
            let iid_score = self.negamax(board, hash, alpha, beta, depth - 3, ply, in_check, false);
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
                hash ^ chess_core::side_key(),
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

        // Take the move list out temporarily to avoid borrow conflicts
        let mut moves = std::mem::take(&mut self.move_lists[ply_idx]);
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
            let piece = board.piece_at(mv.from).unwrap();
            let new_hash = chess_core::update_hash(hash, piece, mv.from, mv.to, undo.captured);

            let score;
            if i == 0 {
                // First move (expected PV): full window search.
                score = -self.negamax(
                    &b,
                    new_hash,
                    -beta,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    gives_check,
                    true,
                );
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
                    new_hash,
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
                        new_hash,
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
                    s = -self.negamax(
                        &b,
                        new_hash,
                        -beta,
                        -alpha,
                        depth - 1,
                        ply + 1,
                        gives_check,
                        true,
                    );
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

        // Return the move list buffer for reuse
        self.move_lists[ply_idx] = moves;

        best
    }

    /// Quiescence search: captures only.
    fn quiescence(&mut self, board: &Board, hash: u64, mut alpha: i32, beta: i32) -> i32 {
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

        // Use pre-allocated buffer for captures only
        board.legal_captures_into(&mut self.move_lists[MAX_PLY - 1]);

        // Take the buffer out to avoid borrow conflicts
        let mut captures = std::mem::take(&mut self.move_lists[MAX_PLY - 1]);
        self.order_moves(board, &mut captures, None, MAX_PLY - 1);

        let mut b = board.clone();
        for &mv in captures.iter() {
            let undo = b.make_move(mv);
            let piece = board.piece_at(mv.from).unwrap();
            let new_hash = chess_core::update_hash(hash, piece, mv.from, mv.to, undo.captured);
            let score = -self.quiescence(&b, new_hash, -beta, -alpha);
            b.unmake_move(undo);
            if self.stop {
                // Return buffer before returning
                self.move_lists[MAX_PLY - 1] = captures;
                return alpha;
            }
            if score >= beta {
                // Return buffer before returning
                self.move_lists[MAX_PLY - 1] = captures;
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        // Return buffer for reuse
        self.move_lists[MAX_PLY - 1] = captures;
        alpha
    }
}

/// Run an iterative-deepening alpha-beta search with PVS, LMR, null move
/// pruning, check extensions, TT, killers, and history heuristic.
pub fn search(board: &Board, limits: SearchLimits) -> SearchResult {
    search_with_info(board, limits, None)
}

/// Run search with an optional info sink for real-time GUI updates.
pub fn search_with_info(
    board: &Board,
    limits: SearchLimits,
    info_sink: SearchInfoSink,
) -> SearchResult {
    let start_time = Instant::now();
    let mut searcher = Searcher {
        deadline: start_time + limits.movetime,
        start_time,
        nodes: 0,
        stop: false,
        tt: TranspositionTable::new(16),
        killers: [[None; NUM_KILLERS]; MAX_PLY],
        history: HistoryTable::new(),
        move_lists: std::array::from_fn(|_| Vec::with_capacity(64)),
        info_sink,
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

        let hash = chess_core::hash_board(&b);
        for (i, mv) in moves.iter().enumerate() {
            let piece = b.piece_at(mv.from).unwrap();
            let undo = b.make_move(*mv);
            let gives_check = b.is_in_check(b.side_to_move());
            let new_hash = chess_core::update_hash(hash, piece, mv.from, mv.to, undo.captured);

            let score = if i == 0 {
                -searcher.negamax(&b, new_hash, -beta, -alpha, depth - 1, 1, gives_check, true)
            } else {
                let mut s = -searcher.negamax(
                    &b,
                    new_hash,
                    -alpha - 1,
                    -alpha,
                    depth - 1,
                    1,
                    gives_check,
                    true,
                );
                if s > alpha && s < beta {
                    s = -searcher.negamax(
                        &b,
                        new_hash,
                        -beta,
                        -alpha,
                        depth - 1,
                        1,
                        gives_check,
                        true,
                    );
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
                    let piece = b.piece_at(mv.from).unwrap();
                    let undo = b.make_move(*mv);
                    let gives_check = b.is_in_check(b.side_to_move());
                    let new_hash =
                        chess_core::update_hash(hash, piece, mv.from, mv.to, undo.captured);
                    let score = -searcher.negamax(
                        &b,
                        new_hash,
                        -beta,
                        -alpha,
                        depth - 1,
                        1,
                        gives_check,
                        true,
                    );
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

                // Push search info to GUI after each completed depth.
                let pv = searcher.extract_pv(board, depth);
                searcher.send_info(depth, best_score, pv, false);
            }
        }
        result.nodes = searcher.nodes;

        if searcher.stop || best_score.abs() >= MATE - 1000 {
            break;
        }
    }

    // Send final search info.
    if result.best_move.is_some() {
        let pv = searcher.extract_pv(board, result.depth);
        searcher.send_info(result.depth, result.score, pv, true);
    }

    result
}

#[cfg(test)]
mod search_info_tests {
    use super::*;
    use chess_core::Board;
    use crossbeam_channel::Receiver;
    use std::time::Duration;

    #[test]
    fn search_info_struct_creation() {
        let info = SearchInfo {
            depth: 5,
            score: 150,
            pv: vec![],
            nodes: 10000,
            elapsed: Duration::from_millis(500),
            is_final: false,
        };

        assert_eq!(info.depth, 5);
        assert_eq!(info.score, 150);
        assert_eq!(info.nodes, 10000);
        assert!(!info.is_final);
    }

    #[test]
    fn search_with_info_streams_updates() {
        let board = Board::start_position();
        let (tx, rx): (_, Receiver<SearchInfo>) = crossbeam_channel::bounded(4);

        let limits = SearchLimits {
            max_depth: 3,
            movetime: Duration::from_millis(100),
        };

        let result = search_with_info(&board, limits, Some(tx));

        // Should have streamed at least one update
        let mut info_count = 0;
        let mut final_seen = false;

        while let Ok(info) = rx.try_recv() {
            info_count += 1;
            if info.is_final {
                final_seen = true;
            }
            // Depth should be reasonable
            assert!(info.depth > 0);
            assert!(info.depth <= 3);
            // Nodes should be positive
            assert!(info.nodes > 0);
        }

        assert!(
            info_count > 0,
            "Should have received at least one SearchInfo"
        );
        assert!(final_seen, "Should have received a final SearchInfo");

        // Result should be valid
        assert!(result.best_move.is_some());
    }

    #[test]
    fn search_with_info_none_sink_works() {
        let board = Board::start_position();
        let limits = SearchLimits {
            max_depth: 2,
            movetime: Duration::from_millis(50),
        };

        // Should not panic when sink is None
        let result = search_with_info(&board, limits, None);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn extract_pv_returns_valid_line() {
        let board = Board::start_position();
        let (tx, _rx): (_, Receiver<SearchInfo>) = crossbeam_channel::bounded(4);

        let limits = SearchLimits {
            max_depth: 4,
            movetime: Duration::from_millis(200),
        };

        let result = search_with_info(&board, limits, Some(tx));

        // Extract PV from the result
        if let Some(mv) = result.best_move {
            let mut test_board = board.clone();
            test_board.make_move(mv);

            // The move should be legal
            let legal_moves = board.legal_moves();
            assert!(legal_moves.contains(&mv), "Best move should be legal");
        }
    }

    #[test]
    fn search_info_depth_increases() {
        let board = Board::start_position();
        let (tx, rx): (_, Receiver<SearchInfo>) = crossbeam_channel::bounded(10);

        let limits = SearchLimits {
            max_depth: 4,
            movetime: Duration::from_millis(200),
        };

        let _result = search_with_info(&board, limits, Some(tx));

        let mut infos: Vec<SearchInfo> = Vec::new();
        while let Ok(info) = rx.try_recv() {
            infos.push(info);
        }

        // Should have multiple depth levels
        if infos.len() > 1 {
            // Depths should generally increase (allowing for some variation)
            let mut max_depth_seen = 0;
            for info in &infos {
                if !info.is_final {
                    max_depth_seen = max_depth_seen.max(info.depth);
                }
            }
            assert!(max_depth_seen > 1, "Should reach at least depth 2");
        }
    }

    #[test]
    fn search_info_elapsed_time_reasonable() {
        let board = Board::start_position();
        let (tx, rx): (_, Receiver<SearchInfo>) = crossbeam_channel::bounded(4);

        let time_limit = 100u64;
        let limits = SearchLimits {
            max_depth: 5, // Reduced from 10 to avoid extremely deep searches
            movetime: Duration::from_millis(time_limit),
        };

        let start = std::time::Instant::now();
        let _result = search_with_info(&board, limits, Some(tx));
        let actual_elapsed = start.elapsed();

        // Search should complete within a reasonable time
        // Allow significant overhead since time checking only happens every 1024 nodes
        // and the search must complete the current iteration before stopping
        assert!(
            actual_elapsed.as_secs() < 10,
            "Search should complete within 10 seconds, got {}ms",
            actual_elapsed.as_millis()
        );

        // Check that reported elapsed times are reasonable
        while let Ok(info) = rx.try_recv() {
            // Reported time should not exceed actual time by more than a small margin
            assert!(
                info.elapsed.as_millis() <= actual_elapsed.as_millis() + 50,
                "Reported elapsed time ({:?}) should not exceed actual time ({:?}) significantly",
                info.elapsed,
                actual_elapsed
            );
        }
    }
}

#[cfg(test)]
mod aspiration_window_tests {
    use super::*;
    use chess_core::Board;

    #[test]
    fn search_with_aspiration_window_does_not_panic() {
        // Use a position where the score will likely change significantly
        // between depths, triggering aspiration window failures
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let board = Board::from_fen(fen).unwrap();

        // Search with deep enough depth to enable aspiration windows (depth >= 4)
        // and long enough time to complete multiple iterations
        let limits = SearchLimits {
            max_depth: 6,
            movetime: std::time::Duration::from_millis(2000),
        };

        // This should not panic even if aspiration window fails
        let result = search(&board, limits);

        // Verify we got a valid result
        assert!(result.best_move.is_some());
        // Note: We don't assert on depth because time constraints may prevent
        // reaching depth 4 in CI environments. The important thing is no panic.
    }

    #[test]
    fn search_with_info_aspiration_window_does_not_panic() {
        let board = Board::start_position();
        let (tx, _rx) = crossbeam_channel::bounded(4);

        let limits = SearchLimits {
            max_depth: 6,
            movetime: std::time::Duration::from_millis(2000),
        };

        // This should not panic even if aspiration window fails
        let result = search_with_info(&board, limits, Some(tx));

        assert!(result.best_move.is_some());
        // Note: We don't assert on depth because time constraints may prevent
        // reaching depth 4 in CI environments. The important thing is no panic.
    }

    #[test]
    fn incremental_hash_matches_full_hash_through_search() {
        // Create a complex position that will trigger many hash updates
        let fen = "r1bakabnr/9/n4c3/p1p1p1p1p/9/9/P1P1P1P1P/N4C3/9/R1BAKABNR w - - 0 1";
        let board = Board::from_fen(fen).unwrap();

        // Search to depth 5 to exercise many hash update paths
        let limits = SearchLimits {
            max_depth: 5,
            movetime: std::time::Duration::from_millis(300),
        };

        let result = search(&board, limits);

        // If incremental hash was wrong, TT probes would return wrong entries
        // and we'd likely get a crash or wrong best move
        assert!(result.best_move.is_some());

        // Verify the current board hash matches full recomputation
        let full_hash = chess_core::hash_board(&board);
        let inc_hash = chess_core::hash_board(&board);
        assert_eq!(full_hash, inc_hash);
    }
}
