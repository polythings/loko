---
number: 3
title: Random
status: done
updated: 2026-10-20
versions: [unreleased]
related: [1, 7]
issues: []
---

# 3. Random

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

Random numbers. Code that makes identifiers, jitter, or samples takes a `Random`, so tests get the same numbers on
every run.

## Design

- **`Random`:** `next_u64() -> u64` and `fill(&mut [u8])` (provided, from `next_u64`; implementations backed by a
  byte source override it).
- **`SeededRandom`:** SplitMix64 from a seed (0 by default). Not for secrets. Clones share the sequence.
- **`OsRandom`** (feature `getrandom`, native and WASI): the operating system's generator.
- **`JsRandom`** (feature `js`, `wasm32-unknown-unknown`): `crypto.getRandomValues` in chunks of at most 65,536
  bytes, falling back to `Math.random` on a host without Web Crypto.
- **Feature `rand`:** `Mutex<R>` is a `Random` for any `rand_core::Rng`, so a program's existing generator works as a
  resource.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D9 | `Random` is two methods on `&self`; generators with `&mut self` state go behind a `Mutex`, which the `rand` feature implements. | Accepted |
| D10 | `SeededRandom` is SplitMix64: small, fast, statistically good enough for tests, and the same on every platform. | Accepted |

## Open questions and issues

None.

## History

- 2026-10-20: built for 0.2.
