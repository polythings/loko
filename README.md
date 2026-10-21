# loko

Services for Rust programs: the clock, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals. The program passes the real ones, tests pass ones they control,
and the same code runs natively, under WASI, and in the browser.

## Why

Code that calls `SystemTime::now()`, sleeps, or draws from a global random generator is hard to test: checking a
one-hour timeout takes an hour, and random retries behave differently on every run. With loko, the code takes the clock
as an argument, and the test decides what time it is:

```rust
use std::time::{Duration, SystemTime};

use loko::Clock;
use loko::clock::TestClock;

/// Whether a session that started at `started` has expired.
fn expired(clock: &impl Clock, started: SystemTime) -> bool {
    clock.now() > started + Duration::from_secs(60 * 60)
}

let clock = TestClock::at_ms(0);
let started = clock.now();
assert!(!expired(&clock, started));
clock.advance(Duration::from_secs(60 * 60 + 1)); // an hour passes, at once
assert!(expired(&clock, started));
```

The program passes `StdClock`, `TokioClock`, or `JsClock` in the browser, and `expired` stays as it is. What you get:

- **Tests that control time.** A test clock moves only when told, and sleeping on it returns at once, so retries,
  timeouts, and polling loops run in tests without waiting.
- **Random numbers that repeat in tests,** and secure ones in programs, with signatures that say which is needed.
- **One codebase for native, WASI, and the browser.** Platforms bundle the right implementations for each, and nothing
  requires `Send`, so services backed by JavaScript work.
- **Storage you can swap:** memory in tests, S3, GCS, Azure, or local files in production.
- **Tests that set environment variables and read what was printed,** without touching the process.
- **Your own services and wrappers,** with three small macros, in the same style.

```toml
[dependencies]
loko = { version = "0.2", features = ["tokio", "store"] }
```

| Feature | Adds |
|---|---|
| `getrandom` (default) | `OsRandom` and `StdPlatform` (native and WASI) |
| `tokio` | `TokioClock` and `TokioPlatform` (native) |
| `js` | `JsClock`, `JsRandom`, `JsConsole`, and `JsPlatform` (`wasm32-unknown-unknown`) |
| `store` | `loko::store`: the `Store` service, `MemoryStore`, and the store wrappers |
| `object-store` | `loko::object_store`: a `Store` over an `object_store` client (S3, GCS, Azure, local files) |

## Guide

### 1. Waiting without waiting

A retry loop that waits a second between tries. Its test takes no time, because a test platform's sleeps move its
clock instead of blocking.

```rust
use std::time::Duration;

use loko::{Clock, TestPlatform};

/// Tries `attempt` up to three times, a second apart.
async fn retry<T, E>(clock: &impl Clock, mut attempt: impl FnMut() -> Result<T, E>) -> Result<T, E> {
    for _ in 1..3 {
        if let Ok(value) = attempt() {
            return Ok(value);
        }
        clock.sleep(Duration::from_secs(1)).await;
    }
    attempt()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let platform = TestPlatform::new(); // TokioPlatform::default() in a program
    let mut calls = 0;
    let result = retry(&platform, || {
        calls += 1;
        if calls < 3 { Err("busy") } else { Ok(calls) }
    })
    .await;
    assert_eq!(result, Ok(3));
    assert_eq!(platform.now_ms(), 2_000, "two seconds of test time, none of real time");
}
```

### 2. Random numbers that repeat

A `Random` hands out generators (`rng()`). In tests they come from a seed, so every run makes the same choices.

```rust
use loko::{Random, Rng, TestPlatform};

/// Picks one of `servers` at random.
fn pick<'a>(random: &impl Random, servers: &[&'a str]) -> &'a str {
    servers[(random.rng().next_u64() % servers.len() as u64) as usize]
}

let servers = ["a", "b", "c"];
assert_eq!(pick(&TestPlatform::new(), &servers), pick(&TestPlatform::new(), &servers));
```

Code that makes secrets asks for a secure generator, `fn token(random: &impl Random<Rng: CryptoRng>)`, so a seeded
one can't be passed to it by mistake. Drawing many numbers from one `rng()` takes no locks.

### 3. Several services, one platform

A function can ask for several services at once. A platform is one of each of `Clock`, `Random`, `Env`, and `Console`:
`TestPlatform` for tests, `StdPlatform` or `TokioPlatform` for programs, `JsPlatform` in the browser.

