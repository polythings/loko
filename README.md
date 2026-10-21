# loko

Resources for Rust programs: the time, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals. Code takes `&impl Clock` (or `&(impl Clock + Random)`, …), the
program passes the system's resources, and tests pass resources they control. The same code runs natively, under WASI,
and in browsers and Workers.

loko is what the [polythings](https://github.com/polythings) projects share (Timelake, lakekit, and lakeglass among
them), and anyone can use it.

```rust
use std::time::Duration;

use loko::{Clock, Random, TestPlatform};

/// An ID from the time and a random suffix.
fn id(resources: &(impl Clock + Random)) -> String {
    format!("{}-{:04x}", resources.now_ms(), resources.next_u64() & 0xffff)
}

let platform = TestPlatform::default(); // TokioPlatform, StdPlatform, or JsPlatform in a program
platform.clock.advance(Duration::from_secs(1));
assert_eq!(id(&platform), "1000-cdaf"); // the same on every run
```

## Resources

| Resource | Methods | For tests | System | JavaScript (feature `js`) |
|---|---|---|---|---|
| [`Clock`](crates/loko-clock) | `now`, `now_ms`, `sleep` | `TestClock` | `StdClock`, `TokioClock` (feature `tokio`) | `JsClock` |
| [`Random`](crates/loko-random) | `next_u64`, `fill` | `SeededRandom` | `OsRandom` (feature `getrandom`) | `JsRandom` |
| [`Env`](crates/loko-env) | `var`, `vars` | `MapEnv` | `SystemEnv` | `MapEnv` |
| [`Console`](crates/loko-console) | `print_line`, `print_error_line` | `TestConsole` | `StdConsole` | `JsConsole` |
| [`Store`](crates/loko-store) (feature `store`) | get, put, put-if-absent, upload, list, delete | `MemoryStore` | [`ObjectStoreAdapter`](crates/loko-object-store) (feature `object-store`) | the application's binding |

Futures aren't required to be `Send`, so a resource can wrap JavaScript promises. With feature `rand`, any `rand_core`
generator in a `Mutex` is a `Random`.

## Platforms

A platform is one of each of `Clock`, `Random`, `Env`, and `Console`, and is each of them:

| Platform | Clock | Random | Env | Console | Where |
|---|---|---|---|---|---|
| `TestPlatform` | `TestClock` | `SeededRandom` | `MapEnv` | `TestConsole` | Tests, anywhere |
| `StdPlatform` | `StdClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native and WASI, without an async runtime |
| `TokioPlatform` | `TokioClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native, under tokio (feature `tokio`) |
| `JsPlatform` | `JsClock` | `JsRandom` | `MapEnv` | `JsConsole` | Browsers, Workers, and Node (feature `js`) |

## Your own resources and bundles

`#[loko::resource]` makes a trait a resource, and `#[derive(Provides)]` makes a struct each resource its fields provide.
A program bundles a platform with a store and its own resources; a resource's implementation can need other resources.

```rust
use loko::prelude::*;
use loko::store::MemoryStore;

#[loko::resource]
trait Greeter {
    fn greet(&self, name: &str) -> String;
}

#[derive(Provides)]
struct App<G> {
    #[provides(Clock, Random, Env, Console)]
    platform: TestPlatform,
    #[provides(Store)]
    store: MemoryStore,
    #[provides(Greeter)]
    greeter: G,
}

fn welcome(app: &(impl Greeter + Env + Console)) -> std::io::Result<()> {
    let name = app.var("USER").unwrap_or_else(|| "stranger".into());
    app.print_line(&app.greet(&name))
}
```

`#[resource]` on `trait Greeter` adds `trait HasGreeter` (a type that holds a `Greeter`) and makes every `HasGreeter`
a `Greeter`; the derive implements `HasGreeter` for `App`. It names `HasGreeter` next to the path you give, so bring
both in (`loko::prelude::*` does for loko's resources) or write the path (`#[provides(loko::Clock)]`).

## Wrappers

A wrapper holds one implementation of a resource and is itself one, changing some methods. `#[loko::wraps(field:
Type)]` on its `impl` forwards every method it doesn't write to the field:

```rust
use loko::prelude::*;

/// Shouts standard output; standard error passes as it is.
struct Shouting<C> {
    inner: C,
}

#[loko::wraps(inner: C)]
impl<C: Console> Console for Shouting<C> {
    fn print_line(&self, line: &str) -> std::io::Result<()> {
        self.inner.print_line(&line.to_uppercase())
    }
}
```

Wrappers stack, and loko has some ready-made:

| Wrapper | What it does |
|---|---|
| `OffsetClock` | Another clock, ahead or behind by a fixed amount, for testing against clock skew |
| `CountingStore` | Counts the calls of each kind it passes on |
| `ReadOnlyStore` | Reads through, refuses writes |
| `PrefixedStore` | Passes on only URIs under a prefix, refusing `..` and the rest |

`CountingStore::new(ReadOnlyStore::new(PrefixedStore::new(s3, "s3://bucket/tenant-a/")))` is a store that counts
its requests, never writes, and stays inside one tenant's data.

A wrapper keeps the inner resource's associated types, and a provided method it doesn't write (such as `now_ms`)
forwards to the inner one too, so write it when it depends on a method the wrapper changes.

## Crates

| Crate | What it holds |
|---|---|
| [`loko`](crates/loko) | Everything below, the platforms, and a prelude, with a feature per optional part |
| [`loko-clock`](crates/loko-clock), [`loko-random`](crates/loko-random), [`loko-env`](crates/loko-env), [`loko-console`](crates/loko-console), [`loko-store`](crates/loko-store) | One resource each, with its implementations |
| [`loko-object-store`](crates/loko-object-store) | A `Store` over an `object_store` client: S3, GCS, Azure, local files, memory |
| [`loko-macros`](crates/loko-macros) | `#[resource]`, `#[derive(Provides)]`, and `#[wraps]` |

A library that needs one resource can depend on its crate alone.

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
