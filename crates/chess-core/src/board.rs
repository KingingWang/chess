//! Board state, move application, move generation, and attack detection.

use crate::moves::{Move, UndoState};
use crate::piece::{Color, Piece, PieceKind};
use crate::square::{self, Square, FILES, NUM_SQUARES, RANKS};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Full board position: piece placement plus side to move.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Board {
    #[cfg_attr(feature = "serde", serde(with = "serde_squares"))]
    squares: [Option<Piece>; NUM_SQUARES],
    side_to_move: Color,
}

#[cfg(feature = "serde")]
mod serde_squares {
    //! Serde doesn't provide a default impl for arrays larger than 32; the
    //! 90-square board is serialized as a length-prefixed sequence instead,
    //! keeping the wire format compact and free of extra dependencies.
    use super::*;
    use serde::de::Error as _;

    pub fn serialize<S: serde::Serializer>(
        arr: &[Option<Piece>; NUM_SQUARES],
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        ser.collect_seq(arr.iter())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        de: D,
    ) -> Result<[Option<Piece>; NUM_SQUARES], D::Error> {
        let v: Vec<Option<Piece>> = Vec::deserialize(de)?;
        if v.len() != NUM_SQUARES {
            return Err(D::Error::invalid_length(v.len(), &"90 squares"));
        }
        let mut out = [None; NUM_SQUARES];
        for (i, p) in v.into_iter().enumerate() {
            out[i] = p;
        }
        Ok(out)
    }
}

impl Board {
    /// An empty board with Red to move.
    pub fn empty() -> Board {
        Board {
            squares: [None; NUM_SQUARES],
            side_to_move: Color::Red,
        }
    }