```rust
use loko::{Console, Env, TestPlatform};

fn greet(services: &(impl Env + Console)) {
    let name = services.var("USER").unwrap_or_else(|| "stranger".into());
    services.print_line(&format!("Hello, {name}")).unwrap();
}

let platform = TestPlatform::new().with_var("USER", "Ada");
greet(&platform);
assert_eq!(platform.console.lines(), ["Hello, Ada"]);
```

### 4. Storage

`Store` reads and writes objects by URI, including a write that succeeds only if nothing is there yet, for writers
that must not overwrite each other.

```rust
use loko::store::{MemoryStore, Store};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let store = MemoryStore::new(); // ObjectStoreAdapter::from_url("s3://bucket/…", options)? in production
    assert!(store.put_if_absent("memory://a.txt", b"first".to_vec()).await.unwrap());
    assert!(!store.put_if_absent("memory://a.txt", b"second".to_vec()).await.unwrap(), "the first write wins");
}
```

### 5. Bundles

When code needs a platform and a store together, put them in one struct: `#[loko::bundle]` makes it each service its
fields hold, as marked with `#[loko::has(…)]`.

```rust
use loko::prelude::*;
use loko::store::MemoryStore;

#[loko::bundle]
struct App<P, S> {
    #[loko::has(Clock, Random, Env, Console)]
    platform: P,
    #[loko::has(Store)]
    store: S,
}

/// Saves a report named by the time.
async fn save_report(app: &(impl Store + Clock)) -> loko::store::Result<String> {
    let uri = format!("memory://reports/{}.txt", app.now_ms());
    app.put(&uri, b"ok".to_vec()).await?;
    Ok(uri)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // TokioPlatform and an S3 store in production.
    let app = App { platform: TestPlatform::new().at_ms(42), store: MemoryStore::new() };
    assert_eq!(save_report(&app).await.unwrap(), "memory://reports/42.txt");
}
```

`#[loko::has(Clock)]` names `HasClock` next to `Clock`, so bring both in (`loko::prelude::*` does). A library type can
hold its services the same way, as one generic field: `struct Uploader<S> { services: S }` with
`impl<S: Store + Clock> Uploader<S>`.

### 6. Your own service

`#[loko::service]` makes any trait a service, so your own dependencies (a mailer, a payment client) work like loko's:
a real implementation for the program, a test one that records, and a field in the bundle.

```rust
use std::sync::Mutex;

use loko::prelude::*;

#[loko::service]
trait Mailer {
    fn send(&self, to: &str, body: &str);
}

/// For tests: remembers what was sent.
#[derive(Default)]
struct TestMailer {
    sent: Mutex<Vec<String>>,
}

impl Mailer for TestMailer {
    fn send(&self, to: &str, body: &str) {
        self.sent.lock().unwrap().push(format!("{to}: {body}"));
    }
}

fn report_backup(app: &(impl Mailer + Env)) {
    let to = app.var("ADMIN").unwrap_or_else(|| "root".into());
    app.send(&to, "backup finished");
}

#[loko::bundle]
struct App {
    #[loko::has(Env)]
    platform: TestPlatform,
    #[loko::has(Mailer)]
    mailer: TestMailer,
}

let app = App { platform: TestPlatform::new().with_var("ADMIN", "ops"), mailer: TestMailer::default() };
report_backup(&app);
assert_eq!(*app.mailer.sent.lock().unwrap(), ["ops: backup finished"]);
```

### 7. Your own implementation and platform

loko's services are ordinary traits, so you can implement one: write the required methods, and the provided ones
(`now_ms`) come with it. A platform is a bundle with one field per service, which is how loko's own are built.

