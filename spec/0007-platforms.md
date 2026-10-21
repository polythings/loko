---
number: 7
title: Platforms
status: done
updated: 2026-10-20
versions: [unreleased]
related: [1, 2, 3, 4, 5]
issues: [3]
---

# 7. Platforms

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

A platform is one of each of `Clock`, `Random`, `Env`, and `Console`, ready to pass: the test services, the
system's, the system's under tokio, or a JavaScript host's. A program that needs more puts a platform in its own
`#[derive(Services)]` bundle.

## Design

| Platform | Clock | Random | Env | Console | Targets |
|---|---|---|---|---|---|
| `TestPlatform` | `TestClock` | `SeededRandom` | `MapEnv` | `TestConsole` | All |
| `StdPlatform` | `StdClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native and WASI (feature `getrandom`, on by default) |
| `TokioPlatform` | `TokioClock` | `OsRandom` | `SystemEnv` | `StdConsole` | Native (feature `tokio`) |
| `JsPlatform` | `JsClock` | `JsRandom` | `MapEnv` | `JsConsole` | `wasm32-unknown-unknown` (feature `js`) |

- Each platform is a struct with public fields, one per service, built with `#[derive(Services)]`, so it is each
  service and a test can set up and inspect the parts (`platform.clock.advance(…)`, `platform.console.lines()`).
- The structs are `non_exhaustive`, so a service added later isn't a breaking change; they are built with `default()`
  (and `TestPlatform`'s `at_ms`, `seeded`, and `with_var`).
- `Store` is not in a platform: which store a program uses is its own choice.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D14 | Platforms bundle `Clock`, `Random`, `Env`, and `Console`, without `Store`, and are named for where they run (`TestPlatform`, `StdPlatform`, `TokioPlatform`, `JsPlatform`). | Accepted |
| D15 | JavaScript implementations are for `wasm32-unknown-unknown` only; system implementations, and `StdPlatform`, also build for WASI. | Accepted |

## Open questions and issues

- [#3](https://github.com/polythings/loko/issues/3) Environment variables in JavaScript hosts

## History

- 2026-10-20: built for 0.2.
