---
number: 1
title: Resources, Provides, and wrappers
status: done
updated: 2026-10-21
versions: [unreleased]
related: [2, 3, 4, 5, 6, 7]
issues: [1, 2, 4, 7]
---

# 1. Resources, Provides, and wrappers

**Done** · updated 2026-10-21 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

A resource is a trait for something code would otherwise reach for as a global: the time, random numbers,
environment variables, the console, object storage. Code takes the resources it needs as `&impl Trait`, the program
passes the system's, and tests pass ones they control. `#[resource]`, `#[derive(Provides)]`, and `#[wraps]` let
anyone define new resources, bundle them with loko's, and wrap one implementation in another.

## Design

- **A resource** is a trait marked `#[loko_macros::resource]` (`#[loko::resource]` through the `loko` crate). Its
  methods take `&self`; async ones return `impl Future` with no `Send` bound, so an implementation can wrap JavaScript
  promises. The trait can't be generic; it can have associated types, constants, and provided methods.
- **`#[resource]` on `trait X`** adds:
  - `trait HasX { type X: X; fn x(&self) -> &Self::X; … }` (the accessor is the trait's name in snake case), with
    a hidden method `wrapped_m` for each method `m` of `X` that forwards to the held resource by default, and
  - `impl<T: HasX> X for T`, whose methods call the `wrapped_` ones, and whose associated types and constants are the
    held resource's. Provided methods forward too, so an implementation's overrides are kept.
- **`#[derive(Provides)]`** on a struct implements `HasX` for each `#[provides(X, …)]` on a field, with the field's
  type as the held resource (and `FieldType: X` as a bound, so generic fields work). Named and tuple fields both work.
  A field can provide several resources, such as a platform providing `Clock, Random`.
- **Paths:** the derive names `HasX` next to the path given: `#[provides(loko::Clock)]` uses `loko::HasClock`, and a
  bare `#[provides(Clock)]` needs `HasClock` in scope. `loko::prelude` brings in each resource with its `HasX`.
- **`#[wraps(field: Type)]`** on `impl X for Wrapper` implements `HasX` instead, with `field` (a name or a tuple
  index) of type `Type` as the held resource. The methods written become the `wrapped_` methods, replacing the
  forwarding ones; `Self::Assoc` in their signatures means the held resource's. A wrapper keeps the held resource's
  associated types and constants, and a provided method it doesn't write forwards to the held one, so a wrapper writes
  the provided methods that depend on what it changes (`OffsetClock` writes `now_ms` as well as `now`). Wrappers
  stack: `CountingStore<ReadOnlyStore<S>>`.
- **Composition:** an implementation of one resource can hold others (a greeter that holds a clock), and a program's
  bundle can mix loko's resources with its own. Code asks only for what it uses: `&(impl Store + Clock)`.
- **Crates:** one per resource (`loko-clock`, `loko-random`, `loko-env`, `loko-console`, `loko-store`), each with its
  trait and implementations; `loko-object-store`; `loko-macros`; and `loko`, which re-exports them
  (`loko::clock`, …; the traits also at the root), adds the platforms ([spec 7](0007-platforms.md)), and has a feature
  per optional part. A library that needs one resource depends on its crate alone.
- **Targets:** every crate builds for native targets, `wasm32-unknown-unknown`, and `wasm32-wasip2`. JavaScript
  implementations need feature `js` and `wasm32-unknown-unknown`; system ones are on every target with a system
  (native and WASI).

## Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | A crate per resource, re-exported by `loko`, so a library takes only the resource it needs. | Accepted |
| D2 | Futures aren't `Send`: resources can wrap JavaScript promises; multi-threaded executors are an open question (#2). | Accepted |
| D3 | `#[resource]` adds `HasX` and a blanket `impl<T: HasX> X for T`; the derive implements `HasX` per field attribute. A resource implemented by hand and one held in a bundle look the same to callers. | Accepted |
| D4 | The derive names `HasX` next to the path given, and `loko::prelude` brings each resource in with its `HasX`, rather than generating macros named after the traits. | Accepted |
| D5 | The crate is `loko`, at 0.2.0: the name's owner already published an unrelated 0.1.0 in 2022. | Accepted |
| D16 | The things passed are called resources (`#[resource]`), not services: "service" means a request handler in Rust (tower) and a server elsewhere, and a resource is what Bevy calls its shared `Time`. | Accepted |
| D17 | The derive is `Provides`, with `#[provides(X, …)]` on fields: a field provides resources. | Accepted |
| D18 | Wrappers implement `HasX` through `#[wraps]`, overriding hidden `wrapped_` methods, so they write only what they change. The forwarding lives in `HasX`, whose signatures resolve where the trait is defined, rather than in generated macros that would resolve them at the wrapper. | Accepted |

## Open questions and issues

- [#1](https://github.com/polythings/loko/issues/1) Publish 0.2.0 to crates.io
- [#2](https://github.com/polythings/loko/issues/2) Send futures for multi-threaded executors
- [#4](https://github.com/polythings/loko/issues/4) Which resources come next?
- [#7](https://github.com/polythings/loko/issues/7) Layers: stacking wrappers in reading order

## History

- 2026-10-20: built for 0.2, as services with `#[derive(Services)]`.
- 2026-10-21: services renamed resources (D16), the derive `Provides` (D17); `#[wraps]` added (D18).
