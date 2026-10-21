# loko

Services for Rust programs: the time, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals, with test, system, and JavaScript implementations, and platforms
that bundle one of each.

```rust
use std::time::{Duration, SystemTime};

use loko::Clock;
use loko::clock::TestClock;

/// Whether a session that started at `started` has expired.
fn expired(clock: &impl Clock, started: SystemTime) -> bool {
    clock.now() > started + Duration::from_secs(60 * 60)
}

let clock = TestClock::at_ms(0); // StdClock, TokioClock, or JsClock in a program
let started = clock.now();
clock.advance(Duration::from_secs(60 * 60 + 1)); // an hour passes, at once
assert!(expired(&clock, started));
```

| Feature | Adds |
|---|---|
| `getrandom` (default) | `OsRandom` and `StdPlatform` (native and WASI) |
| `tokio` | `TokioClock` and `TokioPlatform` (native) |
| `js` | `JsClock`, `JsRandom`, `JsConsole`, and `JsPlatform` (`wasm32-unknown-unknown`) |
| `store` | `loko::store`: the `Store` service and `MemoryStore` |
| `object-store` | `loko::object_store`: a `Store` over an `object_store` client |

`#[loko::service]` makes your own traits services and `#[loko::bundle]` bundles them with loko's. The
[repository](https://github.com/polythings/loko) has the details.

Apache License 2.0.
