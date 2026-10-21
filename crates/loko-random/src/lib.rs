//! loko's `Random` resource: random numbers, passed to code instead of drawn from a global source, so
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

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
mod js;
#[cfg(feature = "rand")]
mod rand;
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod system;
mod testing;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use js::JsRandom;
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
pub use system::OsRandom;
pub use testing::SeededRandom;

/// Random numbers.
#[loko_macros::resource]
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
