//! loko's `Random` service: random numbers, passed to code instead of drawn from a global source, so
//! tests are repeatable and the same code runs natively and in WebAssembly.
//!
//! - [`Random`]: `next_u64` and `fill`.
//! - [`SeededRandom`]: a repeatable sequence from a seed, for tests.
//! - `OsRandom` (feature `getrandom`): the operating system's generator, on native targets and WASI.
//! - `JsRandom` (feature `js`): `crypto.getRandomValues`, in browsers, Workers, and Node.
//! - Feature `rand`: any `rand_core` generator in a `Mutex` is a `Random`.
//!
//! ```
//! use loko_random::{Random, SeededRandom};
//!
//! let (a, b) = (SeededRandom::new(7), SeededRandom::new(7));
//! assert_eq!(a.next_u64(), b.next_u64());
//! ```

use std::sync::{Arc, Mutex, PoisonError};

/// Random numbers.
#[loko_macros::service]
pub trait Random {
    /// A random number.
    fn next_u64(&self) -> u64;

    /// Fills `dest` with random bytes.
    fn fill(&self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }
}

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

/// The operating system's random number generator (feature `getrandom`, native targets and WASI).
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRandom;

#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
impl Random for OsRandom {
    fn next_u64(&self) -> u64 {
        // The operating system's generator only fails when it isn't available at all.
        getrandom::u64().expect("the operating system's random number generator")
    }

    fn fill(&self, dest: &mut [u8]) {
        getrandom::fill(dest).expect("the operating system's random number generator");
    }
}

/// `crypto.getRandomValues` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`): browsers, Workers, and
/// Node. Hosts without it fall back to `Math.random`.
#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct JsRandom;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
impl Random for JsRandom {
    fn next_u64(&self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn fill(&self, dest: &mut [u8]) {
        use js_sys::{Function, Reflect, Uint8Array};
        use wasm_bindgen::JsCast;

        let crypto = Reflect::get(&js_sys::global(), &"crypto".into()).ok().filter(|c| !c.is_undefined());
        let get_random_values = crypto
            .as_ref()
            .and_then(|c| Reflect::get(c, &"getRandomValues".into()).ok().and_then(|f| f.dyn_into::<Function>().ok()));
        // getRandomValues fills at most 65,536 bytes per call.
        for chunk in dest.chunks_mut(65_536) {
            let filled = match (&crypto, &get_random_values) {
                (Some(crypto), Some(f)) => {
                    let array = Uint8Array::new_with_length(u32::try_from(chunk.len()).unwrap_or(u32::MAX));
                    f.call1(crypto, &array).is_ok() && {
                        array.copy_to(chunk);
                        true
                    }
                }
                _ => false,
            };
            if !filled {
                for b in chunk.iter_mut() {
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a byte")]
                    let byte = (js_sys::Math::random() * 256.0) as u8;
                    *b = byte;
                }
            }
        }
    }
}

/// Any `rand_core` generator, behind a mutex so it can be shared (feature `rand`).
#[cfg(feature = "rand")]
impl<R: rand_core::Rng> Random for Mutex<R> {
    fn next_u64(&self) -> u64 {
        self.lock().unwrap_or_else(PoisonError::into_inner).next_u64()
    }

    fn fill(&self, dest: &mut [u8]) {
        self.lock().unwrap_or_else(PoisonError::into_inner).fill_bytes(dest);
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

    #[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
    #[test]
    fn os_random_numbers_differ() {
        assert_ne!(OsRandom.next_u64(), OsRandom.next_u64());
        let mut bytes = [0u8; 32];
        OsRandom.fill(&mut bytes);
        assert!(bytes.iter().any(|&b| b != 0));
    }
}