```rust
use std::future::{Future, ready};
use std::time::{Duration, SystemTime};

use loko::console::TestConsole;
use loko::env::MapEnv;
use loko::prelude::*;
use loko::random::SeededRandom;

/// A clock stopped at one moment, for output that must be the same on every run.
struct FrozenClock(SystemTime);

impl Clock for FrozenClock {
    fn now(&self) -> SystemTime {
        self.0
    }

    fn sleep(&self, _: Duration) -> impl Future<Output = ()> {
        ready(())
    }
}

/// Everything fixed: for snapshot tests.
#[loko::bundle]
struct SnapshotPlatform {
    #[loko::has(Clock)]
    clock: FrozenClock,
    #[loko::has(Random)]
    random: SeededRandom,
    #[loko::has(Env)]
    env: MapEnv,
    #[loko::has(Console)]
    console: TestConsole,
}

let platform = SnapshotPlatform {
    clock: FrozenClock(loko::clock::from_ms(1_700_000_000_000).unwrap()),
    random: SeededRandom::new(1),
    env: MapEnv::new(),
    console: TestConsole::new(),
};
assert_eq!(platform.now_ms(), 1_700_000_000_000);
```

### 8. Wrappers

A wrapper holds one implementation and is one itself, changing some methods. `#[loko::wraps(field: Type)]` forwards
every method it doesn't write to the field. This one speeds up any clock, test or real:

```rust
use std::future::Future;
use std::time::{Duration, SystemTime};

use loko::clock::{TestClock, to_ms};
use loko::prelude::*;

/// Any clock, running `speed` times faster from when the wrapper is made.
struct Faster<C> {
    inner: C,
    started: SystemTime,
    speed: u32,
}

#[loko::wraps(inner: C)]
impl<C: Clock> Clock for Faster<C> {
    fn now(&self) -> SystemTime {
        let elapsed = self.inner.now().duration_since(self.started).unwrap_or_default();
        self.started + elapsed * self.speed
    }

    // Written too: the inner clock's `now_ms` would leave out the speed-up.
    fn now_ms(&self) -> i64 {
        to_ms(Clock::now(self))
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        self.inner.sleep(duration / self.speed)
    }
}

let real = TestClock::at_ms(0);
let fast = Faster { inner: real.clone(), started: real.now(), speed: 60 };
real.advance(Duration::from_secs(1));
assert_eq!(fast.now_ms(), 60_000, "a second of the inner clock is a minute");
```

loko has some ready-made, and they stack: `OffsetClock` (a clock ahead or behind another, for clock-skew tests),
`CountingStore`, `ReadOnlyStore`, and `PrefixedStore`.

```rust
use loko::store::{CountingStore, MemoryStore, PrefixedStore, ReadOnlyStore, Store};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Counts its calls, never writes, and stays inside one tenant's data.
    let store = CountingStore::new(ReadOnlyStore::new(PrefixedStore::new(MemoryStore::new(), "memory://tenant-a/")));
    assert!(store.get("memory://tenant-b/report").await.is_err(), "outside the prefix");
    assert!(store.put("memory://tenant-a/report", vec![]).await.is_err(), "read-only");
    assert_eq!(store.counts().total(), 2);
}
```

### 9. Services with state

Every service method takes `&self`, so one value can be several services (`greet(&platform)` above), clones share
state (a test advances the clock it handed to the code), and services can be used across `.await`. A service whose
state changes keeps it behind a lock or an atomic, as `TestMailer` does. Three rules:

- **Lock only for the synchronous part.** Never hold a `std::sync::Mutex` guard or a `RefCell` borrow across `.await`:
  another task using the service meanwhile would deadlock or panic. Clippy warns about both (`await_holding_lock`,
  `await_holding_refcell_ref`). If state must stay locked while waiting, use an async mutex (`tokio::sync::Mutex`).
- **Counters are atomics,** which need no lock.
- **Clones share state.** Cloning a service that holds an `Arc` gives a second handle to the same state.

### 10. In the browser

With feature `js` on `wasm32-unknown-unknown`, `JsPlatform` uses `Date.now`, `setTimeout`, `crypto.getRandomValues`,
and the JavaScript console. The `retry` from step 1 runs unchanged:

```rust,ignore
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn fetch_with_retries() -> Result<u32, JsValue> {
    let platform = loko::JsPlatform::default();
    retry(&platform, || try_once()).await.map_err(JsValue::from)
}
```

## Sans-I/O

Libraries that do no I/O of their own come in two shapes, and loko fits both:

- **State machines driven by the caller** (quinn-proto, str0m, smoltcp): the time comes in as an argument, the next
  deadline goes out, and the library never sleeps. The program around it waits with a `Clock`, and its tests run
  through hours of timers at once with a `TestClock`.
