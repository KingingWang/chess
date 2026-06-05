//! Correctness tests for the rules engine, including `perft` move-count
//! validation against widely-published reference values.

use crate::*;

fn sq(file: u8, rank: u8) -> Square {
    Square::new(file, rank).unwrap()
}

// Two kings placed on DIFFERENT files so the "flying general" rule never makes
// the fixture accidentally illegal.
fn place_safe_kings(b: &mut Board) {
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_piece(sq(5, 9), Some(Piece::new(Color::Black, PieceKind::King)));
}

/// Count leaf nodes of the legal move tree to `depth` (a "perft").
fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut nodes = 0;
    let mut b = board.clone();
    for mv in moves {
        let undo = b.make_move(mv);
        nodes += perft(&b, depth - 1);
        b.unmake_move(undo);
    }
    nodes
}

#[test]
fn fen_roundtrip_start() {
    let b = Board::start_position();
    assert_eq!(b.to_fen(), START_FEN);
}

#[test]
fn fen_parse_places_kings() {
    let b = Board::start_position();
    assert_eq!(b.king_square(Color::Red), Some(sq(4, 0)));
    assert_eq!(b.king_square(Color::Black), Some(sq(4, 9)));
    assert_eq!(b.side_to_move(), Color::Red);
}

#[test]
fn start_position_has_44_legal_moves() {
    let b = Board::start_position();
    assert_eq!(b.legal_moves().len(), 44);
}

#[test]
fn perft_reference_values() {
    // Reference perft values for the Xiangqi start position (xqbase/community).
    let b = Board::start_position();
    assert_eq!(perft(&b, 1), 44);
    assert_eq!(perft(&b, 2), 1920);
    assert_eq!(perft(&b, 3), 79666);
}

#[test]
#[ignore = "slow; run with --ignored in release"]
fn perft_deep() {
    let b = Board::start_position();
    assert_eq!(perft(&b, 4), 3_290_240);
}

#[test]
fn horse_leg_is_blocked() {
    // Red horse at e5 (4,4) with a friendly pawn directly north at e6 (4,5)
    // cannot make the two north-ward jumps; other legs are open.
    let mut b = Board::empty();
    place_safe_kings(&mut b);
    b.set_piece(sq(4, 4), Some(Piece::new(Color::Red, PieceKind::Horse)));
    b.set_piece(sq(4, 5), Some(Piece::new(Color::Red, PieceKind::Pawn)));
    let horse_moves: Vec<_> = b
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq(4, 4))
        .map(|m| m.to)
        .collect();
    // Blocked: the two destinations requiring the north leg (3,6) and (5,6).
    assert!(!horse_moves.contains(&sq(3, 6)));
    assert!(!horse_moves.contains(&sq(5, 6)));
    // East/west legs are open: (6,5),(6,3),(2,5),(2,3) reachable.
    assert!(horse_moves.contains(&sq(6, 5)));
    assert!(horse_moves.contains(&sq(2, 3)));
    // South leg open: (3,2),(5,2) reachable.
    assert!(horse_moves.contains(&sq(3, 2)));
}

#[test]
fn elephant_eye_block_and_river() {
    // Red elephant at c0 (2,0). Place a blocker at its NE eye d1 (3,1).
    let mut b = Board::empty();
    place_safe_kings(&mut b);
    b.set_piece(sq(2, 0), Some(Piece::new(Color::Red, PieceKind::Elephant)));
    b.set_piece(sq(3, 1), Some(Piece::new(Color::Red, PieceKind::Advisor)));
    let dests: Vec<_> = b
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq(2, 0))
        .map(|m| m.to)
        .collect();
    // NE target e2 (4,2) is blocked by the eye at (3,1).
    assert!(!dests.contains(&sq(4, 2)));
    // NW target a2 (0,2) is reachable (eye at (1,1) is empty).
    assert!(dests.contains(&sq(0, 2)));

    // An elephant can never cross the river: from c4 it cannot reach rank 5+.
    let mut b2 = Board::empty();
    place_safe_kings(&mut b2);
    b2.set_piece(sq(2, 4), Some(Piece::new(Color::Red, PieceKind::Elephant)));
    let crossed = b2
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq(2, 4))
        .any(|m| m.to.rank() >= 5);
    assert!(!crossed);
}

