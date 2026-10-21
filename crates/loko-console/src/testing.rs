use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::Console;

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
}
