//! Transposition table for caching search results.
//!
//! A fixed-size hash table that stores the best move, score, and search depth
//! for positions already visited. This avoids redundant work when the same
//! position is reached by different move orders (transpositions). The table
//! uses a lockless "always-replace" scheme sized to a power of two for fast
//! index masking.

use chess_core::Move;

/// The type of bound stored in a TT entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Exact minimax value (PV node).
    Exact,
    /// Lower bound (the search found a beta cutoff, so the true value is ≥ score).
    Lower,
    /// Upper bound (all moves searched, none raised alpha; true value is ≤ score).
    Upper,
}

/// A single transposition table entry.
#[derive(Debug, Clone, Copy)]
pub struct TtEntry {
    /// Verification key: upper bits of the Zobrist hash (to detect collisions).
    pub key: u32,
    /// Best (or refutation) move found in this position.
    pub best_move: Option<Move>,
    /// Evaluation score.
    pub score: i32,
    /// Search depth at which this score was computed.
    pub depth: u16,
    /// Bound type (exact, lower, upper).
    pub bound: Bound,
}

/// A fixed-size transposition table.
pub struct TranspositionTable {
    /// The table entries. Size is always a power of two.
    entries: Vec<Option<TtEntry>>,
    /// Bitmask for fast index computation: `hash & mask`.
    mask: u64,
}

impl TranspositionTable {
    /// Create a new TT with approximately `size_mb` megabytes of capacity.
    /// Actual size is rounded down to the nearest power of two in entry count.
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TtEntry>>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        // Round down to power of two.
        let capacity = if num_entries == 0 {
            1
        } else {
            1 << (63 - (num_entries as u64).leading_zeros())
        };
        TranspositionTable {
            entries: vec![None; capacity as usize],
            mask: capacity - 1,
        }
    }

    /// Look up a position by its Zobrist hash.
    #[inline]
    pub fn probe(&self, hash: u64) -> Option<&TtEntry> {
        let idx = (hash & self.mask) as usize;
        let entry = self.entries[idx].as_ref()?;
        // Verify the key matches (collision detection).
        if entry.key == Self::verification_key(hash) {
            Some(entry)
        } else {
            None
        }
    }

    /// Store a search result. Uses an always-replace policy (simple and
    /// effective for iterative deepening where deeper results naturally
    /// overwrite shallower ones).
    #[inline]
    pub fn store(
        &mut self,
        hash: u64,
        best_move: Option<Move>,
        score: i32,
        depth: u16,
        bound: Bound,
    ) {
        let idx = (hash & self.mask) as usize;
        // Only replace if the new entry is at least as deep as the existing one
        // OR the existing entry has a different position (collision).
        let key = Self::verification_key(hash);
        let should_replace = match &self.entries[idx] {
            None => true,
            Some(existing) => existing.key != key || depth >= existing.depth,
        };
        if should_replace {
            self.entries[idx] = Some(TtEntry {
                key,
                best_move,
                score,
                depth,
                bound,
            });
        }
    }

    /// Clear all entries (used when starting a new game).
    pub fn clear(&mut self) {
        self.entries.iter_mut().for_each(|e| *e = None);
    }

    /// Upper 32 bits of the hash, used as a collision-detection key.
    #[inline]
    fn verification_key(hash: u64) -> u32 {
        (hash >> 32) as u32
    }

    /// Number of entries in the table (for diagnostics).
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::{hash_board, Board, Square};

    #[test]
    fn store_and_probe() {
        let mut tt = TranspositionTable::new(1); // 1 MB
        let b = Board::start_position();
        let hash = hash_board(&b);
        let mv = chess_core::Move::from_iccs("h2e2").unwrap();

        tt.store(hash, Some(mv), 42, 5, Bound::Exact);
        let entry = tt.probe(hash).expect("should find stored entry");
        assert_eq!(entry.best_move, Some(mv));
        assert_eq!(entry.score, 42);
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.bound, Bound::Exact);
    }

    #[test]
    fn probe_returns_none_for_unknown() {
        let tt = TranspositionTable::new(1);
        assert!(tt.probe(0x123456789ABCDEF0).is_none());
    }

    #[test]
    fn deeper_entry_replaces_shallower() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0xDEADBEEF_CAFEBABE;

        tt.store(hash, None, 10, 3, Bound::Lower);
        tt.store(hash, None, 20, 5, Bound::Exact);

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.score, 20);
        assert_eq!(entry.depth, 5);
    }

    #[test]
    fn shallower_entry_does_not_replace_deeper() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0xDEADBEEF_CAFEBABE;

        tt.store(hash, None, 20, 8, Bound::Exact);
        tt.store(hash, None, 10, 3, Bound::Lower);

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.score, 20, "deeper entry should be preserved");
        assert_eq!(entry.depth, 8);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0xFEEDFACE_12345678;
        tt.store(hash, None, 100, 10, Bound::Exact);
        assert!(tt.probe(hash).is_some());
        tt.clear();
        assert!(tt.probe(hash).is_none());
    }
}
