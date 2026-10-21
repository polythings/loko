use std::convert::Infallible;
use std::sync::{Arc, Mutex, PoisonError};

use rand_core::{SeedableRng, TryRng};

use crate::Random;

/// One step of `SplitMix64` (Steele, Lea, and Flood, 2014): a Weyl sequence with a mixing function.
fn split_mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A repeatable source for tests: every [`rng`](Random::rng) is a [`SeededRng`] seeded from one
/// sequence that starts at the seed, so the same seed and the same calls give the same numbers on
/// every run. Not for secrets. Clones share the sequence.
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
    type Rng = SeededRng;

    fn rng(&self) -> SeededRng {
        SeededRng::new(split_mix(&mut self.state.lock().unwrap_or_else(PoisonError::into_inner)))
    }
}

/// A `SplitMix64` generator: small, fast, and the same on every platform. Not for secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    /// The generator whose state starts at `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl TryRng for SeededRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok((split_mix(&mut self.state) >> 32) as u32)
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(split_mix(&mut self.state))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        for chunk in dst.chunks_mut(8) {
            let bytes = split_mix(&mut self.state).to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Ok(())
    }
}

impl SeedableRng for SeededRng {
    type Seed = [u8; 8];

    fn from_seed(seed: [u8; 8]) -> Self {
        Self::new(u64::from_le_bytes(seed))
    }
}

#[cfg(test)]
mod tests {
    use rand_core::Rng;

    use super::*;

    #[test]
    fn split_mix_matches_the_published_sequence() {
        // SplitMix64's first output from state 0, as published.
        assert_eq!(SeededRng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn seeded_sources_repeat() {
        let draw = |random: &SeededRandom| -> Vec<u64> {
            let mut rng = random.rng();
            (0..5).map(|_| rng.next_u64()).collect()
        };
        let (a, b) = (SeededRandom::new(42), SeededRandom::new(42));
        assert_eq!(draw(&a), draw(&b));
        assert_ne!(draw(&a), draw(&a), "each rng() is a new generator");
        assert_ne!(SeededRandom::new(1).rng().next_u64(), SeededRandom::new(2).rng().next_u64());
        assert_eq!(SeededRandom::new(0).rng().next_u64(), 0xA706_DD2F_4D19_7E6F);
    }

    #[test]
    fn clones_share_the_sequence_and_bytes_cover_odd_lengths() {
        let random = SeededRandom::new(3);
        let clone = random.clone();
        assert_ne!(random.rng(), clone.rng(), "clones share one sequence");
        let mut bytes = [0u8; 13];
        random.rng().fill_bytes(&mut bytes);
        assert!(bytes.iter().any(|&b| b != 0));
    }
}
