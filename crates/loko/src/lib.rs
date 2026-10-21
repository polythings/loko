//! Resources passed to code instead of reached for: the time, randomness, environment variables,
//! the console, and object storage. Code takes `&impl Clock` (or `&(impl Clock + Random)`, …), the
//! program passes a platform, and tests pass a [`TestPlatform`] they control. The same code runs
//! natively and in WebAssembly.
//!
//! - Resources: [`clock`], [`random`], [`env`](mod@env), [`console`], and `store` (feature `store`), each a
//!   trait with test, native, and JavaScript implementations. The traits are also at the root.
//! - Platforms: [`TestPlatform`], `StdPlatform`, `TokioPlatform`, and `JsPlatform` bundle one of each.
//! - [`resource`] makes a trait a resource, [`Provides`] makes a struct each resource its fields
//!   provide, and [`wraps`] makes a wrapper that changes some of a resource's methods and forwards
//!   the rest. [`prelude`] brings in the resources with the `HasClock`-style traits these name.
//!
//! Resource methods take `&self`, so one value can be several resources and clones can share state.
//! A resource whose state changes keeps it behind a lock or an atomic, locked only for the
//! synchronous part of a call: a `std::sync::Mutex` guard or `RefCell` borrow held across `.await`
//! deadlocks or panics when another task uses the resource meanwhile.
//!
//! ```
//! use std::time::Duration;
//!
//! use loko::{Clock, Random, Rng, TestPlatform};
//!
//! /// An ID from the time and a random suffix.
//! fn id(resources: &(impl Clock + Random)) -> String {
//!     format!("{}-{:04x}", resources.now_ms(), resources.rng().next_u64() & 0xffff)
//! }
//!
//! let platform = TestPlatform::default();
//! platform.clock.advance(Duration::from_secs(1));
//! assert_eq!(id(&platform), "1000-7e6f");
//! ```
//!
//! A program bundles the resources it needs, its own among them:
//!
//! ```
//! # #[cfg(feature = "store")] {
//! use loko::prelude::*;
//! use loko::store::MemoryStore;
//!
//! #[derive(Provides)]
//! struct App {
//!     #[provides(Store)]
//!     store: MemoryStore,
//!     #[provides(Clock, Random)]
//!     platform: TestPlatform,
//! }
//!
//! async fn save(app: &(impl Store + Clock)) -> loko::store::Result<()> {
//!     app.put(&format!("memory:///logs/{}", app.now_ms()), b"started".to_vec()).await
//! }
//! # }
//! ```

pub use loko_clock as clock;
pub use loko_clock::{Clock, HasClock};
pub use loko_console as console;
pub use loko_console::{Console, HasConsole};
pub use loko_env as env;
pub use loko_env::{Env, HasEnv};
pub use loko_macros::{Provides, resource, wraps};
#[cfg(feature = "object-store")]
pub use loko_object_store as object_store;
pub use loko_random as random;
pub use loko_random::{CryptoRng, HasRandom, Random, Rng};
#[cfg(feature = "store")]
pub use loko_store as store;
#[cfg(feature = "store")]
pub use loko_store::{HasStore, Store};

mod platform;

/// The repository README's examples, run as doctests.
#[cfg(all(doctest, feature = "store", feature = "tokio"))]
#[doc = include_str!("../../../README.md")]
pub struct ReadmeExamples;

/// The resources, the `Has` traits that `#[derive(Provides)]` and `#[wraps]` name next to them, the
/// macros, and the platforms.
pub mod prelude {
    #[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
    pub use crate::JsPlatform;
    #[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
    pub use crate::StdPlatform;
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    pub use crate::TokioPlatform;
    pub use crate::{
        Clock, Console, Env, HasClock, HasConsole, HasEnv, HasRandom, Provides, Random, Rng, TestPlatform, resource,
        wraps,
    };
    #[cfg(feature = "store")]
    pub use crate::{HasStore, Store};
}

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use platform::JsPlatform;
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
pub use platform::StdPlatform;
pub use platform::TestPlatform;
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub use platform::TokioPlatform;
