# loko

Services for Rust programs: the time, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals. Code takes `&impl Clock` (or `&(impl Clock + Random)`, …), the
program passes the system's services, and tests pass services they control. The same code runs natively, under WASI,
and in browsers and Workers.

loko is what the [polythings](https://github.com/polythings) projects share (Timelake, lakekit, and lakeglass among
them), and anyone can use it.

```rust
use std::time::Duration;

use loko::{Clock, Random, TestPlatform};

/// An ID from the time and a random suffix.
fn id(services: &(impl Clock + Random)) -> String {
    format!("{}-{:04x}", services.now_ms(), services.next_u64() & 0xffff)
}

let platform = TestPlatform::default(); // TokioPlatform, StdPlatform, or JsPlatform in a program
platform.clock.advance(Duration::from_secs(1));
assert_eq!(id(&platform), "1000-cdaf"); // the same on every run
```

## Services

| Service | Methods | For tests | System | JavaScript (feature `js`) |
|---|---|---|---|---|
| [`Clock`](crates/loko-clock) | `now`, `now_ms`, `sleep` | `TestClock` | `StdClock`, `TokioClock` (feature `tokio`) | `JsClock` |
| [`Random`](crates/loko-random) | `next_u64`, `fill` | `SeededRandom` | `OsRandom` (feature `getrandom`) | `JsRandom` |
| [`Env`](crates/loko-env) | `var`, `vars` | `MapEnv` | `SystemEnv` | `MapEnv` |
| [`Console`](crates/loko-console) | `print_line`, `print_error_line` | `TestConsole` | `StdConsole` | `JsConsole` |
| [`Store`](crates/loko-store) (feature `store`) | get, put, put-if-absent, upload, list, delete | `MemoryStore` | [`ObjectStoreAdapter`](crates/loko-object-store) (feature `object-store`) | the application's binding |

Futures aren't required to be `Send`, so a service can wrap JavaScript promises. With feature `rand`, any `rand_core`
generator in a `Mutex` is a `Random`.

## Platforms

A platform is one of each of `Clock`, `Random`, `Env`, and `Console`, and is each of them:

| Platform | Clock | Random | Env | Console | Where |
|---|---|---|---|---|---|
| `TestPlatform` | `TestClock` | `SeededRandom` | `MapEnv` | `TestConsole` | Tests, anywhere |
| `StdPlatform` | `StdClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native and WASI, without an async runtime |
| `TokioPlatform` | `TokioClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native, under tokio (feature `tokio`) |
| `JsPlatform` | `JsClock` | `JsRandom` | `MapEnv` | `JsConsole` | Browsers, Workers, and Node (feature `js`) |

## Your own services and bundles

`#[loko::service]` makes a trait a service, and `#[derive(Services)]` makes a struct each service its fields provide.
A program bundles a platform with a store and its own services; a service's implementation can need other services.

```rust
use loko::prelude::*;
use loko::store::MemoryStore;

#[loko::service]
trait Greeter {
    fn greet(&self, name: &str) -> String;
}

#[derive(Services)]
struct App<G> {
    #[service(Clock, Random, Env, Console)]
    platform: TestPlatform,
    #[service(Store)]
    store: MemoryStore,
    #[service(Greeter)]
    greeter: G,
}

fn welcome(app: &(impl Greeter + Env + Console)) -> std::io::Result<()> {
    let name = app.var("USER").unwrap_or_else(|| "stranger".into());
    app.print_line(&app.greet(&name))
}
```

`#[service]` on `trait Greeter` adds `trait HasGreeter` (a type that holds a `Greeter`) and makes every `HasGreeter`
a `Greeter`; the derive implements `HasGreeter` for `App`. It names `HasGreeter` next to the path you give, so bring
both in (`loko::prelude::*` does for loko's services) or write the path (`#[service(loko::Clock)]`).

## Crates

| Crate | What it holds |
|---|---|
| [`loko`](crates/loko) | Everything below, the platforms, and a prelude, with a feature per optional part |
| [`loko-clock`](crates/loko-clock), [`loko-random`](crates/loko-random), [`loko-env`](crates/loko-env), [`loko-console`](crates/loko-console), [`loko-store`](crates/loko-store) | One service each, with its implementations |
| [`loko-object-store`](crates/loko-object-store) | A `Store` over an `object_store` client: S3, GCS, Azure, local files, memory |
| [`loko-macros`](crates/loko-macros) | `#[service]` and `#[derive(Services)]` |

A library that needs one service can depend on its crate alone.

## Development

```bash
cargo test --workspace --all-features                                    # tests, offline
cargo build --workspace --all-features --target wasm32-unknown-unknown   # browsers and Workers
cargo build --workspace --all-features --target wasm32-wasip2            # WASI
prek run --all-files --hook-stage pre-push                               # formatters and linters
```

[CONTRIBUTING.md](CONTRIBUTING.md) is the code guide, and [spec/](spec/README.md) holds the design.

## License

Apache License 2.0 ([LICENSE](LICENSE)).
