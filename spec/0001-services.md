---
number: 1
title: Services, bundles, and wrappers
status: done
updated: 2026-10-21
versions: [0.2.0]
related: [2, 3, 4, 5, 6, 7]
issues: [1, 2, 4, 7]
---

# 1. Services, bundles, and wrappers

**Done** · updated 2026-10-21 · versions 0.2.0 · milestone: [0.2](milestones/v0-2.md)

## Summary

A service is a trait for something code would otherwise reach for as a global: the time, random numbers,
environment variables, the console, object storage. Code takes the services it needs as `&impl Trait`, the program
passes the system's, and tests pass ones they control. `#[service]`, `#[loko::bundle]`, and `#[wraps]` let
anyone define new services, bundle them with loko's, and wrap one implementation in another.

## Design

- **A service** is a trait marked `#[loko_macros::service]` (`#[loko::service]` through the `loko` crate). Its
  methods take `&self`; async ones return `impl Future` with no `Send` bound, so an implementation can wrap JavaScript
  promises. The trait can't be generic; it can have associated types, constants, and provided methods.
- **`#[service]` on `trait X`** adds:
  - `trait HasX { type X: X; fn x(&self) -> &Self::X; … }` (the accessor is the trait's name in snake case), with
    a hidden method `wrapped_m` for each method `m` of `X` that forwards to the held service by default, and
  - `impl<T: HasX> X for T`, whose methods call the `wrapped_` ones, and whose associated types and constants are the
    held service's. Provided methods forward too, so an implementation's overrides are kept.
- **`#[loko::bundle]`** on a struct implements `HasX` for each `#[loko::has(X, …)]` on a field, with the field's
  type as the held service (and `FieldType: X` as a bound, so generic fields work). Named and tuple fields both work.
  A field can provide several services, such as a platform providing `Clock, Random`.
- **Paths:** `#[loko::bundle]` names `HasX` next to the path given: `#[loko::has(loko::Clock)]` uses `loko::HasClock`, and a
  bare `#[loko::has(Clock)]` needs `HasClock` in scope. `loko::prelude` brings in each service with its `HasX`.
- **`#[wraps(field: Type)]`** on `impl X for Wrapper` implements `HasX` instead, with `field` (a name or a tuple
  index) of type `Type` as the held service. The methods written become the `wrapped_` methods, replacing the
  forwarding ones; `Self::Assoc` in their signatures means the held service's. A wrapper keeps the held service's
  associated types and constants, and a provided method it doesn't write forwards to the held one, so a wrapper writes
  the provided methods that depend on what it changes (`OffsetClock` writes `now_ms` as well as `now`). Wrappers
  stack: `CountingStore<ReadOnlyStore<S>>`.
- **`&self` and state:** every method takes `&self`, so one value can be several services (`f(r, r)`), clones can share
  state, and services can be held across `.await`. An implementation whose state changes keeps it behind a lock or an
  atomic, locked only for the synchronous part of a call; a lock or `RefCell` borrow held across `.await` deadlocks or
  panics when another task uses the service meanwhile.
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
| D3 | `#[service]` adds `HasX` and a blanket `impl<T: HasX> X for T`; `#[loko::bundle]` implements `HasX` per field attribute. A service implemented by hand and one held in a bundle look the same to callers. | Accepted |
| D4 | `#[loko::bundle]` names `HasX` next to the path given, and `loko::prelude` brings each service in with its `HasX`, rather than generating macros named after the traits. | Accepted |
| D5 | The crate is `loko`, at 0.2.0: the name's owner already published an unrelated 0.1.0 in 2022. | Accepted |
| D16 | The things passed are called services (`#[service]`), the word ZIO and most dependency-injection libraries use (ferrous-di, dependency-injector, .NET, Angular). tower's `Service` is a different thing, a request handler; loko exports no `Service` trait, so the two don't meet in code. | Accepted |
| D17 | Bundles are declared with `#[loko::bundle]` on the struct and `#[loko::has(X, …)]` on fields: the struct-level name says what the struct is, the field-level one what each field holds. An attribute macro (not a derive) so the field attribute can be the path `loko::has` and the three macros look alike. | Accepted |
| D18 | Wrappers implement `HasX` through `#[wraps]`, overriding hidden `wrapped_` methods, so they write only what they change. The forwarding lives in `HasX`, whose signatures resolve where the trait is defined, rather than in generated macros that would resolve them at the wrapper. | Accepted |
| D19 | loko's own service crates write out what `#[service]` generates instead of using the macro, so they have no proc-macro dependency (and `loko-store` no `thiserror`): `loko-clock`, `loko-env`, and `loko-console` depend on nothing, `loko-random` only on `rand_core`. | Accepted |

## Open questions and issues

- [#1](https://github.com/polythings/loko/issues/1) Publish 0.2.0 to crates.io
- [#2](https://github.com/polythings/loko/issues/2) Send futures for multi-threaded executors
- [#4](https://github.com/polythings/loko/issues/4) Which services come next?
- [#7](https://github.com/polythings/loko/issues/7) Layers: stacking wrappers in reading order

## History

- 2026-10-20: built for 0.2, as services with `#[derive(Services)]`.
- 2026-10-21: `#[derive(Services)]` became `#[loko::bundle]` with `#[loko::has]` fields (D17); services were called
  resources for a day, and are services again (D16);
  `#[wraps]` added (D18).
- 2026-10-21: loko's own services written out by hand, without the macros (D19).
- 2026-10-22: released in 0.2.0.
