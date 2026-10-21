//! loko's `Console` service: lines to standard output and standard error, passed to code instead of
//! printed directly, so tests read what was printed and code in a browser logs to its console.
//!
//! - [`Console`]: `print_line` and `print_error_line`.
//! - [`StdConsole`]: the process's standard output and error.
//! - [`TestConsole`]: keeps the lines, for tests.
//! - `JsConsole` (feature `js`): `console.log` and `console.error`, in browsers, Workers, and Node.
//!
//! ```
//! use loko_console::{Console, TestConsole};
//!
//! let console = TestConsole::new();
//! console.print_line("done")?;
//! assert_eq!(console.lines(), ["done"]);
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Lines of text to standard output and standard error.
#[loko_macros::service]
pub trait Console {
    /// Writes `line` and a newline to standard output.
    fn print_line(&self, line: &str) -> io::Result<()>;

    /// Writes `line` and a newline to standard error.
    fn print_error_line(&self, line: &str) -> io::Result<()>;
}

/// The process's standard output and standard error.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdConsole;

impl Console for StdConsole {
    fn print_line(&self, line: &str) -> io::Result<()> {
        writeln!(io::stdout().lock(), "{line}")
    }

    fn print_error_line(&self, line: &str) -> io::Result<()> {
        writeln!(io::stderr().lock(), "{line}")
    }
}

/// Keeps the lines written, for tests. Clones share the lines.
#[derive(Clone, Debug, Default)]
pub struct TestConsole {
    out: Arc<Mutex<Vec<String>>>,
    err: Arc<Mutex<Vec<String>>>,
}

fn lock(lines: &Mutex<Vec<String>>) -> MutexGuard<'_, Vec<String>> {
    lines.lock().unwrap_or_else(PoisonError::into_inner)
}

impl TestConsole {
    /// No lines yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The lines written to standard output, in order.
    pub fn lines(&self) -> Vec<String> {
        lock(&self.out).clone()
    }

    /// The lines written to standard error, in order.
    pub fn error_lines(&self) -> Vec<String> {
        lock(&self.err).clone()
    }
}

impl Console for TestConsole {
    fn print_line(&self, line: &str) -> io::Result<()> {
        lock(&self.out).push(line.to_string());
        Ok(())
    }

    fn print_error_line(&self, line: &str) -> io::Result<()> {
        lock(&self.err).push(line.to_string());
        Ok(())
    }
}

/// `console.log` and `console.error` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`): browsers,
/// Workers, and Node.
#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct JsConsole;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
impl JsConsole {
    fn call(method: &str, line: &str) -> io::Result<()> {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::JsCast;

        let console = Reflect::get(&js_sys::global(), &"console".into())
            .map_err(|_| io::Error::other("the host has no console"))?;
        let f: Function = Reflect::get(&console, &method.into())
            .ok()
            .and_then(|f| f.dyn_into().ok())
            .ok_or_else(|| io::Error::other(format!("the console has no {method}")))?;
        f.call1(&console, &line.into()).map(drop).map_err(|e| io::Error::other(format!("console.{method}: {e:?}")))
    }
}

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
impl Console for JsConsole {
    fn print_line(&self, line: &str) -> io::Result<()> {
        Self::call("log", line)
    }

    fn print_error_line(&self, line: &str) -> io::Result<()> {
        Self::call("error", line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consoles_keep_lines() {
        let console = TestConsole::new();
        let shared = console.clone();
        console.print_line("a").unwrap();
        shared.print_error_line("oops").unwrap();
        console.print_line("b").unwrap();
        assert_eq!(console.lines(), ["a", "b"]);
        assert_eq!(console.error_lines(), ["oops"]);
    }

    #[test]
    fn the_std_console_writes() {
        StdConsole.print_line("loko-console test line").unwrap();
    }
}
