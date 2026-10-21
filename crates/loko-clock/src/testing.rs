use std::future::{Future, ready};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{Clock, from_ms};

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
    ///
    /// # Panics
    ///
    /// If the platform can't hold that time ([`from_ms`]): before the epoch on WebAssembly (WASI and
    /// `wasm32-unknown-unknown`).
    pub fn at_ms(ms: i64) -> Self {
        Self::new(from_ms(ms).expect("a time this platform can hold"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clocks_move_when_told() {
        let clock = TestClock::at_ms(5_000);
        let other = clock.clone();
        other.advance(Duration::from_millis(250));
        assert_eq!(clock.now_ms(), 5_250, "clones share the time");
        // Times before the epoch exist natively, not on WebAssembly.
        if let Some(before) = from_ms(-1_500) {
            clock.set(before);
            assert_eq!(clock.now_ms(), -1_500);
        }
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
}
