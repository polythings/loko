# loko

Resources for Rust programs: the time, random numbers, environment variables, the console, and object storage, passed
to code as traits instead of reached for as globals. Code takes `&impl Clock` (or `&(impl Clock + Random)`, …), the
program passes the system's resources, and tests pass resources they control. The same code runs natively, under WASI,
and in browsers and Workers.

```toml
[dependencies]
loko = { version = "0.2", features = ["tokio", "store"] }
```

| Feature | Adds |
|---|---|
| `getrandom` (default) | `OsRandom` and `StdPlatform` (native and WASI) |
| `tokio` | `TokioClock` and `TokioPlatform` (native) |
| `js` | `JsClock`, `JsRandom`, `JsConsole`, and `JsPlatform` (`wasm32-unknown-unknown`) |
| `store` | `loko::store`: the `Store` resource, `MemoryStore`, and the store wrappers |
| `object-store` | `loko::object_store`: a `Store` over an `object_store` client (S3, GCS, Azure, local files) |

## Guide

The examples build on each other, from one resource to bundles of your own resources and wrappers.

### 1. One resource

A function that needs the time takes a `Clock` instead of calling `SystemTime::now()`. The program passes the system
clock; a test passes a clock it sets.

```rust
use loko::Clock;
use loko::clock::{StdClock, TestClock};

/// A log line stamped with the time.
fn stamp(clock: &impl Clock, message: &str) -> String {
    format!("[{}] {message}", clock.now_ms())
}

// A program: the system clock.
let line = stamp(&StdClock, "started");
assert!(line.ends_with("] started"));

// A test: a clock that moves only when told.
let clock = TestClock::at_ms(1_000);
assert_eq!(stamp(&clock, "started"), "[1000] started");
```

### 2. Waiting without waiting

`sleep` is a `Clock` method too. A test clock's sleep returns at once and moves the clock forward, so code that waits
(retries, timeouts, polling) runs in tests without waiting. Random numbers come from a generator that a `Random`
hands out (`rng()`), so the jitter is the same on every test run.

```rust
use std::time::Duration;

use loko::{Clock, Random, Rng, TestPlatform};

/// Calls `attempt` up to five times, waiting one to two seconds between tries.
async fn retry<T, E>(
    resources: &(impl Clock + Random),
    mut attempt: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut rng = resources.rng();
    let mut tries = 1;
    loop {
        match attempt() {
            Err(_) if tries < 5 => {
                let jitter = rng.next_u64() % 1_000;
                resources.sleep(Duration::from_millis(1_000 + jitter)).await;
                tries += 1;
            }
            result => return result,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // A program would pass `TokioPlatform::default()`.
    let platform = TestPlatform::new();
    let mut calls = 0;
    let result = retry(&platform, || {
        calls += 1;
        if calls < 3 { Err("busy") } else { Ok(calls) }
    })
    .await;
    assert_eq!(result, Ok(3));
    // Two waits of one to two seconds each, on the test clock: the test itself took no time.
    assert!((2_000..4_000).contains(&platform.now_ms()));
}
```

### 3. Several resources

A function can take each resource as its own argument, or one value that is all of them. A platform is one of each
of `Clock`, `Random`, `Env`, and `Console`: `TestPlatform` for tests, `StdPlatform` or `TokioPlatform` for programs,
and `JsPlatform` in the browser.

```rust
use loko::{Console, Env, TestPlatform};

/// Separate arguments: plain, and fine for two or three resources.
fn greet(env: &impl Env, console: &impl Console) -> std::io::Result<()> {
    let name = env.var("USER").unwrap_or_else(|| "stranger".into());
    console.print_line(&format!("Hello, {name}"))
}

/// One value: handier as the list grows, and easy to store.
fn greet_all(resources: &(impl Env + Console)) -> std::io::Result<()> {
    greet(resources, resources)
}

// A program would pass `StdPlatform::default()`.
let platform = TestPlatform::new().with_var("USER", "Ada");
greet_all(&platform).unwrap();
assert_eq!(platform.console.lines(), ["Hello, Ada"]);
```

