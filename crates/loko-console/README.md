# loko-console

[loko](https://github.com/polythings/loko)'s `Console` service: lines to standard output and standard error, passed to
code instead of printed directly, so tests read what was printed and code in a browser logs to its console.

- `Console`: `print_line` and `print_error_line`.
- `StdConsole`: the process's standard output and error.
- `TestConsole`: keeps the lines (`lines`, `error_lines`). Clones share them.
- `JsConsole` (feature `js`, `wasm32-unknown-unknown`): `console.log` and `console.error`.

```rust
use loko_console::{Console, TestConsole};

let console = TestConsole::new();
console.print_line("done")?;
assert_eq!(console.lines(), ["done"]);
# Ok::<(), std::io::Error>(())
```

Apache License 2.0.