- **Libraries that call traits the application implements** (delta-kernel-rs's `Engine`, rustls's time and randomness
  providers): the library runs a chain of dependent steps and calls back for each read, write, or timestamp. That's the
  shape loko's services serve: the library takes `&(impl Store + Clock + Random)`, and the application chooses.

Choose the first for a protocol reacting to packets and timers, the second for reads and writes that depend on each
other.

## Related crates

[COMPARISON.md](COMPARISON.md) compares loko with forwarding macros (delegate, ambassador), dependency-injection crates
(fundle, shaku, nject, service-builder, dill, dependency-injector, ferrous-di, reinhardt-di), tower, and cap-std: what
each is for, when to use which, and what loko does differently.

## Reference

### Services

| Service | Methods | For tests | System | JavaScript (feature `js`) |
|---|---|---|---|---|
| [`Clock`](crates/loko-clock) | `now`, `now_ms`, `sleep` | `TestClock` | `StdClock`, `TokioClock` (feature `tokio`) | `JsClock` |
| [`Random`](crates/loko-random) | `rng`, a generator of your own | `SeededRandom` | `OsRandom` (feature `getrandom`) | `JsRandom` |
| [`Env`](crates/loko-env) | `var`, `vars` | `MapEnv` | `SystemEnv` | `MapEnv` |
| [`Console`](crates/loko-console) | `print_line`, `print_error_line` | `TestConsole` | `StdConsole` | `JsConsole` |
| [`Store`](crates/loko-store) (feature `store`) | get, put, put-if-absent, upload, list, delete | `MemoryStore` | [`ObjectStoreAdapter`](crates/loko-object-store) (feature `object-store`) | the application's binding |

### Platforms

| Platform | Clock | Random | Env | Console | Where |
|---|---|---|---|---|---|
| `TestPlatform` | `TestClock` | `SeededRandom` | `MapEnv` | `TestConsole` | Tests, anywhere |
| `StdPlatform` | `StdClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native and WASI, without an async runtime |
| `TokioPlatform` | `TokioClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native, under tokio (feature `tokio`) |
| `JsPlatform` | `JsClock` | `JsRandom` | `MapEnv` | `JsConsole` | Browsers, Workers, and Node (feature `js`) |

### Wrappers

| Wrapper | What it does |
|---|---|
| `OffsetClock` | Another clock, ahead or behind by a fixed amount |
| `CountingStore` | Counts the calls of each kind it passes on |
| `ReadOnlyStore` | Reads through, refuses writes |
| `PrefixedStore` | Passes on only URIs under a prefix, refusing `..` and the rest |

### Macros

| Macro | What it does |
|---|---|
| `#[loko::service]` | Makes a trait a service: adds `HasX` and makes every `HasX` an `X` |
| `#[loko::bundle]` | Makes a struct each service its `#[loko::has(…)]` fields provide |
| `#[loko::wraps(field: Type)]` | Makes an `impl X for Wrapper` a wrapper: the methods written replace `X`'s, the rest forward |

### Crates

| Crate | What it holds |
|---|---|
| [`loko`](crates/loko) | Everything below, the platforms, and a prelude, with a feature per optional part |
| [`loko-clock`](crates/loko-clock), [`loko-random`](crates/loko-random), [`loko-env`](crates/loko-env), [`loko-console`](crates/loko-console), [`loko-store`](crates/loko-store) | One service each, with its implementations and wrappers |
| [`loko-object-store`](crates/loko-object-store) | A `Store` over an `object_store` client: S3, GCS, Azure, local files, memory |
| [`loko-macros`](crates/loko-macros) | `#[service]`, `#[loko::bundle]`, and `#[wraps]` |

A library that needs one service can depend on its crate alone. The service crates have no dependencies of their
own (`loko-random` has `rand_core`), and no macros: only `loko` and code defining its own services use `loko-macros`.

## Development

```bash
cargo test --workspace --all-features                                    # tests, offline, and this README's examples
cargo build --workspace --all-features --target wasm32-unknown-unknown   # browsers and Workers
cargo build --workspace --all-features --target wasm32-wasip2            # WASI
prek run --all-files --hook-stage pre-push                               # formatters and linters
```

[CONTRIBUTING.md](CONTRIBUTING.md) is the code guide, and [spec/](spec/README.md) holds the design.

## License

Apache License 2.0 ([LICENSE](LICENSE)).