### 4. Storage

`Store` reads and writes objects by URI, with a conditional write (`put_if_absent`) for the cases where two writers
must not overwrite each other. Tests use `MemoryStore`; programs use `ObjectStoreAdapter` over S3, GCS, Azure, or local
files (feature `object-store`, with the backend enabled on your `object_store` dependency).

```rust
use loko::store::{MemoryStore, Store};

/// Writes a report unless one with that name exists.
async fn save(store: &impl Store, name: &str, body: &str) -> loko::store::Result<bool> {
    store.put_if_absent(&format!("memory://reports/{name}"), body.as_bytes().to_vec()).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> loko::store::Result<()> {
    // A program: ObjectStoreAdapter::from_url("s3://bucket/reports", [("aws_region", "us-east-2")])?
    let store = MemoryStore::new();
    assert!(save(&store, "a.txt", "first").await?);
    assert!(!save(&store, "a.txt", "second").await?, "the first write wins");
    Ok(())
}
```

### 5. Bundles

When code needs a store and the time together, put them in one struct with `#[derive(Provides)]`: each field marked
`#[provides(…)]` makes the struct that resource. Generic fields let the program and its tests fill the same struct
differently.

```rust
use loko::prelude::*;
use loko::store::MemoryStore;

#[derive(Provides)]
struct App<P, S> {
    #[provides(Clock, Random, Env, Console)]
    platform: P,
    #[provides(Store)]
    store: S,
}

/// Saves a report under a name that sorts by time.
async fn save_report(app: &(impl Store + Clock), body: &str) -> loko::store::Result<String> {
    let uri = format!("memory://reports/{}.txt", app.now_ms());
    app.put(&uri, body.as_bytes().to_vec()).await?;
    Ok(uri)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> loko::store::Result<()> {
    // A program: App { platform: TokioPlatform::default(), store: ObjectStoreAdapter::from_url(…)? }
    let app = App { platform: TestPlatform::new().at_ms(42), store: MemoryStore::new() };
    assert_eq!(save_report(&app, "all good").await?, "memory://reports/42.txt");
    assert_eq!(app.store.uris(), ["memory://reports/42.txt"]);
    Ok(())
}
```

`#[provides(Clock)]` names `HasClock` next to `Clock`, so bring both in: `loko::prelude::*` does for loko's resources,
or write the path (`#[provides(loko::Clock)]`).

### 6. Keeping resources in a struct

A library type holds its resources as one generic field and asks for what each method needs.

```rust
use loko::prelude::*;
use loko::store::MemoryStore;

/// Uploads files under a prefix, named by time and a random suffix.
struct Uploader<R> {
    resources: R,
    prefix: String,
}

impl<R: Store + Clock + Random> Uploader<R> {
    async fn upload(&self, body: Vec<u8>) -> loko::store::Result<String> {
        let r = &self.resources;
        let uri = format!("{}/{}-{:04x}", self.prefix, r.now_ms(), r.rng().next_u64() & 0xffff);
        r.put(&uri, body).await?;
        Ok(uri)
    }
}

#[derive(Provides)]
struct Test {
    #[provides(Clock, Random)]
    platform: TestPlatform,
    #[provides(Store)]
    store: MemoryStore,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> loko::store::Result<()> {
    let uploader = Uploader {
        resources: Test { platform: TestPlatform::new(), store: MemoryStore::new() },
        prefix: "memory://uploads".into(),
    };
    assert_eq!(uploader.upload(b"x".to_vec()).await?, "memory://uploads/0-7e6f");
    Ok(())
}
```

### 7. Your own resource

`#[loko::resource]` makes any trait a resource, so a program's own dependencies (a mailer, a payment client, a feature
flag service) work like loko's: a real implementation for the program, a test one that records, and a place in the
bundle. An implementation can use other resources.

