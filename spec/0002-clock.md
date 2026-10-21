---
number: 2
title: Clock
status: done
updated: 2026-10-20
versions: [unreleased]
related: [1, 7]
issues: []
---

# 2. Clock

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

The time and a timer. Code that stamps records, measures, retries, or waits takes a `Clock`, so tests set the time
and skip the waits.

## Design

- **`Clock`:** `now() -> SystemTime`, `now_ms() -> i64` (provided: milliseconds since the Unix epoch, negative before
  it, saturating), and `sleep(Duration) -> impl Future<Output = ()>`.
- **`from_ms(i64) -> SystemTime`:** the inverse of `now_ms`.
- **`TestClock`:** starts where told (`new`, `at_ms`; the Unix epoch by default), moves on `advance` and `set`, and
  on `sleep`, which moves it forward by the sleep and returns at once. Clones share the time.
- **`StdClock`:** `SystemTime::now`, and a `sleep` that blocks the thread before returning a ready future: for
  programs without an async runtime (native and WASI).
- **`TokioClock`** (feature `tokio`, native): `SystemTime::now` and `tokio::time::sleep`.
- **`JsClock`** (feature `js`, `wasm32-unknown-unknown`): `Date.now` and a promise around `setTimeout` (resolved at
  once on a host without `setTimeout`).

## Decisions

| ID | Decision | Status |
|---|---|---|
| D6 | `now` returns `SystemTime` and `sleep` takes `Duration`, the standard library's types; `now_ms` is provided for the common case. | Accepted |
| D7 | `TestClock`'s `sleep` moves the clock and returns at once, so code that waits runs in tests without waiting. | Accepted |
| D8 | `StdClock` blocks in `sleep`, documented, so programs without a runtime have a system clock; async programs use their runtime's clock. | Accepted |

## Open questions and issues

None.

## History

- 2026-10-20: built for 0.2.
