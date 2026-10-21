use std::future::Future;
use std::time::{Duration, SystemTime};

use crate::Clock;

/// The system clock and tokio's timer (feature `tokio`, native targets).
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioClock;

impl Clock for TokioClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        tokio::time::sleep(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_tokio_clock_tells_time_and_waits() {
        let before = TokioClock.now_ms();
        assert!(before > 1_700_000_000_000, "after November 2023");
        TokioClock.sleep(Duration::from_millis(5)).await;
        assert!(TokioClock.now_ms() >= before + 5);
    }
}