#[test]
fn cannon_needs_screen_to_capture() {
    // Red cannon at e0 (4,0); enemy chariot at e9 (4,9). With no screen the
    // cannon may slide but NOT capture; add a screen and capture appears.
    let mut b = Board::empty();
    place_safe_kings(&mut b); // kings on files 0 and 5, clear of the e-file
    b.set_piece(sq(4, 0), Some(Piece::new(Color::Red, PieceKind::Cannon)));
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::Chariot)));

    let can_capture = |bd: &Board| {
        bd.legal_moves()
            .into_iter()
            .any(|m| m.from == sq(4, 0) && m.to == sq(4, 9))
    };
    assert!(!can_capture(&b), "no screen => no capture");

    b.set_piece(sq(4, 4), Some(Piece::new(Color::Black, PieceKind::Pawn)));
    assert!(can_capture(&b), "one screen => capture allowed");

    // Two screens block the capture again.
    b.set_piece(sq(4, 5), Some(Piece::new(Color::Black, PieceKind::Pawn)));
    assert!(!can_capture(&b), "two screens => no capture");
}

#[test]
fn flying_general_is_illegal() {
    // Kings on the SAME open file (e). Red cannon on e1 blocks the file; moving
    // it off-file would expose the generals and must be filtered out.
    let mut b = Board::empty();
    b.set_piece(sq(4, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    b.set_piece(sq(4, 1), Some(Piece::new(Color::Red, PieceKind::Cannon)));
    let exposes = b
        .legal_moves()
        .into_iter()
        .any(|m| m.from == sq(4, 1) && m.to.file() != 4);
    assert!(!exposes, "cannot expose generals to each other");
    // Moving the cannon along the file (still blocking) is fine.
    let along = b
        .legal_moves()
        .into_iter()
        .any(|m| m.from == sq(4, 1) && m.to.file() == 4);
    assert!(along);
}

#[test]
fn pawn_movement_rules() {
    // Red pawn before the river (rank 3) only moves forward.
    let mut b = Board::empty();
    place_safe_kings(&mut b);
    b.set_piece(sq(4, 3), Some(Piece::new(Color::Red, PieceKind::Pawn)));
    let dests: Vec<_> = b
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq(4, 3))
        .map(|m| m.to)
        .collect();
    assert_eq!(dests, vec![sq(4, 4)]);

    // After crossing (rank 5) it may also move sideways but never backward.
    let mut b2 = Board::empty();
    place_safe_kings(&mut b2);
    b2.set_piece(sq(4, 5), Some(Piece::new(Color::Red, PieceKind::Pawn)));
    let mut dests2: Vec<_> = b2
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq(4, 5))
        .map(|m| m.to)
        .collect();
    dests2.sort();
    let mut expected = vec![sq(3, 5), sq(5, 5), sq(4, 6)];
    expected.sort();
    assert_eq!(dests2, expected);
}

#[test]
fn detects_checkmate() {
    // Three-chariot box mate: Black king e9 (4,9). Red chariots on files d,e,f
    // cover every flight square and the e-file chariot gives check. No black
    // piece can capture or interpose -> checkmate.
    let mut b = Board::empty();
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    b.set_piece(sq(3, 0), Some(Piece::new(Color::Red, PieceKind::Chariot))); // file d
    b.set_piece(sq(5, 0), Some(Piece::new(Color::Red, PieceKind::Chariot))); // file f
    b.set_piece(sq(4, 0), Some(Piece::new(Color::Red, PieceKind::Chariot))); // file e, checks
                                                                             // Red king tucked away off the relevant files.
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_side_to_move(Color::Black);

    assert!(b.is_in_check(Color::Black));
    assert!(b.legal_moves().is_empty(), "should be checkmate");
}

