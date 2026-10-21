# How loko compares

loko does two things:

1. **It ships resources.** `Clock`, `Random`, `Env`, `Console`, and `Store` come with an implementation for tests, one
   for the system, and one for JavaScript hosts, plus platforms that bundle them and wrappers that change them.
2. **It has a small macro layer.** `#[resource]`, `#[loko::bundle]`, and `#[wraps]` let your own traits work the
   same way.

Each crate below overlaps with one of those. The comparisons were checked against these versions (October 2026):
delegate 0.13.5, ambassador 0.5.1, fundle 0.4.0, shaku 0.6.3, nject 0.5.1, service-builder 0.3.0, dill 0.17.2,
dependency-injector 2.1.0, ferrous-di 0.2.0, reinhardt-di 0.3.25, tower 0.5.3, and cap-std 4.0.3.

## In short

| Crate | What it is | Use it when |
|---|---|---|
| [delegate](https://crates.io/crates/delegate) | A macro that writes forwarding methods you list | You want a few methods forwarded to a field, inherent or trait |
| [ambassador](https://crates.io/crates/ambassador) | A derive that forwards a whole trait to a field or enum variants | A type should implement a trait entirely through a field, or an enum through its variants |
| [fundle](https://crates.io/crates/fundle) | A builder for a struct of dependencies, built in order | Many components need each other while being built |
| [shaku](https://crates.io/crates/shaku) | A compile-time-checked container of trait objects | A server needs singletons and per-request instances, with axum, actix, or Rocket integration |
| [nject](https://crates.io/crates/nject) | Compile-time providers that build types from their fields | You want constructors generated from a provider, including in `no_std` |
| [service-builder](https://crates.io/crates/service-builder) | A builder-pattern macro for structs of services | You want a builder per service struct |
| [dill](https://crates.io/crates/dill), [dependency-injector](https://crates.io/crates/dependency-injector), [ferrous-di](https://crates.io/crates/ferrous-di) | Runtime containers: register, then resolve by type | Implementations are chosen at run time, or you want Java/.NET-style lifetimes |
| [reinhardt-di](https://crates.io/crates/reinhardt-di) | FastAPI-style injection for the Reinhardt web framework | You build on Reinhardt |
| [tower](https://crates.io/crates/tower) | Request-to-response services and middleware layers | You compose network clients and servers |
| [cap-std](https://crates.io/crates/cap-std) | The standard library's file system and network through capability handles | Code must not reach files outside a directory it was given |
| loko | Resources with test, system, and JavaScript implementations, and the macros to add your own | Code should get the time, randomness, environment, console, or storage passed in, and run in tests, natively, and in the browser |

## Forwarding: delegate and ambassador

These are the closest to loko's macros.

- **delegate** writes methods that forward to a field: `delegate! { to self.inner { fn get(&self, key: &str) ->
  Option<String>; } }`. You list each signature. Methods you don't list you write yourself, so a trait impl can forward
  some methods and write others. `async fn` gets an `.await`, and generic methods work. Associated types are yours to
  write.
- **ambassador** forwards a whole trait. Mark the trait `#[delegatable_trait]` (or copy a foreign trait's signature
  with `#[delegatable_trait_remote]`), and derive `#[delegate(Trait, target = "field")]`. It forwards to enum variants
  too, and its tests cover associated types, generic methods, GATs, `async fn`, and `-> impl Future` methods.
  Forwarding is all or nothing: overriding some methods is an open request (issues #13 and #67).

**What loko adds:**
- `#[loko::bundle]` is ambassador's idea. Credited in [PRIOR_ART.md](PRIOR_ART.md).
- `#[resource]` also generates a `HasX` trait. Code can bound on it, and get the held resource with `.x()`.
- `#[wraps]` forwards everything you don't write, without listing signatures. That combines what delegate and
  ambassador each do half of.

**Use them instead** for forwarding inherent methods (delegate), for enums (ambassador), for traits you can't mark
(ambassador's remote traits), or anywhere you don't need loko's resources.

## Compile-time wiring: fundle, nject, shaku, and service-builder

These build a graph of components and check at compile time that every dependency is there.

- **fundle** generates a builder for a struct of dependencies:
  `AppState::builder().logger(|_| Logger::new()).database(|x| Database::connect(url, x)).build()`. Each field is set
  once, and its closure sees the fields set before it, so construction happens in order. It implements `AsRef<T>` for
  each field type that appears once, and components take `impl AsRef<Logger>`. It hands out concrete types, with no
  trait objects and no `Send` requirement, and has async and fallible setters.
- **nject** generates construction from providers: `provider.provide::<Service>()` builds a type from its fields,
  sharing what the provider holds by reference. It works with generics and with `&dyn` or `Box<dyn>` traits, has
  scopes, and is `no_std`.
- **shaku** declares components (singletons) and providers (a new instance each time) in a `module!`, injects them into
  fields as `Arc<dyn Trait>`, and resolves them with `module.resolve_ref()`. Interfaces must be `Send + Sync + 'static`
  with the default `thread_safe` feature (`'static` without it), and must be dyn-compatible. For async methods it
  suggests `async-trait`. It has test overrides (`with_component_override`) and integrations for axum, actix, and
  Rocket.
- **service-builder** generates `Service::builder().dependency(…).build()`. `build()` returns a `Result`, and a missing
  dependency is a `BuildError::MissingDependency` at run time.

**Where loko differs:**
- **It doesn't build a graph.** A loko bundle is a struct literal you write in `main` or in a test. Construction order
  is ordinary code.
- **It decides what code may use, and with which implementation.** Code bounds on traits (`impl Store + Clock`), not on
  concrete types (fundle's `AsRef<Database>`).
- **No trait objects.** loko never needs `dyn`, so resources with `-> impl Future` methods (`Store`, `Clock::sleep`)
  work as they are. shaku would need them boxed through `async-trait`.
- **No `Send`, `Sync`, or `'static` requirement,** so JavaScript-backed resources work.
- **It ships the resources:** test clocks, seeded randomness, an in-memory store, consoles that record. These crates
  wire your types; you bring the implementations, test ones included.

**Use them alongside or instead** when a large application has many components that need each other at
construction (fundle), or when a server needs per-request instances next to singletons and framework integration
(shaku). They combine with loko: fundle can build the struct that `#[loko::bundle]` then exposes as resources, and
a shaku component can hold a loko platform.

## Runtime containers: dill, dependency-injector, ferrous-di, and reinhardt-di

These register implementations in a container and resolve them by type at run time, as Java and .NET containers do.

- **dill** has a `Catalog`, `#[component]` constructor injection, `Arc<dyn Trait>` interfaces, `OneOf`, `AllOf`,
  `Maybe`, and `Lazy`, and transient, singleton, and transaction lifetimes. Components are `Send + Sync`.
- **dependency-injector** resolves `get::<T>()` to `Arc<T>` from a lock-free map, with singletons, lazy singletons,
  transients, and child scopes that can override their parent's services. Services are `Send + Sync + 'static`.
- **ferrous-di** follows Microsoft.Extensions.DependencyInjection: a `ServiceCollection` of factories, `Arc<T>` and
  `Arc<dyn Trait>`, and singleton, scoped, and transient lifetimes. Everything is `Send + Sync + 'static`; its `async`
  feature uses `async-trait` and tokio's multi-threaded runtime.
- **reinhardt-di** is FastAPI-style injection for the Reinhardt web framework: `Depends<…>` in handlers, async
  providers, request, singleton, and transient scopes, and test overrides. Injectables are `Send + Sync + 'static`, it
  uses `async-trait`, and its README says resolution runs only on native targets.

**Where loko differs:** nothing is resolved at run time, so a missing resource is a compile error, not a startup
failure. There's no registry to configure, and calls are statically dispatched. loko also has no lifetimes or scopes,
and no choosing implementations from configuration at run time.

**Use them instead** when the implementation is only known at run time (plugins, configuration), or when you want a
container's lifetimes and are fine with trait objects and `Send + Sync`.

## Middleware: tower

tower's `Service` is one generic trait, a request in and a future of a response out, and its `Layer` wraps a service in
another. Timeouts, retries, rate limits, and load balancing stack around any client or server.

**Where loko differs:**
- **Many specific traits instead of one generic one.** Wrapping is one feature among several, not the point of the
  library.
- **The same stacking idea, applied to resources.** loko's wrappers stack the way tower's layers do (credited in
  [PRIOR_ART.md](PRIOR_ART.md)), but there's no `Layer` trait yet ([#7](https://github.com/polythings/loko/issues/7)).
- **Complementary:** a tower retry layer could take its backoff from a loko `Clock`, so tests could skip the waits.

## Capabilities: cap-std

cap-std gives the standard library's file system and network through handles. A program opens files through a `Dir`
it was given, not by path, so it can't reach outside it. It's a foundation for WASI in Wasmtime.

**Where loko differs:** the idea is shared, passing access instead of reaching for globals, but the aim isn't.
- **cap-std confines.** It's about security, and it covers files and sockets.
- **loko substitutes.** It's about swapping implementations for tests and platforms, and it covers the time,
  randomness, the environment, the console, and object storage.
- **Closest overlap:** `PrefixedStore` (URIs under a prefix, refusing `..`) is the nearest loko comes to cap-std's
  confinement.

## What loko does that the others don't

- **Ships the resources.** A test clock whose `sleep` moves time instead of waiting, seeded randomness, an environment
  in a map, a console that records lines, and an in-memory store with conditional writes. Each has system and
  JavaScript counterparts and the same interface. The other crates wire up implementations; none ships these.
- **Async without boxing.** Resources return `impl Future`, and loko passes them as generics, never as `dyn`, so
  there's no `async-trait` and no heap allocation per call.
- **No `Send`, `Sync`, or `'static` requirement.** Futures that wrap JavaScript promises aren't `Send`. The containers
  above require `Send + Sync`; shaku requires it unless `thread_safe` is turned off.
- **Checked on WebAssembly.** CI builds every crate for `wasm32-unknown-unknown` and `wasm32-wasip2`, and the
  JavaScript implementations use the host's clock, timers, `crypto.getRandomValues`, and console.
- **Wrappers that write only what they change,** with `#[wraps]`, and ready-made ones that stack.

## What loko doesn't do

- **No container or resolution by type.** Bundles are structs you write.
- **No lifetimes or scopes** (singleton, transient, per request), and no field injection.
- **No choosing implementations at run time** from configuration or plugins.
- **No help building a graph in order,** and no web framework integrations.

For those, pair loko with the crate that does them, or use that crate on its own. For a program with a handful of
resources, chosen once and swapped in tests and in the browser, loko and ordinary Rust are enough.
