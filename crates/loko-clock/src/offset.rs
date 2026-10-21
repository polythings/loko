use std::time::{Duration, SystemTime};

use crate::{Clock, HasClock, to_ms};

/// Another clock, ahead or behind by a fixed amount: what a machine whose clock is off reads, for
/// testing code against clock skew. Sleeping is the other clock's.
#[derive(Clone, Debug, Default)]
pub struct OffsetClock<C> {
    inner: C,
    offset: Duration,
    behind: bool,
}

impl<C> OffsetClock<C> {
    /// `inner`, ahead by `offset`.
    pub fn ahead(inner: C, offset: Duration) -> Self {
        Self { inner, offset, behind: false }
    }

    /// `inner`, behind by `offset`.
    pub fn behind(inner: C, offset: Duration) -> Self {
        Self { inner, offset, behind: true }
    }

    /// The clock offset.
    pub fn inner(&self) -> &C {
        &self.inner
    }
}

#[loko_macros::wraps(inner: C)]
impl<C: Clock> Clock for OffsetClock<C> {
    fn now(&self) -> SystemTime {
        let now = self.inner.now();
        let moved = if self.behind { now.checked_sub(self.offset) } else { now.checked_add(self.offset) };
        // Past what `SystemTime` holds: stay at the other clock's time rather than panic.
        moved.unwrap_or(now)
    }

    // The other clock's `now_ms` would leave out the offset.
    fn now_ms(&self) -> i64 {
        to_ms(Clock::now(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestClock;

    #[test]
    fn offset_clocks_follow_the_other_clock() {
        let clock = TestClock::at_ms(10_000);
        let ahead = OffsetClock::ahead(clock.clone(), Duration::from_secs(2));
        let behind = OffsetClock::behind(clock.clone(), Duration::from_secs(3));
        assert_eq!((ahead.now_ms(), behind.now_ms()), (12_000, 7_000));
        clock.advance(Duration::from_secs(1));
        assert_eq!((ahead.now_ms(), behind.now_ms()), (13_000, 8_000));

        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::pin::pin!(ahead.sleep(Duration::from_secs(5))).as_mut().poll(&mut cx).is_ready());
        assert_eq!(clock.now_ms(), 16_000, "sleeping is the other clock's");
    }
}
