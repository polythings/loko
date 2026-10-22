---
number: 3
title: Random
status: done
updated: 2026-10-21
versions: [0.2.0]
related: [1, 7]
issues: []
---

# 3. Random

**Done** · updated 2026-10-21 · versions 0.2.0 · milestone: [0.2](milestones/v0-2.md)

## Summary

A source of random number generators. Code that makes identifiers, jitter, or samples takes a `Random` and asks it for
a generator of its own, so tests get the same numbers on every run and drawing many numbers takes no locks.

## Design

- **`Random`:** `type Rng: rand_core::Rng` and `rng(&self) -> Self::Rng`: a generator of the caller's own, drawn from
  with `&mut` and no locks, usable with the whole `rand` ecosystem. Each implementation chooses the generator, and so
  whether it's fit for secrets: code that makes secrets asks for `Random<Rng: CryptoRng>`.
- **`SeededRandom`:** one SplitMix64 sequence from a seed (0 by default), shared by clones; each `rng()` is a
  `SeededRng` (SplitMix64) seeded from the sequence's next number, so the same seed and the same calls give the same
  numbers on every platform. Not for secrets. `SeededRng` is also a `SeedableRng`.
- **`OsRandom`** (feature `getrandom`, native and WASI): `OsRng`, which reads the operating system's generator on
  every draw. A `CryptoRng`.
- **`JsRandom`** (feature `js`, `wasm32-unknown-unknown`): `JsRng`, which calls `crypto.getRandomValues` in chunks of
  at most 65,536 bytes. A `CryptoRng`; it panics on a host without Web Crypto rather than fall back to numbers that
  aren't.
- **`Mutex<R>`** is a `Random` for any `R: SeedableRng + Rng`, so a program's existing generator works as a service;
  `rng()` forks it (`SeedableRng::fork`).
- `rand_core`'s `Rng`, `CryptoRng`, and `SeedableRng` are re-exported, and `Rng` is in `loko::prelude`.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D9 | `Random` hands out generators (`rng()`) rather than numbers: one lock at most per generator instead of one per number, `&mut` generators as the `rand` ecosystem expects, and the generator's type says whether it's fit for secrets. | Accepted |
| D10 | `SeededRandom` is SplitMix64: small, fast, statistically good enough for tests, and the same on every platform. | Accepted |

## Open questions and issues

None.

## History

- 2026-10-20: built for 0.2.
- 2026-10-21: `Random` hands out generators (`rng()`, D9) instead of `next_u64` and `fill`; the `rand` feature is gone
  (`rand_core` is always a dependency); `JsRandom` no longer falls back to `Math.random`.
- 2026-10-22: released in 0.2.0.