```rust
use std::future::{Future, ready};
use std::sync::{Arc, Mutex};

use loko::prelude::*;

#[loko::resource]
trait Mailer {
    /// Sends `body` to `to`.
    fn send(&self, to: &str, body: &str) -> impl Future<Output = std::io::Result<()>>;
}

/// For tests: keeps what would have been sent.
#[derive(Clone, Default)]
struct TestMailer {
    sent: Arc<Mutex<Vec<(String, String)>>>,
}

impl Mailer for TestMailer {
    fn send(&self, to: &str, body: &str) -> impl Future<Output = std::io::Result<()>> {
        self.sent.lock().unwrap().push((to.into(), body.into()));
        ready(Ok(()))
    }
}

#[derive(Provides)]
struct App<M> {
    #[provides(Clock, Random, Env, Console)]
    platform: TestPlatform,
    #[provides(Mailer)]
    mailer: M,
}

/// Tells the administrator a backup finished.
async fn report_backup(app: &(impl Mailer + Env + Clock)) -> std::io::Result<()> {
    let to = app.var("ADMIN").unwrap_or_else(|| "root@localhost".into());
    app.send(&to, &format!("backup finished at {}", app.now_ms())).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let mailer = TestMailer::default();
    let app = App { platform: TestPlatform::new().with_var("ADMIN", "ops@example.com"), mailer: mailer.clone() };
    report_backup(&app).await?;
    assert_eq!(*mailer.sent.lock().unwrap(), [("ops@example.com".into(), "backup finished at 0".into())]);
    Ok(())
}
```

### 8. State in your own resources

Every resource method takes `&self`, never `&mut self`. That's what lets one value be several resources at once
(`greet(resources, resources)` in step 3), lets clones share state (a test keeps one `TestClock` and advances the one
the code holds), and lets resources be used across `.await`. A resource whose state changes keeps it behind a lock or
an atomic, as `TestMailer` above does with its `Mutex`. Three rules keep that safe:

- **Lock only for the synchronous part.** Don't hold a `std::sync::Mutex` guard or a `RefCell` borrow across an
  `.await`. While the task waits, another task can use the same resource and try to take the lock: a `Mutex` then
  blocks that thread for good (on a single-threaded runtime or in the browser, the whole program stops), and a
  `RefCell` panics. Clippy warns about both (`await_holding_lock`, `await_holding_refcell_ref`). If state really must
  stay locked while waiting, use an async mutex such as `tokio::sync::Mutex`.
- **Counters are atomics,** which need no lock.
- **Clones share state.** Cloning a resource that holds an `Arc` gives a second handle to the same state, not a copy.

```rust
use std::sync::Mutex;
use std::time::Duration;

use loko::{Clock, TestPlatform};

/// Remembers when each job ran.
struct Jobs {
    runs: Mutex<Vec<i64>>,
}

impl Jobs {
    async fn run(&self, clock: &impl Clock) {
        // Wait first, holding nothing; then lock, change, and unlock in one statement.
        clock.sleep(Duration::from_secs(1)).await;
        self.runs.lock().unwrap().push(clock.now_ms());
        // Not this: `let mut runs = self.runs.lock().unwrap(); clock.sleep(…).await; runs.push(…);`
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let jobs = Jobs { runs: Mutex::new(vec![]) };
    let platform = TestPlatform::new();
    jobs.run(&platform).await;
    jobs.run(&platform).await;
    assert_eq!(*jobs.runs.lock().unwrap(), [1_000, 2_000]);
}
```

A lock taken and released at once costs tens of nanoseconds, which matters only in hot loops. That's why `Random`
hands out generators instead of numbers: `resources.rng()` locks at most once, and drawing from the generator it
returns takes no lock at all, however many numbers you draw. The system implementations (`StdClock`, `OsRandom`,
`SystemEnv`, `StdConsole`) hold no state and take no lock.

### 9. Wrappers

A wrapper holds one implementation of a resource and is one itself, changing some methods. `#[loko::wraps(field:
Type)]` on its `impl` forwards every method it doesn't write to the field.

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

