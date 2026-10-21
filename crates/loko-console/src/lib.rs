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

use std::io;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
mod js;
mod system;
mod testing;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use js::JsConsole;
pub use system::StdConsole;
pub use testing::TestConsole;

/// Lines of text to standard output and standard error.
#[loko_macros::service]
pub trait Console {
    /// Writes `line` and a newline to standard output.
    fn print_line(&self, line: &str) -> io::Result<()>;

    /// Writes `line` and a newline to standard error.
    fn print_error_line(&self, line: &str) -> io::Result<()>;
}
