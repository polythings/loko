# loko-clock

[loko](https://github.com/polythings/loko)'s `Clock` service: the time and a timer, passed to code instead of read
from the system, so tests control time and the same code runs natively, under WASI, and in browsers.

- `Clock`: `now` (a `SystemTime`), `now_ms` (milliseconds since the Unix epoch), and `sleep(Duration)`. The future
  isn't required to be `Send`.
- `TestClock`: moves only when told (`advance`, `set`) or when slept on, so a test of a retry loop runs at once.
  Clones share the time.
- `StdClock`: the system clock, with a `sleep` that blocks the thread (native and WASI, no async runtime needed).
- `TokioClock` (feature `tokio`): the system clock and tokio's timer.
- `JsClock` (feature `js`, `wasm32-unknown-unknown`): `Date.now` and `setTimeout`.
- `OffsetClock`: another clock, ahead or behind by a fixed amount, for testing against clock skew.

```rust
use std::time::Duration;

use loko_clock::{Clock, TestClock};

let clock = TestClock::at_ms(1_000);
clock.advance(Duration::from_secs(1));
assert_eq!(clock.now_ms(), 2_000);
```

Apache License 2.0.