let platform = TestPlatform::new();
let console = Shouting { inner: platform.console.clone() };
console.print_line("done").unwrap();
console.print_error_line("as it is").unwrap();
assert_eq!(platform.console.lines(), ["DONE"]);
assert_eq!(platform.console.error_lines(), ["as it is"]);
```

Wrappers stack, and loko has some ready-made: `OffsetClock` (a clock ahead or behind another, for clock-skew tests),
`CountingStore`, `ReadOnlyStore`, and `PrefixedStore`.

```rust
use std::time::Duration;

use loko::Clock;
use loko::clock::{OffsetClock, TestClock};
use loko::store::{CountingStore, MemoryStore, PrefixedStore, ReadOnlyStore, Store};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // A store that counts its calls, never writes, and stays under one prefix.
    let memory = MemoryStore::new();
    memory.put("memory://tenant-a/report", b"x".to_vec()).await.unwrap();
    let store = CountingStore::new(ReadOnlyStore::new(PrefixedStore::new(memory, "memory://tenant-a/")));
    assert!(store.get("memory://tenant-a/report").await.unwrap().is_some());
    assert!(store.get("memory://tenant-b/report").await.is_err(), "outside the prefix");
    assert!(store.put("memory://tenant-a/new", vec![]).await.is_err(), "read-only");
    assert_eq!((store.counts().get, store.counts().put), (2, 1));

    // A second machine whose clock runs five seconds behind.
    let clock = TestClock::at_ms(10_000);
    let late = OffsetClock::behind(clock.clone(), Duration::from_secs(5));
    assert_eq!(late.now_ms(), 5_000);
}
```

A wrapper keeps the inner resource's associated types, and a provided method it doesn't write (such as `now_ms`)
forwards to the inner one too, so write it when it depends on a method the wrapper changes.

### 10. In the browser

With feature `js` on `wasm32-unknown-unknown`, `JsPlatform` uses `Date.now`, `setTimeout`, `crypto.getRandomValues`,
and the JavaScript console. The `retry` from step 2 runs unchanged:

```rust,ignore
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn fetch_with_retries() -> Result<u32, JsValue> {
    let platform = loko::JsPlatform::default();
    retry(&platform, || try_once()).await.map_err(JsValue::from)
}
```

Futures aren't required to be `Send`, so a resource can wrap JavaScript promises, as `JsClock` does.

## Related crates

[COMPARISON.md](COMPARISON.md) compares loko with forwarding macros (delegate, ambassador), dependency-injection crates
(fundle, shaku, nject, service-builder, dill, dependency-injector, ferrous-di, reinhardt-di), tower, and cap-std: what
each is for, when to use which, and what loko does differently.

## Reference

### Resources

| Resource | Methods | For tests | System | JavaScript (feature `js`) |
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
| `#[loko::resource]` | Makes a trait a resource: adds `HasX` and makes every `HasX` an `X` |
| `#[derive(Provides)]` | Makes a struct each resource its `#[provides(…)]` fields provide |
| `#[loko::wraps(field: Type)]` | Makes an `impl X for Wrapper` a wrapper: the methods written replace `X`'s, the rest forward |

### Crates

| Crate | What it holds |
|---|---|
| [`loko`](crates/loko) | Everything below, the platforms, and a prelude, with a feature per optional part |
| [`loko-clock`](crates/loko-clock), [`loko-random`](crates/loko-random), [`loko-env`](crates/loko-env), [`loko-console`](crates/loko-console), [`loko-store`](crates/loko-store) | One resource each, with its implementations and wrappers |
| [`loko-object-store`](crates/loko-object-store) | A `Store` over an `object_store` client: S3, GCS, Azure, local files, memory |
| [`loko-macros`](crates/loko-macros) | `#[resource]`, `#[derive(Provides)]`, and `#[wraps]` |

A library that needs one resource can depend on its crate alone.

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
