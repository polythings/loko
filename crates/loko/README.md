# loko

Services for Rust programs: the time, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals, with test, system, and JavaScript implementations, and platforms
that bundle one of each.

```rust
use std::time::Duration;

use loko::{Clock, Random, TestPlatform};

fn id(services: &(impl Clock + Random)) -> String {
    format!("{}-{:04x}", services.now_ms(), services.next_u64() & 0xffff)
}

let platform = TestPlatform::default(); // TokioPlatform, StdPlatform, or JsPlatform in a program
platform.clock.advance(Duration::from_secs(1));
assert_eq!(id(&platform), "1000-cdaf");
```

| Feature | Adds |
|---|---|
| `getrandom` (default) | `OsRandom` and `StdPlatform` (native and WASI) |
| `tokio` | `TokioClock` and `TokioPlatform` (native) |
| `js` | `JsClock`, `JsRandom`, `JsConsole`, and `JsPlatform` (`wasm32-unknown-unknown`) |
| `rand` | `Random` for a `Mutex` around a `rand_core` generator |
| `store` | `loko::store`: the `Store` service and `MemoryStore` |
| `object-store` | `loko::object_store`: a `Store` over an `object_store` client |

`#[loko::service]` makes your own traits services and `#[derive(loko::Services)]` bundles them with loko's. The
[repository](https://github.com/polythings/loko) has the details.

Apache License 2.0.