    /// The standard opening position.
    pub fn start_position() -> Board {
        crate::fen::START_FEN.parse().expect("START_FEN is valid")
    }

    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    #[inline]
    pub fn set_side_to_move(&mut self, color: Color) {
        self.side_to_move = color;
    }

    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq.index()]
    }

    #[inline]
    pub fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.squares[sq.index()] = piece;
    }

    /// Iterate over `(square, piece)` for every occupied square.
    pub fn pieces(&self) -> impl Iterator<Item = (Square, Piece)> + '_ {
        self.squares
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|p| (Square::from_index(i as u8), p)))
    }

    /// Locate a color's king. Returns `None` only on malformed positions.
    pub fn king_square(&self, color: Color) -> Option<Square> {
        self.pieces().find_map(|(sq, p)| {
            if p.color == color && p.kind == PieceKind::King {
                Some(sq)
            } else {
                None
            }
        })
    }

    // --- Move application -------------------------------------------------

    /// Apply a move unconditionally (no legality check), returning undo info.
    pub fn make_move(&mut self, mv: Move) -> UndoState {
        let captured = self.squares[mv.to.index()];
        self.squares[mv.to.index()] = self.squares[mv.from.index()];
        self.squares[mv.from.index()] = None;
        self.side_to_move = self.side_to_move.opponent();
        UndoState { mv, captured }
    }

    /// Reverse a previously applied move.
    pub fn unmake_move(&mut self, undo: UndoState) {
        self.squares[undo.mv.from.index()] = self.squares[undo.mv.to.index()];
        self.squares[undo.mv.to.index()] = undo.captured;
        self.side_to_move = self.side_to_move.opponent();
    }

    // --- Pseudo-legal move generation ------------------------------------

    /// Generate pseudo-legal moves for the side to move (may leave own king in
    /// check or facing the enemy king). Use [`Board::legal_moves`] for filtered
    /// moves.
    pub fn pseudo_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(48);
        let me = self.side_to_move;
        for (sq, piece) in self.pieces() {
            if piece.color != me {
                continue;
            }
            match piece.kind {
                PieceKind::King => self.gen_king(sq, me, &mut moves),
                PieceKind::Advisor => self.gen_advisor(sq, me, &mut moves),
                PieceKind::Elephant => self.gen_elephant(sq, me, &mut moves),
                PieceKind::Horse => self.gen_horse(sq, me, &mut moves),
                PieceKind::Chariot => self.gen_chariot(sq, me, &mut moves),
                PieceKind::Cannon => self.gen_cannon(sq, me, &mut moves),
                PieceKind::Pawn => self.gen_pawn(sq, me, &mut moves),
            }
        }
        moves
    }

    /// Fully legal moves: pseudo-legal moves that do not leave the mover's king
    /// in check and do not expose the generals to each other ("flying general").
    pub fn legal_moves(&self) -> Vec<Move> {
        let me = self.side_to_move;
        let mut board = self.clone();
        self.pseudo_legal_moves()
            .into_iter()
            .filter(|&mv| {
                let undo = board.make_move(mv);
                let ok = !board.is_king_in_danger(me);
                board.unmake_move(undo);
                ok
            })
            .collect()
    }

    /// Is `mv` a fully legal move for the side to move?
    pub fn is_legal(&self, mv: Move) -> bool {
        self.legal_moves().contains(&mv)
    }

    #[inline]
    fn add_if_not_friendly(&self, from: Square, to: Square, me: Color, out: &mut Vec<Move>) {
        match self.squares[to.index()] {
            Some(p) if p.color == me => {}
            _ => out.push(Move::new(from, to)),
        }
    }

    fn gen_king(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        const STEPS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        let (f, r) = (sq.file() as i8, sq.rank() as i8);
        for (df, dr) in STEPS {
            if let Some(to) = Square::try_new(f + df, r + dr) {
                if square::in_palace(to, me) {
                    self.add_if_not_friendly(sq, to, me, out);
                }
            }
        }
    }

    fn gen_advisor(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        const STEPS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        let (f, r) = (sq.file() as i8, sq.rank() as i8);
        for (df, dr) in STEPS {
            if let Some(to) = Square::try_new(f + df, r + dr) {
                if square::in_palace(to, me) {
                    self.add_if_not_friendly(sq, to, me, out);
                }
            }
        }
    }

    fn gen_elephant(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        // Two-step diagonals; blocked by a piece at the midpoint ("塞象眼").
        const STEPS: [(i8, i8); 4] = [(2, 2), (2, -2), (-2, 2), (-2, -2)];
        let (f, r) = (sq.file() as i8, sq.rank() as i8);
        for (df, dr) in STEPS {
            if let Some(to) = Square::try_new(f + df, r + dr) {
                // Elephants may not cross the river.
                if square::crossed_river(to, me) {
                    continue;
                }
                let eye = Square::try_new(f + df / 2, r + dr / 2).unwrap();
                if self.squares[eye.index()].is_some() {
                    continue; // eye blocked
                }
                self.add_if_not_friendly(sq, to, me, out);
            }
        }
    }

    fn gen_horse(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        // (leg dx, leg dy), (dest dx, dest dy): leg square must be empty.
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
        let (f, r) = (sq.file() as i8, sq.rank() as i8);
        for ((lf, lr), (tf, tr)) in MOVES {
            let leg = match Square::try_new(f + lf, r + lr) {
                Some(s) => s,
                None => continue,
            };
            if self.squares[leg.index()].is_some() {
                continue; // "蹩马腿"
            }
            if let Some(to) = Square::try_new(f + tf, r + tr) {
                self.add_if_not_friendly(sq, to, me, out);
            }
        }
    }

    fn gen_chariot(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        const DIRS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        let (f0, r0) = (sq.file() as i8, sq.rank() as i8);
        for (df, dr) in DIRS {
            let (mut f, mut r) = (f0 + df, r0 + dr);
            while let Some(to) = Square::try_new(f, r) {
                match self.squares[to.index()] {
                    None => out.push(Move::new(sq, to)),
                    Some(p) => {
                        if p.color != me {
                            out.push(Move::new(sq, to));
                        }
                        break;
                    }
                }
                f += df;
                r += dr;
            }
        }
    }

    fn gen_cannon(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        const DIRS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        let (f0, r0) = (sq.file() as i8, sq.rank() as i8);
        for (df, dr) in DIRS {
            let (mut f, mut r) = (f0 + df, r0 + dr);
            let mut jumped = false;
            while let Some(to) = Square::try_new(f, r) {
                match self.squares[to.index()] {
                    None => {
                        if !jumped {
                            out.push(Move::new(sq, to)); // quiet slide
                        }
                    }
                    Some(p) => {
                        if !jumped {
                            jumped = true; // this is the screen ("炮架")
                        } else {
                            if p.color != me {
                                out.push(Move::new(sq, to)); // capture over screen
                            }
                            break;
                        }
                    }
                }
                f += df;
                r += dr;
            }
        }
    }

    fn gen_pawn(&self, sq: Square, me: Color, out: &mut Vec<Move>) {
        let fwd = me.forward();
        let (f, r) = (sq.file() as i8, sq.rank() as i8);
        // Forward is always allowed (within board).
        if let Some(to) = Square::try_new(f, r + fwd) {
            self.add_if_not_friendly(sq, to, me, out);
        }
        // Sideways only after crossing the river.
        if square::crossed_river(sq, me) {
            for df in [-1i8, 1] {
                if let Some(to) = Square::try_new(f + df, r) {
                    self.add_if_not_friendly(sq, to, me, out);
                }
            }
        }
    }

    // --- Attack / check detection ----------------------------------------

    /// Would the generals "face" each other along an open file? This is illegal
    /// (the "flying general" / 白脸将 rule).
    pub fn generals_face(&self) -> bool {
        let (rk, bk) = match (self.king_square(Color::Red), self.king_square(Color::Black)) {
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };
        if rk.file() != bk.file() {
            return false;
        }
        let file = rk.file();
        let (lo, hi) = (rk.rank().min(bk.rank()), rk.rank().max(bk.rank()));
        for r in (lo + 1)..hi {
            if self.squares[Square::new(file, r).unwrap().index()].is_some() {
                return false; // something blocks the file
            }
        }
        true
    }

    /// Is the given color's king currently attacked by any enemy piece?
    pub fn is_in_check(&self, color: Color) -> bool {
        let king_sq = match self.king_square(color) {
            Some(s) => s,
            None => return false,
        };
        self.is_attacked_by(king_sq, color.opponent())
    }

    /// Combined danger test: in check OR generals facing. Used to validate the
    /// mover did not leave its own king exposed.
    pub fn is_king_in_danger(&self, color: Color) -> bool {
        self.generals_face() || self.is_in_check(color)
    }

    /// Is `target` attacked by any piece of `by` color?
    pub fn is_attacked_by(&self, target: Square, by: Color) -> bool {
        let (tf, tr) = (target.file() as i8, target.rank() as i8);

        // Chariot & Cannon along ranks/files.
        const DIRS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (df, dr) in DIRS {
            let (mut f, mut r) = (tf + df, tr + dr);
            let mut screen = false;
            while let Some(sq) = Square::try_new(f, r) {
                if let Some(p) = self.squares[sq.index()] {
                    if !screen {
                        if p.color == by && p.kind == PieceKind::Chariot {
                            return true;
                        }
                        // King facing king counts along the file/rank as a
                        // chariot-like attack only via generals_face(); but a
                        // King attacking an adjacent square is handled below.
                        screen = true;
                    } else {
                        if p.color == by && p.kind == PieceKind::Cannon {
                            return true;
                        }
                        break;
                    }
                }
                f += df;
                r += dr;
            }
        }

        // Horse attackers: a horse attacks `target` if it sits a knight-jump
        // away AND its leg (the square adjacent to the horse toward target) is
        // empty.
        // ((horse offset from target), (leg offset from the horse toward the
        // target)). The leg is the orthogonal square the horse steps over along
        // its major (±2) axis; it must be empty for the horse to attack (蹩马腿).
        const HORSE_FROM: [((i8, i8), (i8, i8)); 8] = [
            ((1, 2), (0, -1)),
            ((-1, 2), (0, -1)),
            ((1, -2), (0, 1)),
            ((-1, -2), (0, 1)),
            ((2, 1), (-1, 0)),
            ((2, -1), (-1, 0)),
            ((-2, 1), (1, 0)),
            ((-2, -1), (1, 0)),
        ];
        for ((hf, hr), (lf, lr)) in HORSE_FROM {
            if let Some(from) = Square::try_new(tf + hf, tr + hr) {
                if let Some(p) = self.squares[from.index()] {
                    if p.color == by && p.kind == PieceKind::Horse {
                        // Leg is adjacent to the horse, toward the target; it is
                        // always on-board (it lies between two on-board squares),
                        // but guard defensively rather than unwrap.
                        if let Some(leg) = Square::try_new(tf + hf + lf, tr + hr + lr) {
                            if self.squares[leg.index()].is_none() {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Pawn attackers: an enemy pawn attacks from the square it would step
        // from. A `by`-pawn moves with direction `by.forward()`, so it attacks
        // `target` if it sits one step "behind" (and sideways after river).
        let pfwd = by.forward();
        // Frontal: pawn directly behind target in its forward direction.
        if let Some(from) = Square::try_new(tf, tr - pfwd) {
            if let Some(p) = self.squares[from.index()] {
                if p.color == by && p.kind == PieceKind::Pawn {
                    return true;
                }
            }
        }
        // Sideways: pawn beside target that has crossed the river.
        for df in [-1i8, 1] {
            if let Some(from) = Square::try_new(tf + df, tr) {
                if let Some(p) = self.squares[from.index()] {
                    if p.color == by && p.kind == PieceKind::Pawn && square::crossed_river(from, by)
                    {
                        return true;
                    }
                }
            }
        }

        // King attacker (adjacent within palace — only matters for the rare
        // case where kings are one apart; the flying-general file case is
        // handled separately).
        for (df, dr) in DIRS {
            if let Some(from) = Square::try_new(tf + df, tr + dr) {
                if let Some(p) = self.squares[from.index()] {
                    if p.color == by && p.kind == PieceKind::King {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Convenience: ranks × files debug rendering with FEN-style glyphs.
    pub fn ascii(&self) -> String {
        let mut s = String::new();
        for r in (0..RANKS).rev() {
            for f in 0..FILES {
                let sq = Square::new(f, r).unwrap();
                match self.squares[sq.index()] {
                    Some(p) => s.push(p.to_fen_char()),
                    None => s.push('.'),
                }
            }
            s.push('\n');
        }
        s
    }
}
