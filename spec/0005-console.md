---
number: 5
title: Console
status: done
updated: 2026-10-20
versions: [unreleased]
related: [1, 7]
issues: []
---

# 5. Console

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

Lines to standard output and standard error. Code that reports progress or results takes a `Console`, so tests
read what it printed and code in a browser logs to the developer console.

## Design

- **`Console`:** `print_line(&str) -> io::Result<()>` and `print_error_line(&str) -> io::Result<()>`; each writes the
  line and a newline.
- **`StdConsole`:** the process's standard output and error, locked for the line.
- **`TestConsole`:** keeps the lines (`lines`, `error_lines`). Clones share them.
- **`JsConsole`** (feature `js`, `wasm32-unknown-unknown`): `console.log` and `console.error`.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D12 | Lines rather than a byte stream: what programs print to a console, and what JavaScript consoles take. | Accepted |

## Open questions and issues

None.

## History

- 2026-10-20: built for 0.2.
