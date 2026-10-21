//! Resources and bundles defined outside loko work like loko's own.

use std::time::Duration;

use loko::clock::TestClock;
use loko::prelude::*;

/// A resource of this crate's own.
#[loko::resource]
trait Greeter {
    /// A greeting for `name`.
    fn greet(&self, name: &str) -> String;
}

/// A greeter that needs other resources: the clock for the time of day.
struct ClockGreeter<C> {
    clock: C,
}

impl<C: Clock> Greeter for ClockGreeter<C> {
    fn greet(&self, name: &str) -> String {
        let hour = self.clock.now_ms() / 3_600_000 % 24;
        if hour < 12 { format!("Good morning, {name}") } else { format!("Good afternoon, {name}") }
    }
}

/// A bundle of a platform and the greeter, generic over the greeter.
#[derive(Provides)]
struct App<G> {
    #[provides(Clock, Random, Env, Console)]
    platform: TestPlatform,
    #[provides(Greeter)]
    greeter: G,
}

/// Code that needs a mix of loko's resources and this crate's.
fn welcome(app: &(impl Greeter + Console + Env)) {
    let name = app.var("USER").unwrap_or_else(|| "stranger".into());
    app.print_line(&app.greet(&name)).expect("a test console doesn't fail");
}

#[test]
fn own_resources_bundle_with_loko_resources() {
    let platform = TestPlatform::new().with_var("USER", "Ada");
    let greeter = ClockGreeter { clock: platform.clock.clone() };
    let app = App { platform: platform.clone(), greeter };
    welcome(&app);
    platform.clock.advance(Duration::from_hours(13));
    welcome(&app);
    assert_eq!(platform.console.lines(), ["Good morning, Ada", "Good afternoon, Ada"]);
    assert_eq!(app.next_u64(), 0xE220_A839_7B1D_CDAF, "Random forwards to the platform");
}

/// Tuple structs bundle too.
#[derive(Provides)]
struct Pair(#[provides(Clock)] TestPlatform, #[provides(Greeter)] ClockGreeter<TestClock>);

#[test]
fn tuple_bundles_forward() {
    let platform = TestPlatform::new().at_ms(15 * 3_600_000);
    let pair = Pair(platform.clone(), ClockGreeter { clock: platform.clock.clone() });
    assert_eq!(pair.now_ms(), 15 * 3_600_000);
    assert_eq!(pair.greet("Lin"), "Good afternoon, Lin");
}

#[cfg(feature = "store")]
mod store {
    use loko::store::{MemoryStore, Upload};

    use super::*;

    #[derive(Clone, Provides)]
    struct Stored {
        #[provides(Store)]
        store: MemoryStore,
        #[provides(Clock)]
        platform: TestPlatform,
    }

    /// Async methods, an associated type, and default methods forward.
    #[test]
    fn async_resources_forward() {
        let app = Stored { store: MemoryStore::new(), platform: TestPlatform::new().at_ms(42) };
        let run = async {
            let uri = format!("memory:///logs/{}", app.now_ms());
            assert!(app.put_if_absent(&uri, b"a".to_vec()).await.unwrap());
            assert!(!app.put_if_absent(&uri, b"b".to_vec()).await.unwrap());
            let mut upload = app.upload("memory:///big").await.unwrap();
            upload.put_part(b"x".to_vec()).await.unwrap();
            upload.complete().await.unwrap();
            app.sleep(Duration::from_secs(1)).await;
            let listed: Vec<_> = app.list("memory:///").await.unwrap().into_iter().map(|l| l.uri).collect();
            (listed, app.get(&uri).await.unwrap())
        };
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready((listed, got)) = std::pin::pin!(run).poll(&mut cx) else {
            panic!("memory and test resources are ready at once");
        };
        assert_eq!(listed, ["memory:///big", "memory:///logs/42"]);
        assert_eq!(got.as_deref(), Some(&b"a"[..]));
        assert_eq!(app.platform.now_ms(), 1_042);
    }
}

/// A console that shouts its standard output and passes standard error on as it is.
struct Shouting<C> {
    inner: C,
}

#[loko::wraps(inner: C)]
impl<C: Console> Console for Shouting<C> {
    fn print_line(&self, line: &str) -> std::io::Result<()> {
        self.inner.print_line(&line.to_uppercase())
    }
}

/// A greeter around another, adding an exclamation mark.
struct Excited<G>(G);

#[loko::wraps(0: G)]
impl<G: Greeter> Greeter for Excited<G> {
    fn greet(&self, name: &str) -> String {
        format!("{}!", self.0.greet(name))
    }
}

#[test]
fn wrappers_replace_some_methods_and_forward_the_rest() {
    let platform = TestPlatform::new();
    let console = Shouting { inner: platform.console.clone() };
    console.print_line("quiet").unwrap();
    console.print_error_line("as it is").unwrap();
    assert_eq!(platform.console.lines(), ["QUIET"]);
    assert_eq!(platform.console.error_lines(), ["as it is"]);

    let greeter = Excited(ClockGreeter { clock: platform.clock.clone() });
    assert_eq!(greeter.greet("Ada"), "Good morning, Ada!");
}

#[cfg(feature = "store")]
#[test]
fn store_wrappers_stack() {
    use loko::store::{CountingStore, MemoryStore, PrefixedStore, ReadOnlyStore};

    let memory = MemoryStore::new();
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut run = |f: std::pin::Pin<&mut dyn Future<Output = bool>>| f.poll(&mut cx).is_ready();
    assert!(run(std::pin::pin!(async { memory.put("memory://wh/a/t", b"x".to_vec()).await.is_ok() })));

    let counted = CountingStore::new(ReadOnlyStore::new(PrefixedStore::new(memory.clone(), "memory://wh/a/")));
    assert!(run(std::pin::pin!(async { counted.get("memory://wh/a/t").await.unwrap().is_some() })));
    assert!(run(std::pin::pin!(async { counted.get("memory://wh/b/t").await.is_err() })));
    assert!(run(std::pin::pin!(async { counted.put("memory://wh/a/u", vec![]).await.is_err() })));
    assert_eq!((counted.counts().get, counted.counts().put), (2, 1));
    assert_eq!(memory.uris(), ["memory://wh/a/t"]);
}
