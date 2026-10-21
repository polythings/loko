---
number: 1
title: Services and the derive
status: done
updated: 2026-10-20
versions: [unreleased]
related: [2, 3, 4, 5, 6, 7]
issues: [1, 2, 4]
---

# 1. Services and the derive

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

A service is a trait for something code would otherwise reach for as a global: the time, random numbers,
environment variables, the console, object storage. Code takes the services it needs as `&impl Trait`, the program
passes the system's, and tests pass ones they control. `#[service]` and `#[derive(Services)]` let anyone define new
services and bundle them with loko's.

## Design

- **A service** is a trait marked `#[loko_macros::service]` (`#[loko::service]` through the `loko` crate). Its
  methods take `&self`; async ones return `impl Future` with no `Send` bound, so an implementation can wrap JavaScript
  promises. The trait can't be generic; it can have associated types, constants, and provided methods.
- **`#[service]` on `trait X`** adds:
  - `trait HasX { type X: X; fn x(&self) -> &Self::X; }` (the accessor is the trait's name in snake case), and
  - `impl<T: HasX> X for T`, forwarding every method (provided ones too, so an implementation's overrides are kept),
    associated type, and constant to the held service.
- **`#[derive(Services)]`** on a struct implements `HasX` for each `#[service(X, …)]` on a field, with the field's
  type as the held service (and `FieldType: X` as a bound, so generic fields work). Named and tuple fields both work.
  A field can provide several services, such as a platform providing `Clock, Random`.
- **Paths:** the derive names `HasX` next to the path given: `#[service(loko::Clock)]` uses `loko::HasClock`, and a
  bare `#[service(Clock)]` needs `HasClock` in scope. `loko::prelude` brings in each service with its `HasX`.
- **Composition:** an implementation of one service can hold others (a greeter that holds a clock), and a program's
  bundle can mix loko's services with its own. Code asks only for what it uses: `&(impl Store + Clock)`.
- **Crates:** one per service (`loko-clock`, `loko-random`, `loko-env`, `loko-console`, `loko-store`), each with its
  trait and implementations; `loko-object-store`; `loko-macros`; and `loko`, which re-exports them
  (`loko::clock`, …; the traits also at the root), adds the platforms ([spec 7](0007-platforms.md)), and has a feature
  per optional part. A library that needs one service depends on its crate alone.
- **Targets:** every crate builds for native targets, `wasm32-unknown-unknown`, and `wasm32-wasip2`. JavaScript
  implementations need feature `js` and `wasm32-unknown-unknown`; system ones are on every target with a system
  (native and WASI).

## Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | A crate per service, re-exported by `loko`, so a library takes only the service it needs. | Accepted |
| D2 | Futures aren't `Send`: services can wrap JavaScript promises; multi-threaded executors are an open question (#2). | Accepted |
| D3 | `#[service]` adds `HasX` and a blanket `impl<T: HasX> X for T`; the derive implements `HasX` per field attribute. A service implemented by hand and one held in a bundle look the same to callers. | Accepted |
| D4 | The derive names `HasX` next to the path given, and `loko::prelude` brings each service in with its `HasX`, rather than generating macros named after the traits. | Accepted |
| D5 | The crate is `loko`, at 0.2.0: the name's owner already published an unrelated 0.1.0 in 2022. | Accepted |

## Open questions and issues

- [#1](https://github.com/polythings/loko/issues/1) Publish 0.2.0 to crates.io
- [#2](https://github.com/polythings/loko/issues/2) Send futures for multi-threaded executors
- [#4](https://github.com/polythings/loko/issues/4) Which services come next?

## History

- 2026-10-20: built for 0.2.
