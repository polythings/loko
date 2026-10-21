//! loko's `Console` resource: lines to standard output and standard error, passed to code instead of
//! printed directly, so tests read what was printed and code in a browser logs to its console.
//!
//! - [`Console`]: `print_line` and `print_error_line`.
//! - `StdConsole`: the process's standard output and error (native targets and WASI).
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
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod system;
mod testing;

#[cfg(all(feature = "js", all(target_arch = "wasm32", target_os = "unknown")))]
pub use js::JsConsole;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use system::StdConsole;
pub use testing::TestConsole;

/// Lines of text to standard output and standard error.
pub trait Console {
    /// Writes `line` and a newline to standard output.
    fn print_line(&self, line: &str) -> io::Result<()>;

    /// Writes `line` and a newline to standard error.
    fn print_error_line(&self, line: &str) -> io::Result<()>;
}

/// A type that holds a [`Console`], and so is one: `#[loko::bundle]` implements this for a field,
/// and `#[loko::wraps]` for a wrapper that changes some methods. It's what `#[loko::resource]`
/// generates, written out so this crate needs no macros.
pub trait HasConsole {
    /// The resource held.
    type Console: Console;

    /// The [`Console`] this holds.
    fn console(&self) -> &Self::Console;

    /// Forwards [`Console::print_line`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_print_line(&self, line: &str) -> io::Result<()> {
        self.console().print_line(line)
    }

    /// Forwards [`Console::print_error_line`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_print_error_line(&self, line: &str) -> io::Result<()> {
        self.console().print_error_line(line)
    }
}

impl<H: HasConsole> Console for H {
    #[inline]
    fn print_line(&self, line: &str) -> io::Result<()> {
        self.wrapped_print_line(line)
    }

    #[inline]
    fn print_error_line(&self, line: &str) -> io::Result<()> {
        self.wrapped_print_error_line(line)
    }
}
