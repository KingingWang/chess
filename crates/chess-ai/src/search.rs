//! Alpha-beta search for the built-in engine.
//!
//! Negamax with alpha-beta pruning, iterative deepening, MVV-LVA capture
//! ordering, and a wall-clock time budget. This is the self-developed fallback
//! used when no external UCI engine (Pikafish) is configured. It is a correct,
//! reasonably strong club-level engine — NOT a 2600-ELO NNUE engine; reaching
//! that bar is delegated to Pikafish (see crate docs).

use std::time::{Duration, Instant};

use chess_core::{Board, Move};

use crate::eval::{evaluate, MATE};

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

struct Searcher {
    deadline: Instant,
    nodes: u64,
    stop: bool,
}

impl Searcher {
    #[inline]
    fn time_up(&mut self) -> bool {
        // Check the clock periodically to avoid syscall overhead.
        if self.nodes & 0x3FF == 0 && Instant::now() >= self.deadline {
            self.stop = true;
        }
        self.stop
    }

    fn order_moves(board: &Board, moves: &mut [Move]) {
        // MVV-LVA: prioritise capturing valuable victims with cheap attackers.
        fn val(k: chess_core::PieceKind) -> i32 {
            use chess_core::PieceKind::*;
            match k {
                King => 6000,
                Chariot => 600,
                Cannon => 300,
                Horse => 270,
                Elephant | Advisor => 120,
                Pawn => 60,
            }
        }
        moves.sort_by_cached_key(|m| {
            let victim = board.piece_at(m.to).map(|p| val(p.kind)).unwrap_or(0);
            let attacker = board.piece_at(m.from).map(|p| val(p.kind)).unwrap_or(0);
            // Descending: larger (victim*16 - attacker) first.
            -(victim * 16 - attacker)
        });
    }

    fn negamax(&mut self, board: &Board, mut alpha: i32, beta: i32, depth: u32, ply: u32) -> i32 {
        if self.time_up() {
            return 0;
        }
        self.nodes += 1;

        let mut moves = board.legal_moves();

        if moves.is_empty() {
            // No legal move: checkmate or stalemate — both are losses in
            // Xiangqi. Prefer faster mates (closer to root).
            return -MATE + ply as i32;
        }

        if depth == 0 {
            return self.quiescence(board, alpha, beta);
        }

        Self::order_moves(board, &mut moves);

        let mut best = -MATE * 2;
        let mut b = board.clone();
        for mv in moves {
            let undo = b.make_move(mv);
            let score = -self.negamax(&b, -beta, -alpha, depth - 1, ply + 1);
            b.unmake_move(undo);

            if self.stop {
                return best.max(alpha);
            }
            if score > best {
                best = score;
            }
            if best > alpha {
                alpha = best;
            }
            if alpha >= beta {
                break; // beta cutoff
            }
        }
        best
    }

    /// Quiescence: extend the search through captures to avoid the horizon
    /// effect, using a stand-pat lower bound.
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

        // Only explore captures.
        let mut captures: Vec<Move> = board
            .legal_moves()
            .into_iter()
            .filter(|m| board.piece_at(m.to).is_some())
            .collect();
        Self::order_moves(board, &mut captures);

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

/// Run an iterative-deepening alpha-beta search and return the best move.
pub fn search(board: &Board, limits: SearchLimits) -> SearchResult {
    let mut searcher = Searcher {
        deadline: Instant::now() + limits.movetime,
        nodes: 0,
        stop: false,
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
        let mut alpha = -MATE * 2;
        let beta = MATE * 2;
        let mut best_move = result.best_move;
        let mut best_score = -MATE * 2;

        let mut moves = root_moves.clone();
        // Order by MVV-LVA, then float the previous iteration's best move to
        // the front for a better first window.
        Searcher::order_moves(&b, &mut moves);
        if let Some(pv) = result.best_move {
            if let Some(pos) = moves.iter().position(|&m| m == pv) {
                moves.swap(0, pos);
            }
        }

        for (i, mv) in moves.iter().enumerate() {
            let undo = b.make_move(*mv);
            let score = -searcher.negamax(&b, -beta, -alpha, depth - 1, 1);
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
        }

        if !searcher.stop {
            result.best_move = best_move;
            result.score = best_score;
            result.depth = depth;
        }
        result.nodes = searcher.nodes;

        // Stop if out of time or a forced mate is proven.
        if searcher.stop || best_score.abs() >= MATE - 1000 {
            break;
        }
    }

    result
}
