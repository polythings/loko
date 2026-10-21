//! loko's `Clock` service: the time and a timer, passed to code instead of read from the system, so
//! tests control time and the same code runs natively and in WebAssembly.
//!
//! - [`Clock`]: `now`, `now_ms`, and `sleep`.
//! - [`TestClock`]: a clock that moves only when told (or when slept on), for tests.
//! - `StdClock`: the system clock with a blocking sleep, for programs without an async runtime.
//! - `TokioClock` (feature `tokio`): the system clock and tokio's timer, on native targets.
//! - `JsClock` (feature `js`): `Date.now` and `setTimeout`, in browsers, Workers, and Node.
//!
//! ```
//! use std::time::Duration;
//!
//! use loko_clock::{Clock, TestClock};
//!
//! let clock = TestClock::at_ms(1_000);
//! clock.advance(Duration::from_secs(1));
//! assert_eq!(clock.now_ms(), 2_000);
//! ```

use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
mod js;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod system;
mod testing;
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
mod tokio_clock;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use js::JsClock;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use system::StdClock;
pub use testing::TestClock;
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub use tokio_clock::TokioClock;

/// The time and a timer.
///
/// The future isn't required to be `Send`, so a clock can wrap JavaScript timers.
#[loko_macros::service]
pub trait Clock {
    /// The current time.
    fn now(&self) -> SystemTime;

    /// Milliseconds since the Unix epoch (negative before it).
    fn now_ms(&self) -> i64 {
        match self.now().duration_since(UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_millis()).unwrap_or(i64::MAX),
            Err(before) => i64::try_from(before.duration().as_millis()).map_or(i64::MIN, |ms| -ms),
        }
    }

    /// Waits `duration`.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()>;
}

/// The time `ms` milliseconds after the Unix epoch (before it, if negative).
pub fn from_ms(ms: i64) -> SystemTime {
    let magnitude = Duration::from_millis(ms.unsigned_abs());
    if ms >= 0 { UNIX_EPOCH + magnitude } else { UNIX_EPOCH - magnitude }
}
