# loko-random

[loko](https://github.com/polythings/loko)'s `Random` service: a source of random number generators, passed to code
instead of drawn from a global, so tests are repeatable and the same code runs natively, under WASI, and in browsers.

- `Random::rng()` hands out a generator of the caller's own: a `rand_core::Rng`, drawn from with `&mut` and no locks,
  and usable with anything in the `rand` ecosystem.
- `SeededRandom`: `SeededRng`s (SplitMix64) forked from one seeded sequence, the same on every run. For tests; not for
  secrets. Clones share the sequence.
- `OsRandom` (feature `getrandom`): `OsRng`, the operating system's generator (native and WASI). A `CryptoRng`.
- `JsRandom` (feature `js`, `wasm32-unknown-unknown`): `JsRng`, `crypto.getRandomValues`. A `CryptoRng`.
- A `Mutex` around any seedable `rand_core` generator is a `Random` whose generators are forked from it.

```rust
use loko_random::{CryptoRng, Random, Rng, SeededRandom};

let (a, b) = (SeededRandom::new(7), SeededRandom::new(7));
assert_eq!(a.rng().next_u64(), b.rng().next_u64());

/// Secrets ask for a cryptographic generator, so a SeededRandom can't be passed here by mistake.
fn token(random: &impl Random<Rng: CryptoRng>) -> [u8; 32] {
    let mut bytes = [0; 32];
    random.rng().fill_bytes(&mut bytes);
    bytes
}
```

Apache License 2.0.
