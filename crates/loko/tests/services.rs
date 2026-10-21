//! Services and bundles defined outside loko work like loko's own.

use std::time::Duration;

use loko::clock::TestClock;
use loko::prelude::*;

/// A service of this crate's own.
#[loko::service]
trait Greeter {
    /// A greeting for `name`.
    fn greet(&self, name: &str) -> String;
}

/// A greeter that needs other services: the clock for the time of day.
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
#[derive(Services)]
struct App<G> {
    #[service(Clock, Random, Env, Console)]
    platform: TestPlatform,
    #[service(Greeter)]
    greeter: G,
}

/// Code that needs a mix of loko's services and this crate's.
fn welcome(app: &(impl Greeter + Console + Env)) {
    let name = app.var("USER").unwrap_or_else(|| "stranger".into());
    app.print_line(&app.greet(&name)).expect("a test console doesn't fail");
}

#[test]
fn own_services_bundle_with_loko_services() {
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
#[derive(Services)]
struct Pair(#[service(Clock)] TestPlatform, #[service(Greeter)] ClockGreeter<TestClock>);

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

    #[derive(Clone, Services)]
    struct Stored {
        #[service(Store)]
        store: MemoryStore,
        #[service(Clock)]
        platform: TestPlatform,
    }

    /// Async methods, an associated type, and default methods forward.
    #[test]
    fn async_services_forward() {
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
            panic!("memory and test services are ready at once");
        };
        assert_eq!(listed, ["memory:///big", "memory:///logs/42"]);
        assert_eq!(got.as_deref(), Some(&b"a"[..]));
        assert_eq!(app.platform.now_ms(), 1_042);
    }
}
