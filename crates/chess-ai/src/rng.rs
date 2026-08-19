//! Tiny non-cryptographic PRNG used to give the AI move variety.
//!
//! Deterministic engines replay the same reply to the same position, which
//! lets players memorize and repeat a single winning line. Seeding from the
//! system clock (plus PID) and sampling among near-equal moves keeps games
//! varied without measurably weakening play. This is *not* suitable for
//! cryptography — it only picks chess moves.

/// xorshift64* — 15 lines of stateless-looking PRNG with a 2^64 period.
#[derive(Debug, Clone)]
pub struct SmallRng(u64);

impl SmallRng {
    /// Seed from wall-clock entropy (system time nanos ^ pid ^ heap address).
    pub fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let addr = &nanos as *const u64 as u64;
        Self::from_seed(nanos ^ (std::process::id() as u64) << 32 ^ addr)
    }

    /// Seed deterministically (tests).
    pub fn from_seed(seed: u64) -> Self {
        // Addition is a bijection (every seed stays distinct) and the offset
        // keeps the state away from the all-zero xorshift fixed point.
        SmallRng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform value in `0..n` (modulo bias is irrelevant for move picking).
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.next_u64() % n
    }
}

/// Among root moves whose score is within `window` centipawns of the best,
/// pick one uniformly at random. With `window == 0` this degenerates to a
/// deterministic best-move pick. `scores` pairs are `(move, score)`.
pub fn pick_within_window(
    scores: &[(chess_core::Move, i32)],
    window: i32,
    rng: &mut SmallRng,
) -> Option<chess_core::Move> {
    let best = scores.iter().map(|(_, s)| *s).max()?;
    let candidates: Vec<chess_core::Move> = scores
        .iter()
        .filter(|(_, s)| best - s <= window)
        .map(|(m, _)| *m)
        .collect();
    debug_assert!(!candidates.is_empty());
    let idx = rng.below(candidates.len() as u64) as usize;
    candidates.into_iter().nth(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::{Move, Square};

    #[test]
    fn rng_is_deterministic_per_seed() {
        let mut a = SmallRng::from_seed(42);
        let mut b = SmallRng::from_seed(42);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = SmallRng::from_seed(7);
        for _ in 0..1000 {
            assert!(rng.below(13) < 13);
        }
    }

    #[test]
    fn window_zero_picks_best() {
        let mv = |f1: u8, r1: u8, f2: u8, r2: u8| {
            Move::new(Square::new(f1, r1).unwrap(), Square::new(f2, r2).unwrap())
        };
        let scores = vec![
            (mv(0, 0, 0, 1), 50),
            (mv(1, 0, 1, 1), 100),
            (mv(2, 0, 2, 1), 80),
        ];
        let mut rng = SmallRng::from_seed(1);
        for _ in 0..32 {
            assert_eq!(
                pick_within_window(&scores, 0, &mut rng),
                Some(mv(1, 0, 1, 1))
            );
        }
    }

    #[test]
    fn wide_window_eventually_picks_every_candidate() {
        let mv = |f1: u8| Move::new(Square::new(f1, 0).unwrap(), Square::new(f1, 1).unwrap());
        // Three moves within 20cp of the best, one far behind.
        let scores = vec![(mv(0), 100), (mv(1), 95), (mv(2), 5), (mv(3), 90)];
        let mut seen = std::collections::HashSet::new();
        for seed in 0..200 {
            let mut rng = SmallRng::from_seed(seed);
            seen.insert(pick_within_window(&scores, 20, &mut rng).unwrap());
        }
        assert!(seen.contains(&mv(0)) && seen.contains(&mv(1)) && seen.contains(&mv(3)));
        assert!(
            !seen.contains(&mv(2)),
            "far-behind move must never be picked"
        );
    }
}
