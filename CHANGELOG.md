# Changelog

All notable changes to loko. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
versions follow [Semantic Versioning](https://semver.org/).

## Unreleased

The first release of loko as a services library. It is version 0.2.0 because the `loko` name on crates.io carries an
unrelated 0.1.0 from 2022 (task scripting), by the same owner.

### Added

- `loko-clock`: the `Clock` service (`now`, `now_ms`, `sleep`), `TestClock`, `StdClock`, `TokioClock` (feature
  `tokio`), and `JsClock` (feature `js`).
- `loko-random`: the `Random` service (`next_u64`, `fill`), `SeededRandom` (SplitMix64), `OsRandom` (feature
  `getrandom`), `JsRandom` (feature `js`), and `Random` for a `Mutex` around a `rand_core` generator (feature `rand`).
- `loko-env`: the `Env` service (`var`, `vars`), `SystemEnv`, and `MapEnv`.
- `loko-console`: the `Console` service (`print_line`, `print_error_line`), `StdConsole`, `TestConsole`, and
  `JsConsole` (feature `js`).
- `loko-store`: the `Store` service (objects by URI, with put-if-absent, multipart uploads, and prefix listing),
  `MemoryStore`, and an error type. It started as lakekit's `Store`.
- `loko-object-store`: a `Store` over an `object_store` client, from a client or a URL. It started as
  `lakekit-object-store`.
- `loko-macros`: `#[service]`, which adds a `HasX` trait to a service trait `X` and makes every `HasX` an `X`, and
  `#[derive(Services)]`, which makes a struct each service its fields provide.
- `loko`: the services under one crate (`loko::clock`, …, `loko::store` with feature `store`), the platforms
  `TestPlatform`, `StdPlatform`, `TokioPlatform`, and `JsPlatform`, and a prelude.
