//! A small seeded random number generator (SplitMix64), so soups, breeding and random threads
//! are reproducible (SPEC ENG-6: the app seeds from the clock, tests seed explicitly).

use std::time::{SystemTime, UNIX_EPOCH};

/// SplitMix64: tiny, fast, and statistically good enough for soups and mutations. Not for
/// cryptography.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator with a fixed seed: the same seed always gives the same sequence.
    pub const fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// A generator seeded from the system clock (what the app uses).
    pub fn from_clock() -> Rng {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Rng::new(nanos)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform float in `[0, 1)` (like `Math.random()`).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// A uniform integer in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next_u64() as u128 * n as u128) >> 64) as usize
    }

    /// `true` with probability `p` (`p ≤ 0` never, `p ≥ 1` always, NaN never).
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// A fresh seed for another generator (e.g. a new random Loom thread).
    pub fn seed(&mut self) -> u64 {
        self.next_u64()
    }
}

impl Default for Rng {
    /// Seed 0.
    fn default() -> Self {
        Rng::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
            let f = a.next_f64();
            assert!((0.0..1.0).contains(&f));
            assert!(a.below(7) < 7);
            b.next_f64();
            b.below(7);
        }
        assert_eq!(Rng::new(1).below(0), 0);
        let mut r = Rng::new(3);
        let heads = (0..10_000).filter(|_| r.chance(0.5)).count();
        assert!((4_700..5_300).contains(&heads), "{heads}");
        assert!(!r.chance(f64::NAN));
    }
}
