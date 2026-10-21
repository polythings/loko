//! Platforms: one of each service, ready to pass. A platform is each service it holds; a program
//! that needs more (a `Store`, its own services) puts a platform in a `#[loko::bundle]` struct.

use crate::clock::{Clock, TestClock};
use crate::console::{Console, TestConsole};
use crate::env::{Env, MapEnv};
use crate::random::{Random, SeededRandom};
use crate::{HasClock, HasConsole, HasEnv, HasRandom, bundle};

/// Services for tests: a [`TestClock`] at the Unix epoch, a [`SeededRandom`] with seed 0, an empty
/// [`MapEnv`], and a [`TestConsole`] that records lines. Clones share the clock, the generator, and
/// the console; the fields are public so a test can set them up and look at them.
#[bundle]
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct TestPlatform {
    /// The time, moved by the test.
    #[has(Clock)]
    pub clock: TestClock,
    /// Random numbers, the same on every run.
    #[has(Random)]
    pub random: SeededRandom,
    /// Environment variables set by the test.
    #[has(Env)]
    pub env: MapEnv,
    /// The lines printed.
    #[has(Console)]
    pub console: TestConsole,
}

impl TestPlatform {
    /// The default test platform.
    pub fn new() -> Self {
        Self::default()
    }

    /// The clock set to `ms` milliseconds after the Unix epoch.
    #[must_use]
    pub fn at_ms(mut self, ms: i64) -> Self {
        self.clock = TestClock::at_ms(ms);
        self
    }

    /// The generator seeded with `seed`.
    #[must_use]
    pub fn seeded(mut self, seed: u64) -> Self {
        self.random = SeededRandom::new(seed);
        self
    }

    /// Environment variable `name` set to `value`.
    #[must_use]
    pub fn with_var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env = self.env.with(name, value);
        self
    }
}

/// The system's services for programs without an async runtime (feature `getrandom`, native
/// targets and WASI): [`StdClock`](crate::clock::StdClock), whose `sleep` blocks the thread,
/// [`OsRandom`](crate::random::OsRandom), [`SystemEnv`](crate::env::SystemEnv), and
/// [`StdConsole`](crate::console::StdConsole).
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
#[bundle]
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct StdPlatform {
    /// The system clock.
    #[has(Clock)]
    pub clock: crate::clock::StdClock,
    /// The operating system's generator.
    #[has(Random)]
    pub random: crate::random::OsRandom,
    /// The process's environment variables.
    #[has(Env)]
    pub env: crate::env::SystemEnv,
    /// Standard output and standard error.
    #[has(Console)]
    pub console: crate::console::StdConsole,
}

/// The system's services under tokio (feature `tokio`, native targets): [`StdPlatform`] with
/// [`TokioClock`](crate::clock::TokioClock), whose `sleep` is tokio's timer.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
#[bundle]
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct TokioPlatform {
    /// The system clock and tokio's timer.
    #[has(Clock)]
    pub clock: crate::clock::TokioClock,
    /// The operating system's generator.
    #[has(Random)]
    pub random: crate::random::OsRandom,
    /// The process's environment variables.
    #[has(Env)]
    pub env: crate::env::SystemEnv,
    /// Standard output and standard error.
    #[has(Console)]
    pub console: crate::console::StdConsole,
}

/// The JavaScript host's services (feature `js`, `wasm32-unknown-unknown`): [`JsClock`](crate::clock::JsClock),
/// [`JsRandom`](crate::random::JsRandom), a [`MapEnv`] (JavaScript hosts have no common environment;
/// set the variables the program reads), and [`JsConsole`](crate::console::JsConsole).
#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
#[bundle]
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct JsPlatform {
    /// `Date.now` and `setTimeout`.
    #[has(Clock)]
    pub clock: crate::clock::JsClock,
    /// `crypto.getRandomValues`.
    #[has(Random)]
    pub random: crate::random::JsRandom,
    /// Variables set by the program.
    #[has(Env)]
    pub env: MapEnv,
    /// `console.log` and `console.error`.
    #[has(Console)]
    pub console: crate::console::JsConsole,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Rng;

    #[test]
    fn a_test_platform_is_each_service() {
        let platform = TestPlatform::new().at_ms(1_000).seeded(0).with_var("REGION", "us-east-2");
        platform.clock.advance(Duration::from_millis(500));
        assert_eq!(platform.now_ms(), 1_500);
        assert_eq!(platform.rng().next_u64(), 0xA706_DD2F_4D19_7E6F);
        assert_eq!(platform.var("REGION").as_deref(), Some("us-east-2"));
        platform.print_line("hello").unwrap();
        assert_eq!(platform.console.lines(), ["hello"]);
    }

    #[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
    #[test]
    fn the_std_platform_reads_the_system() {
        let platform = StdPlatform::default();
        assert!(platform.now_ms() > 1_700_000_000_000);
        assert_ne!(platform.rng().next_u64(), platform.rng().next_u64());
        assert_eq!(platform.var("LOKO_SURELY_UNSET"), None);
    }
}
