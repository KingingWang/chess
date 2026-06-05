//! Static evaluation for the built-in engine.
//!
//! Scores are in centipawns from the perspective of [`Color::Red`]; positive
//! favours Red. The search negates as appropriate. The evaluation combines:
//!
//! 1. **Material** — standard piece values.
//! 2. **Piece-square tables** — positional bonuses for each piece type.
//! 3. **Mobility** — bonus for pieces with more legal destinations.
//! 4. **King safety** — bonus for having advisors/elephants protecting the king.
//! 5. **Pawn structure** — connected pawns, advanced pawns, river-crossing bonuses.

use chess_core::square;
use chess_core::{Board, Color, Piece, PieceKind, Square};

/// A decisive (mate) score, kept well below `i32::MAX` so `±MATE - ply` stays
/// representable and orderable.
pub const MATE: i32 = 1_000_000;

// ---------------------------------------------------------------------------
// Material values
// ---------------------------------------------------------------------------

#[inline]
fn material(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::King => 60_000,
        PieceKind::Chariot => 1_000,
        PieceKind::Cannon => 500,
        PieceKind::Horse => 450,
        PieceKind::Elephant => 200,
        PieceKind::Advisor => 200,
        PieceKind::Pawn => 100,
    }
}

// ---------------------------------------------------------------------------
// Piece-square tables (indexed from RED's view, rank 0 first)
// ---------------------------------------------------------------------------

