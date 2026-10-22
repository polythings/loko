# Changelog

All notable changes to loko. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
versions follow [Semantic Versioning](https://semver.org/).

## Unreleased

## 0.2.0 (2026-10-22)

The first release of loko as a services library. It is version 0.2.0 because the `loko` name on crates.io carries an
unrelated 0.1.0 from 2022 (task scripting), by the same owner.

### Added

- `loko-clock`: the `Clock` service (`now`, `now_ms`, `sleep`), `TestClock`, `StdClock`, `TokioClock` (feature
  `tokio`), `JsClock` (feature `js`), and `OffsetClock`, a clock ahead or behind another.
- `loko-random`: the `Random` service, a source of `rand_core` generators (`rng()`, drawn from with no locks);
  `SeededRandom` (`SeededRng`, SplitMix64, for tests), `OsRandom` (feature `getrandom`, `OsRng`), `JsRandom` (feature
  `js`, `JsRng`), and `Random` for a `Mutex` around any seedable `rand_core` generator. The OS and JavaScript generators
  are `CryptoRng`, so code that makes secrets can ask for `Random<Rng: CryptoRng>`.
- `loko-env`: the `Env` service (`var`, `vars`), `SystemEnv`, and `MapEnv`.
- `loko-console`: the `Console` service (`print_line`, `print_error_line`), `StdConsole`, `TestConsole`, and
  `JsConsole` (feature `js`).
- `loko-store`: the `Store` service (objects by URI, with put-if-absent, multipart uploads, and prefix listing),
  `MemoryStore`, an error type, and the wrappers `CountingStore` (counts calls), `ReadOnlyStore` (refuses writes), and
  `PrefixedStore` (passes on only URIs under a prefix).
- `loko-object-store`: a `Store` over an `object_store` client, from a client or a URL.
- The service crates depend on nothing but `rand_core` (for `loko-random`); the macros are needed only to define
  new services.
- `loko-macros`: `#[service]`, which adds a `HasX` trait to a service trait `X` and makes every `HasX` an `X`, and
  `#[loko::bundle]`, which makes a struct each service its fields provide, and `#[wraps]`, which makes a wrapper
  that replaces some of a service's methods and forwards the rest.
- `loko`: the services under one crate (`loko::clock`, …, `loko::store` with feature `store`), the platforms
  `TestPlatform`, `StdPlatform`, `TokioPlatform`, and `JsPlatform`, and a prelude.
