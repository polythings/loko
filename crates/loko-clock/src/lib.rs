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

use std::future::{Future, ready};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// A clock that moves only when told, or when slept on (sleeping returns at once, the clock moved
/// forward by the sleep). Clones share the same time. The idea is ZIO's `TestClock`.
#[derive(Clone, Debug)]
pub struct TestClock {
    now: Arc<Mutex<SystemTime>>,
}

impl TestClock {
    /// A clock at `now`.
    pub fn new(now: SystemTime) -> Self {
        Self { now: Arc::new(Mutex::new(now)) }
    }

    /// A clock `ms` milliseconds after the Unix epoch.
    pub fn at_ms(ms: i64) -> Self {
        Self::new(from_ms(ms))
    }

    /// Moves the clock forward by `duration`.
    pub fn advance(&self, duration: Duration) {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner) += duration;
    }

    /// Sets the clock to `now`, forward or back.
    pub fn set(&self, now: SystemTime) {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner) = now;
    }
}

impl Default for TestClock {
    /// A clock at the Unix epoch.
    fn default() -> Self {
        Self::new(UNIX_EPOCH)
    }
}

impl Clock for TestClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        self.advance(duration);
        ready(())
    }
}

/// The system clock for programs without an async runtime (native targets and WASI). Its `sleep` blocks the
/// calling thread before the future is returned, so in async code use `TokioClock` (or another
/// runtime's clock) instead.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct StdClock;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl Clock for StdClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        std::thread::sleep(duration);
        ready(())
    }
}

/// The system clock and tokio's timer (feature `tokio`, native targets).
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioClock;

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl Clock for TokioClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        tokio::time::sleep(duration)
    }
}

/// `Date.now` and `setTimeout` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`): browsers, Workers,
/// and Node.
#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct JsClock;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
impl Clock for JsClock {
    fn now(&self) -> SystemTime {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "milliseconds since the epoch")]
        let ms = js_sys::Date::now() as u64;
        UNIX_EPOCH + Duration::from_millis(ms)
    }

    async fn sleep(&self, duration: Duration) {
        use js_sys::{Function, Promise, Reflect};
        use wasm_bindgen::{JsCast, JsValue};

        #[expect(clippy::cast_precision_loss, reason = "timeouts are far below 2^53 ms")]
        let ms = JsValue::from_f64(duration.as_millis() as f64);
        let promise = Promise::new(&mut |resolve, _| {
            let global = js_sys::global();
            match Reflect::get(&global, &"setTimeout".into()).and_then(JsCast::dyn_into::<Function>) {
                Ok(set_timeout) => {
                    let _ = set_timeout.call2(&global, &resolve, &ms);
                }
                Err(_) => {
                    let _ = resolve.call0(&JsValue::NULL);
                }
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clocks_move_when_told() {
        let clock = TestClock::at_ms(5_000);
        let other = clock.clone();
        other.advance(Duration::from_millis(250));
        assert_eq!(clock.now_ms(), 5_250, "clones share the time");
        clock.set(from_ms(-1_500));
        assert_eq!(clock.now_ms(), -1_500);
        assert_eq!(TestClock::default().now(), UNIX_EPOCH);
    }

    #[test]
    fn sleeping_moves_a_test_clock() {
        let clock = TestClock::at_ms(0);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let mut sleep = std::pin::pin!(clock.sleep(Duration::from_secs(3)));
        assert!(sleep.as_mut().poll(&mut cx).is_ready());
        assert_eq!(clock.now_ms(), 3_000);
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[test]
    fn the_std_clock_tells_time_and_blocks() {
        let before = StdClock.now_ms();
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let mut sleep = std::pin::pin!(StdClock.sleep(Duration::from_millis(5)));
        assert!(sleep.as_mut().poll(&mut cx).is_ready());
        assert!(StdClock.now_ms() >= before + 5);
    }

    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn the_tokio_clock_tells_time_and_waits() {
        let before = TokioClock.now_ms();
        assert!(before > 1_700_000_000_000, "after November 2023");
        TokioClock.sleep(Duration::from_millis(5)).await;
        assert!(TokioClock.now_ms() >= before + 5);
    }
}
