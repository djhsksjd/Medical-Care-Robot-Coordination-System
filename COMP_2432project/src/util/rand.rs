//! Randomness helpers.
//!
//! This project avoids heavy third-party dependencies for core OS concepts.
//! We provide a tiny deterministic PRNG for demos/tests and an opt-in seed
//! derived from time for convenience.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SEED_SALT: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    /// Create a deterministic RNG from an explicit seed.
    pub fn from_seed(seed: u64) -> Self {
        let seed = if seed == 0 {
            0xD1B5_4A32_D192_ED03
        } else {
            seed
        };
        Self { state: seed }
    }

    /// Create an RNG seeded from current time plus a process-local salt.
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let salt = SEED_SALT.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        Self::from_seed(nanos ^ salt.rotate_left(17))
    }

    /// Xorshift64* (fast, not cryptographically secure).
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Generate a number in `[low, high)`. Returns `low` if the range is empty.
    pub fn gen_range_u64(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            return low;
        }
        let span = high - low;
        low + (self.next_u64() % span)
    }

    /// Returns true with probability `numerator/denominator`.
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        if denominator == 0 {
            return false;
        }
        (self.next_u32() % denominator) < numerator
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn deterministic_seed_is_repeatable() {
        let mut a = Random::from_seed(123);
        let mut b = Random::from_seed(123);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_stays_within_bounds() {
        let mut rng = Random::from_seed(7);
        for _ in 0..1000 {
            let v = rng.gen_range_u64(10, 20);
            assert!(v >= 10 && v < 20);
        }
    }
}
