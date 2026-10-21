//! loko's `Random` service: a source of random number generators, passed to code instead of drawn
//! from a global, so tests are repeatable and the same code runs natively and in WebAssembly.
//!
//! [`Random::rng`] hands out a generator of the caller's own, a [`rand_core::Rng`] drawn from with
//! `&mut` and no locks, and usable with anything in the `rand` ecosystem. Each implementation chooses
//! the generator:
//!
//! - [`SeededRandom`]: [`SeededRng`]s forked from one seeded sequence, the same on every run; for
//!   tests, not for secrets.
//! - `OsRandom` (feature `getrandom`): `OsRng`, the operating system's generator, on native targets
//!   and WASI. A [`CryptoRng`].
//! - `JsRandom` (feature `js`): `JsRng`, `crypto.getRandomValues`, in browsers, Workers, and Node.
//!   A [`CryptoRng`].
//! - A `Mutex` around any seedable `rand_core` generator: generators forked from it.
//!
//! Code that makes secrets asks for a cryptographic generator: `random: &impl Random<Rng: CryptoRng>`.
//!
//! ```
//! use loko_random::{Random, Rng, SeededRandom};
//!
//! let (a, b) = (SeededRandom::new(7), SeededRandom::new(7));
//! let (mut x, mut y) = (a.rng(), b.rng());
//! assert_eq!(x.next_u64(), y.next_u64(), "the same seed gives the same numbers");
//! ```

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
mod js;
mod rand;
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod system;
mod testing;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use js::{JsRandom, JsRng};
pub use rand_core::{self, CryptoRng, Rng, SeedableRng};
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
pub use system::{OsRandom, OsRng};
pub use testing::{SeededRandom, SeededRng};

/// A source of random number generators.
pub trait Random {
    /// The generator [`rng`](Random::rng) hands out.
    type Rng: rand_core::Rng;

    /// A generator of the caller's own: draw from it with `&mut` as much as needed, with no locks.
    /// Whether it is fit for secrets depends on the implementation: ask for `Random<Rng: CryptoRng>`.
    fn rng(&self) -> Self::Rng;
}

/// A type that holds a [`Random`], and so is one: `#[loko::bundle]` implements this for a field,
/// and `#[loko::wraps]` for a wrapper that changes some methods. It's what `#[loko::service]`
/// generates, written out so this crate needs no macros.
pub trait HasRandom {
    /// The service held.
    type Random: Random;

    /// The [`Random`] this holds.
    fn random(&self) -> &Self::Random;

    /// Forwards [`Random::rng`] to the held service; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_rng(&self) -> <Self::Random as Random>::Rng {
        self.random().rng()
    }
}

impl<H: HasRandom> Random for H {
    type Rng = <<H as HasRandom>::Random as Random>::Rng;

    #[inline]
    fn rng(&self) -> Self::Rng {
        self.wrapped_rng()
    }
}