#[rustfmt::skip]
const PAWN_PST: [i32; 90] = [
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     2,  0,  4,  0,  6,  0,  4,  0,  2,
     6, 12, 18, 24, 30, 24, 18, 12,  6,
    20, 30, 40, 50, 55, 50, 40, 30, 20,
    20, 30, 45, 55, 60, 55, 45, 30, 20,
    20, 28, 40, 50, 55, 50, 40, 28, 20,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const HORSE_PST: [i32; 90] = [
     0, -4,  0,  0,  0,  0,  0, -4,  0,
    -4,  2,  6,  6,  4,  6,  6,  2, -4,
     2,  6, 10, 12, 12, 12, 10,  6,  2,
     4, 12, 14, 16, 16, 16, 14, 12,  4,
     6, 14, 18, 20, 22, 20, 18, 14,  6,
     6, 14, 18, 20, 22, 20, 18, 14,  6,
     4, 12, 16, 18, 18, 18, 16, 12,  4,
     2,  8, 12, 14, 14, 14, 12,  8,  2,
     0,  4,  8, 10, 10, 10,  8,  4,  0,
     0,  0,  4,  6,  6,  6,  4,  0,  0,
];

#[rustfmt::skip]
const CHARIOT_PST: [i32; 90] = [
    -2,  4,  4,  8,  8,  8,  4,  4, -2,
     4,  8,  8, 12, 12, 12,  8,  8,  4,
     2,  6,  6, 10, 10, 10,  6,  6,  2,
     6, 10, 10, 14, 14, 14, 10, 10,  6,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 12, 12, 16, 16, 16, 12, 12,  8,
     8, 14, 14, 18, 18, 18, 14, 14,  8,
    10, 14, 14, 18, 18, 18, 14, 14, 10,
     6, 10, 10, 14, 14, 14, 10, 10,  6,
];

#[rustfmt::skip]
const CANNON_PST: [i32; 90] = [
     2,  2,  0, -2, -4, -2,  0,  2,  2,
     0,  2,  0,  2,  4,  2,  0,  2,  0,
     2,  2,  0,  6,  8,  6,  0,  2,  2,
     0,  0,  2,  8, 10,  8,  2,  0,  0,
     0,  2,  4,  8, 12,  8,  4,  2,  0,
     0,  0,  4,  8, 12,  8,  4,  0,  0,
     2,  2,  4, 10, 12, 10,  4,  2,  2,
     2,  2,  4,  8, 10,  8,  4,  2,  2,
     0,  2,  2,  6,  8,  6,  2,  2,  0,
     0,  0,  2,  4,  6,  4,  2,  0,  0,
];

#[inline]
fn pst_value(kind: PieceKind, sq: Square, color: Color) -> i32 {
    let idx = match color {
        Color::Red => sq.index(),
        Color::Black => {
            let mirrored_rank = chess_core::square::RANKS - 1 - sq.rank();
            (mirrored_rank * chess_core::square::FILES + sq.file()) as usize
        }
    };
    match kind {
        PieceKind::Pawn => PAWN_PST[idx],
        PieceKind::Horse => HORSE_PST[idx],
        PieceKind::Chariot => CHARIOT_PST[idx],
        PieceKind::Cannon => CANNON_PST[idx],
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Mobility evaluation
// ---------------------------------------------------------------------------

/// Count pseudo-legal destination squares for a piece (a fast mobility proxy).
/// Uses simplified counting — does not generate full legal moves (that's too
/// expensive for eval), but counts reachable squares based on piece type.
fn piece_mobility(board: &Board, sq: Square, piece: Piece) -> i32 {
    let mut count = 0i32;
    let me = piece.color;
    let (f, r) = (sq.file() as i8, sq.rank() as i8);

    match piece.kind {
        PieceKind::Chariot => {
            // Count open squares in each direction.
            for (df, dr) in [(0i8, 1i8), (0, -1), (1, 0), (-1, 0)] {
                let (mut cf, mut cr) = (f + df, r + dr);
                while let Some(to) = Square::try_new(cf, cr) {
                    match board.piece_at(to) {
                        None => count += 1,
                        Some(p) => {
                            if p.color != me {
                                count += 1;
                            }
                            break;
                        }
                    }
                    cf += df;
                    cr += dr;
                }
            }
        }
        PieceKind::Horse => {
            const MOVES: [((i8, i8), (i8, i8)); 8] = [
                ((0, 1), (1, 2)),
                ((0, 1), (-1, 2)),
                ((0, -1), (1, -2)),
                ((0, -1), (-1, -2)),
                ((1, 0), (2, 1)),
                ((1, 0), (2, -1)),
                ((-1, 0), (-2, 1)),
                ((-1, 0), (-2, -1)),
            ];
            for ((lf, lr), (tf, tr)) in MOVES {
                let leg = match Square::try_new(f + lf, r + lr) {
                    Some(s) => s,
                    None => continue,
                };
                if board.piece_at(leg).is_some() {
                    continue;
                }
                if let Some(to) = Square::try_new(f + tf, r + tr) {
                    match board.piece_at(to) {
                        Some(p) if p.color == me => {}
                        _ => count += 1,
                    }
                }
            }
        }
        PieceKind::Cannon => {
            for (df, dr) in [(0i8, 1i8), (0, -1), (1, 0), (-1, 0)] {
                let (mut cf, mut cr) = (f + df, r + dr);
                let mut jumped = false;
                while let Some(to) = Square::try_new(cf, cr) {
                    match board.piece_at(to) {
                        None => {
                            if !jumped {
                                count += 1;
                            }
                        }
                        Some(p) => {
                            if !jumped {
                                jumped = true;
                            } else {
                                if p.color != me {
                                    count += 1;
                                }
                                break;
                            }
                        }
                    }
                    cf += df;
                    cr += dr;
                }
            }
        }
        _ => {} // King, Advisor, Elephant, Pawn — mobility is not very meaningful.
    }
    count
}

/// Weight for each piece type's mobility bonus (centipawns per accessible square).
#[inline]
fn mobility_weight(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Chariot => 3,
        PieceKind::Horse => 5,
        PieceKind::Cannon => 2,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// King safety evaluation
// ---------------------------------------------------------------------------

/// Evaluate king safety: bonus for having defensive pieces (advisors/elephants)
/// near the king, penalty for exposed kings.
fn king_safety(board: &Board, color: Color) -> i32 {
    let king_sq = match board.king_square(color) {
        Some(sq) => sq,
        None => return -500, // Missing king is catastrophic.
    };

    let mut score = 0i32;

    // Count advisors and elephants for this color (defensive pieces).
    let mut advisors = 0;
    let mut elephants = 0;
    for (_, piece) in board.pieces() {
        if piece.color != color {
            continue;
        }
        match piece.kind {
            PieceKind::Advisor => advisors += 1,
            PieceKind::Elephant => elephants += 1,
            _ => {}
        }
    }

    // Full complement: 2 advisors + 2 elephants = best defense.
    // Missing defenders = penalty.
    score += advisors * 15;
    score += elephants * 10;

    // Penalty if king is not in the center file of the palace (exposed).
    let center_file: u8 = 4;
    if king_sq.file() != center_file {
        score -= 10;
    }

    // Bonus for king being on its home rank (less exposed).
    let home_rank = match color {
        Color::Red => 0,
        Color::Black => 9,
    };
    if king_sq.rank() == home_rank {
        score += 8;
    }

    score
}

// ---------------------------------------------------------------------------
// Pawn structure evaluation
// ---------------------------------------------------------------------------

/// Evaluate pawn structure: connected pawns, advanced pawns crossing the river.
fn pawn_structure(board: &Board, color: Color) -> i32 {
    let mut score = 0i32;
    let mut pawn_files: Vec<u8> = Vec::new();

    for (sq, piece) in board.pieces() {
        if piece.color != color || piece.kind != PieceKind::Pawn {
            continue;
        }

        pawn_files.push(sq.file());

        // Bonus for crossed-river pawns (already handled partially by PST,
        // but add an extra dynamic bonus for multiple crossed pawns).
        if square::crossed_river(sq, color) {
            score += 15;

            // Extra bonus for central crossed pawns (files 3-5).
            if (3..=5).contains(&sq.file()) {
                score += 8;
            }
        }
    }

    // Connected pawns: bonus when two pawns are on adjacent files.
    pawn_files.sort_unstable();
    for i in 1..pawn_files.len() {
        if pawn_files[i] == pawn_files[i - 1] + 1 {
            score += 10; // adjacent file bonus
        }
    }

    score
}

// ---------------------------------------------------------------------------
// Endgame heuristics
// ---------------------------------------------------------------------------

/// Count total material (excluding kings) for a side.
fn total_material(board: &Board, color: Color) -> i32 {
    board
        .pieces()
        .filter(|(_, p)| p.color == color && p.kind != PieceKind::King)
        .map(|(_, p)| material(p.kind))
        .sum()
}

/// Is this an endgame position? (combined non-king material below threshold)
fn is_endgame(board: &Board) -> bool {
    let red_mat = total_material(board, Color::Red);
    let black_mat = total_material(board, Color::Black);
    // Endgame: both sides have less than rook + horse worth of material
    // (excluding king value).
    red_mat + black_mat < 3000
}

/// In endgame positions, drive the losing king toward the edge and
/// the winning king toward the center for easier mating patterns.
fn endgame_king_distance(board: &Board) -> i32 {
    let red_king = match board.king_square(Color::Red) {
        Some(sq) => sq,
        None => return 0,
    };
    let black_king = match board.king_square(Color::Black) {
        Some(sq) => sq,
        None => return 0,
    };

    // King proximity: closer kings favor the stronger side (easier to mate).
    let file_dist = (red_king.file() as i32 - black_king.file() as i32).abs();
    let rank_dist = (red_king.rank() as i32 - black_king.rank() as i32).abs();
    let proximity = 14 - file_dist - rank_dist;

    // Center distance penalty for the weaker side's king.
    // Palace center is file 4, rank 1 (Red) or rank 8 (Black).
    let red_mat = total_material(board, Color::Red);
    let black_mat = total_material(board, Color::Black);

    if red_mat > black_mat + 200 {
        // Red is winning: penalize Black king for being in corner of palace,
        // and reward Red king for approaching Black king.
        let bk_center_dist =
            (black_king.file() as i32 - 4).abs() + (black_king.rank() as i32 - 8).abs();
        bk_center_dist * 5 + proximity * 2
    } else if black_mat > red_mat + 200 {
        // Black is winning.
        let rk_center_dist =
            (red_king.file() as i32 - 4).abs() + (red_king.rank() as i32 - 1).abs();
        -(rk_center_dist * 5 + proximity * 2)
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Main evaluation function
// ---------------------------------------------------------------------------

/// Static evaluation from `side`'s perspective (positive = good for `side`).
pub fn evaluate(board: &Board, side: Color) -> i32 {
    let mut red = 0i32;

    // Material + PST + Mobility.
    for (sq, piece) in board.pieces() {
        let mat = material(piece.kind) + pst_value(piece.kind, sq, piece.color);
        let mob = piece_mobility(board, sq, piece) * mobility_weight(piece.kind);
        let v = mat + mob;
        match piece.color {
            Color::Red => red += v,
            Color::Black => red -= v,
        }
    }

    // King safety.
    red += king_safety(board, Color::Red);
    red -= king_safety(board, Color::Black);

    // Pawn structure.
    red += pawn_structure(board, Color::Red);
    red -= pawn_structure(board, Color::Black);

    // Endgame adjustments.
    if is_endgame(board) {
        red += endgame_king_distance(board);
    }

    match side {
        Color::Red => red,
        Color::Black => -red,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Board;

    #[test]
    fn start_position_is_roughly_equal() {
        let b = Board::start_position();
        let score = evaluate(&b, Color::Red);
        // The start position should be close to 0 (symmetric).
        assert!(
            score.abs() < 50,
            "start position should be roughly equal, got {score}"
        );
    }

    #[test]
    fn chariot_advantage_is_positive() {
        // Red has an extra chariot.
        let mut b = Board::start_position();
        // Remove Black's a-file chariot.
        let sq = Square::new(0, 9).unwrap();
        b.set_piece(sq, None);
        let score = evaluate(&b, Color::Red);
        assert!(
            score > 800,
            "extra chariot should give Red a large advantage, got {score}"
        );
    }

    #[test]
    fn mobility_bonus_matters() {
        // A chariot in the center should have higher mobility than one in a corner.
        let mut b = Board::empty();
        b.set_piece(
            Square::new(0, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(8, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        // Red chariot in center.
        b.set_piece(
            Square::new(4, 4).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        let center_score = evaluate(&b, Color::Red);

        // Move chariot to corner.
        b.set_piece(Square::new(4, 4).unwrap(), None);
        b.set_piece(
            Square::new(0, 4).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Chariot)),
        );
        let corner_score = evaluate(&b, Color::Red);

        assert!(
            center_score > corner_score,
            "center chariot should score higher: center={center_score} vs corner={corner_score}"
        );
    }

    #[test]
    fn king_safety_rewards_defenders() {
        let mut b = Board::empty();
        b.set_piece(
            Square::new(4, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::King)),
        );
        b.set_piece(
            Square::new(4, 9).unwrap(),
            Some(Piece::new(Color::Black, PieceKind::King)),
        );
        let bare_score = king_safety(&b, Color::Red);

        // Add two advisors.
        b.set_piece(
            Square::new(3, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Advisor)),
        );
        b.set_piece(
            Square::new(5, 0).unwrap(),
            Some(Piece::new(Color::Red, PieceKind::Advisor)),
        );
        let guarded_score = king_safety(&b, Color::Red);

        assert!(
            guarded_score > bare_score,
            "advisors should improve king safety: guarded={guarded_score} vs bare={bare_score}"
        );
    }
}