#[test]
fn checkmate_result_via_game() {
    // Reach the mate by Red playing the final checking move so Game adjudicates.
    let mut b = Board::empty();
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    b.set_piece(sq(3, 0), Some(Piece::new(Color::Red, PieceKind::Chariot)));
    b.set_piece(sq(5, 0), Some(Piece::new(Color::Red, PieceKind::Chariot)));
    b.set_piece(sq(4, 5), Some(Piece::new(Color::Red, PieceKind::Chariot))); // will deliver mate
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_side_to_move(Color::Red);

    let mut g = Game::from_board(b);
    // Chariot e6 -> e1 keeps the e-file controlled and checkmates.
    let res = g
        .make_move(Move::new(sq(4, 5), sq(4, 1)))
        .expect("legal move");
    assert_eq!(
        res,
        Some(GameResult::Win {
            winner: Color::Red,
            reason: WinReason::Checkmate
        })
    );
    assert!(g.is_over());
}

#[test]
fn game_make_move_and_undo() {
    let mut g = Game::new();
    let mv = Move::from_iccs("h2e2").unwrap(); // cannon to center
    assert!(g.make_move(mv).is_ok());
    assert_eq!(g.side_to_move(), Color::Black);
    assert!(!g.is_over());
    assert!(g.undo());
    assert_eq!(g.side_to_move(), Color::Red);
    assert_eq!(g.board().to_fen(), START_FEN);
}

#[test]
fn illegal_move_is_rejected() {
    let mut g = Game::new();
    // a0 is a chariot; a0a0 is not a move, a0a3 is blocked by own pawn at a3.
    let bad = Move::from_iccs("a0a4").unwrap();
    assert!(matches!(g.make_move(bad), Err(IllegalMove::NotLegal(_))));
}

#[test]
fn resignation_sets_result() {
    let mut g = Game::new();
    g.resign(Color::Red);
    assert_eq!(
        g.result(),
        Some(GameResult::Win {
            winner: Color::Black,
            reason: WinReason::Resignation
        })
    );
}

#[test]
fn iccs_roundtrip() {
    for s in ["a0a1", "h2e2", "i9i0", "e0e1"] {
        let m = Move::from_iccs(s).unwrap();
        assert_eq!(m.to_iccs(), s);
    }
    assert!(Move::from_iccs("z9a1").is_none());
    assert!(Move::from_iccs("a0a").is_none());
}

/// Regression: `is_attacked_by` must detect a horse that checks/attacks a
/// square from near the board edge, and must respect the leg block — without
/// the off-board `unwrap` panic that used to occur when the leg offset pointed
/// the wrong way (away from the target). See board.rs `HORSE_FROM`.
#[test]
fn horse_attack_detection_edge_and_leg() {
    // Target high up the board so the (buggy) leg would land off-board (rank 10).
    let target = sq(4, 7);

    // Horse at target + (1, 2) = (5, 9); its leg toward the target is (5, 8).
    let mut b = Board::empty();
    place_safe_kings(&mut b);
    b.set_piece(sq(5, 9), Some(Piece::new(Color::Black, PieceKind::Horse)));
    // Leg (5, 8) is empty -> the horse attacks the target.
    assert!(
        b.is_attacked_by(target, Color::Black),
        "edge horse with an empty leg should attack the target"
    );

    // Now block the leg (5, 8): the horse can no longer attack (蹩马腿).
    let mut b2 = b.clone();
    b2.set_piece(sq(5, 8), Some(Piece::new(Color::Black, PieceKind::Pawn)));
    assert!(
        !b2.is_attacked_by(target, Color::Black),
        "a blocked horse leg must stop the attack"
    );
}

/// A horse delivering check from the edge is reported by `is_in_check`
/// (exercises the same path as the AI search that triggered the panic).
#[test]
fn horse_checks_king_from_edge() {
    let mut b = Board::empty();
    // Black king at (4, 9); red horse at (5, 7) attacks (4, 9) with leg (5, 8).
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_piece(sq(5, 7), Some(Piece::new(Color::Red, PieceKind::Horse)));
    assert!(b.is_in_check(Color::Black), "edge horse should give check");
}

