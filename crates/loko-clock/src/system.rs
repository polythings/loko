use std::future::{Future, ready};
use std::time::{Duration, SystemTime};

use crate::Clock;

/// The system clock for programs without an async runtime (native targets and WASI). Its `sleep`
/// blocks the calling thread before the future is returned, so in async code use `TokioClock` (or
/// another runtime's clock) instead.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdClock;

impl Clock for StdClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        std::thread::sleep(duration);
        ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_std_clock_tells_time_and_blocks() {
        let before = StdClock.now_ms();
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let mut sleep = std::pin::pin!(StdClock.sleep(Duration::from_millis(5)));
        assert!(sleep.as_mut().poll(&mut cx).is_ready());
        assert!(StdClock.now_ms() >= before + 5);
    }
}
