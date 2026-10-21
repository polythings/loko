# loko-random

[loko](https://github.com/polythings/loko)'s `Random` resource: random numbers, passed to code instead of drawn from a
global source, so tests are repeatable and the same code runs natively, under WASI, and in browsers.

- `Random`: `next_u64` and `fill`.
- `SeededRandom`: a repeatable sequence from a seed (SplitMix64). Not for secrets. Clones share the sequence.
- `OsRandom` (feature `getrandom`): the operating system's generator (native and WASI).
- `JsRandom` (feature `js`, `wasm32-unknown-unknown`): `crypto.getRandomValues`.
- Feature `rand`: any `rand_core` generator in a `Mutex` is a `Random`.

```rust
use loko_random::{Random, SeededRandom};

let (a, b) = (SeededRandom::new(7), SeededRandom::new(7));
assert_eq!(a.next_u64(), b.next_u64());
```

Apache License 2.0.