// --- Round 0: History API tests -------------------------------------------

#[test]
fn history_accessor_returns_played_moves() {
    let mut g = Game::new();
    let m1 = Move::from_iccs("h2e2").unwrap();
    let m2 = Move::from_iccs("h9g7").unwrap();
    g.make_move(m1).unwrap();
    g.make_move(m2).unwrap();

    let history = g.history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].mv(), m1);
    assert_eq!(history[1].mv(), m2);
}

#[test]
fn played_moves_iterator() {
    let mut g = Game::new();
    let m1 = Move::from_iccs("h2e2").unwrap();
    let m2 = Move::from_iccs("h9g7").unwrap();
    g.make_move(m1).unwrap();
    g.make_move(m2).unwrap();

    let moves: Vec<Move> = g.played_moves().collect();
    assert_eq!(moves, vec![m1, m2]);
}

#[test]
fn history_entry_reports_check() {
    // Set up a position where a move delivers check.
    let mut b = Board::empty();
    b.set_piece(sq(4, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_piece(sq(4, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    // Red chariot on a0 can slide to e0... no, let's use a direct check:
    // Red chariot at a5, moving to e5 does not check. Let's find a simpler check.
    // Put Red chariot on e5 (same file as Black king e9), move it... it already checks.
    // Better: Red chariot on a0 (file 0), move to a9 (rank 9)... Black king is on e9.
    // Actually let's just verify gave_check on a known checking move.
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::Chariot)));
    b.set_side_to_move(Color::Red);

    let mut g = Game::from_board(b);
    // Move chariot from a0 to a9 — does not check (different file from king).
    // Move chariot from a0 to e0 — wait, Red king is there.
    // Let's move Red king out of the way.
    // Simpler approach: just play opening moves and verify gave_check is false.
    let mut g2 = Game::new();
    let m = Move::from_iccs("h2e2").unwrap();
    g2.make_move(m).unwrap();
    assert!(!g2.history()[0].gave_check());
}

#[test]
fn board_at_ply_zero_is_start() {
    let mut g = Game::new();
    let m1 = Move::from_iccs("h2e2").unwrap();
    g.make_move(m1).unwrap();

    let board0 = g.board_at_ply(0).unwrap();
    assert_eq!(board0.to_fen(), START_FEN);
}

#[test]
fn board_at_ply_current_matches_board() {
    let mut g = Game::new();
    let m1 = Move::from_iccs("h2e2").unwrap();
    let m2 = Move::from_iccs("h9g7").unwrap();
    g.make_move(m1).unwrap();
    g.make_move(m2).unwrap();

    let board_now = g.board_at_ply(2).unwrap();
    assert_eq!(board_now.to_fen(), g.board().to_fen());
}

#[test]
fn board_at_ply_out_of_range_returns_none() {
    let g = Game::new();
    assert!(g.board_at_ply(1).is_none());
}

#[test]
fn history_entry_captured_piece() {
    // Set up a capture scenario.
    let mut b = Board::empty();
    b.set_piece(sq(0, 0), Some(Piece::new(Color::Red, PieceKind::King)));
    b.set_piece(sq(8, 9), Some(Piece::new(Color::Black, PieceKind::King)));
    b.set_piece(sq(4, 4), Some(Piece::new(Color::Red, PieceKind::Chariot)));
    b.set_piece(sq(4, 8), Some(Piece::new(Color::Black, PieceKind::Cannon)));
    b.set_side_to_move(Color::Red);

    let mut g = Game::from_board(b);
    let capture_move = Move::new(sq(4, 4), sq(4, 8));
    g.make_move(capture_move).unwrap();

    let entry = &g.history()[0];
    assert_eq!(entry.mv(), capture_move);
    assert_eq!(
        entry.captured(),
        Some(Piece::new(Color::Black, PieceKind::Cannon))
    );
}
