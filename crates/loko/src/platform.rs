//! Platforms: one of each resource, ready to pass. A platform is each resource it holds; a program
//! that needs more (a `Store`, its own resources) puts a platform in a `#[derive(Provides)]` struct.

use crate::clock::{Clock, TestClock};
use crate::console::{Console, TestConsole};
use crate::env::{Env, MapEnv};
use crate::random::{Random, SeededRandom};
use crate::{HasClock, HasConsole, HasEnv, HasRandom, Provides};

/// Resources for tests: a [`TestClock`] at the Unix epoch, a [`SeededRandom`] with seed 0, an empty
/// [`MapEnv`], and a [`TestConsole`] that records lines. Clones share the clock, the generator, and
/// the console; the fields are public so a test can set them up and look at them.
#[derive(Clone, Debug, Default, Provides)]
#[non_exhaustive]
pub struct TestPlatform {
    /// The time, moved by the test.
    #[provides(Clock)]
    pub clock: TestClock,
    /// Random numbers, the same on every run.
    #[provides(Random)]
    pub random: SeededRandom,
    /// Environment variables set by the test.
    #[provides(Env)]
    pub env: MapEnv,
    /// The lines printed.
    #[provides(Console)]
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

/// The system's resources for programs without an async runtime (feature `getrandom`, native
/// targets and WASI): [`StdClock`](crate::clock::StdClock), whose `sleep` blocks the thread,
/// [`OsRandom`](crate::random::OsRandom), [`SystemEnv`](crate::env::SystemEnv), and
/// [`StdConsole`](crate::console::StdConsole).
#[cfg(all(feature = "getrandom", not(all(target_arch = "wasm32", target_os = "unknown"))))]
#[derive(Clone, Copy, Debug, Default, Provides)]
#[non_exhaustive]
pub struct StdPlatform {
    /// The system clock.
    #[provides(Clock)]
    pub clock: crate::clock::StdClock,
    /// The operating system's generator.
    #[provides(Random)]
    pub random: crate::random::OsRandom,
    /// The process's environment variables.
    #[provides(Env)]
    pub env: crate::env::SystemEnv,
    /// Standard output and standard error.
    #[provides(Console)]
    pub console: crate::console::StdConsole,
}

/// The system's resources under tokio (feature `tokio`, native targets): [`StdPlatform`] with
/// [`TokioClock`](crate::clock::TokioClock), whose `sleep` is tokio's timer.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
#[derive(Clone, Copy, Debug, Default, Provides)]
#[non_exhaustive]
pub struct TokioPlatform {
    /// The system clock and tokio's timer.
    #[provides(Clock)]
    pub clock: crate::clock::TokioClock,
    /// The operating system's generator.
    #[provides(Random)]
    pub random: crate::random::OsRandom,
    /// The process's environment variables.
    #[provides(Env)]
    pub env: crate::env::SystemEnv,
    /// Standard output and standard error.
    #[provides(Console)]
    pub console: crate::console::StdConsole,
}

/// The JavaScript host's resources (feature `js`, `wasm32-unknown-unknown`): [`JsClock`](crate::clock::JsClock),
/// [`JsRandom`](crate::random::JsRandom), a [`MapEnv`] (JavaScript hosts have no common environment;
/// set the variables the program reads), and [`JsConsole`](crate::console::JsConsole).
#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Debug, Default, Provides)]
#[non_exhaustive]
pub struct JsPlatform {
    /// `Date.now` and `setTimeout`.
    #[provides(Clock)]
    pub clock: crate::clock::JsClock,
    /// `crypto.getRandomValues`.
    #[provides(Random)]
    pub random: crate::random::JsRandom,
    /// Variables set by the program.
    #[provides(Env)]
    pub env: MapEnv,
    /// `console.log` and `console.error`.
    #[provides(Console)]
    pub console: crate::console::JsConsole,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Rng;

    #[test]
    fn a_test_platform_is_each_resource() {
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
