use std::sync::{Arc, Mutex, PoisonError};

use crate::Random;

/// A repeatable sequence of numbers from a seed (SplitMix64): the same seed gives the same numbers.
/// Not for secrets. Clones share the sequence.
#[derive(Clone, Debug)]
pub struct SeededRandom {
    state: Arc<Mutex<u64>>,
}

impl SeededRandom {
    /// The sequence of `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: Arc::new(Mutex::new(seed)) }
    }
}

impl Default for SeededRandom {
    /// The sequence of seed 0.
    fn default() -> Self {
        Self::new(0)
    }
}

impl Random for SeededRandom {
    fn next_u64(&self) -> u64 {
        // SplitMix64 (Steele, Lea, and Flood, 2014): a Weyl sequence with a mixing function.
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_sequences_repeat() {
        let a = SeededRandom::new(42);
        let b = SeededRandom::new(42);
        let first: Vec<u64> = (0..5).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..5).map(|_| b.next_u64()).collect();
        assert_eq!(first, second);
        assert_ne!(first[0], first[1]);
        assert_ne!(SeededRandom::new(1).next_u64(), SeededRandom::new(2).next_u64());
        // SplitMix64's first output for seed 0, as published.
        assert_eq!(SeededRandom::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn fill_covers_odd_lengths() {
        let random = SeededRandom::new(3);
        let mut bytes = [0u8; 13];
        random.fill(&mut bytes);
        assert!(bytes.iter().any(|&b| b != 0));
        let clone = random.clone();
        assert_ne!(random.next_u64(), clone.next_u64(), "clones share one sequence");
    }
}
