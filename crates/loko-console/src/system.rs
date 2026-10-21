use std::io::{self, Write};

use crate::Console;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_std_console_writes() {
        StdConsole.print_line("loko-console test line").unwrap();
    }
}
