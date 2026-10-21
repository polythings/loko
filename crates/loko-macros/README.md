# loko-macros

[loko](https://github.com/polythings/loko)'s macros. Use them through the `loko` crate.

- `#[service]` on a trait `X` adds a trait `HasX` (a type that holds an `X`, with an accessor) and implements `X` for
  every `HasX`, forwarding each method, associated type, and constant. Methods take `&self`; the trait can't be
  generic.
- `#[derive(Services)]` implements `HasX` for each `#[service(X, …)]` on a struct's fields, so the struct is each of
  those services. It names `HasX` next to the path given: `#[service(loko::Clock)]` uses `loko::HasClock`, and a bare
  `#[service(Clock)]` needs `HasClock` in scope (`loko::prelude::*` brings in both).

```rust
use loko::prelude::*;

#[loko::service]
trait Greeter {
    fn greet(&self, name: &str) -> String;
}

#[derive(Services)]
struct App<G> {
    #[service(Clock, Random)]
    platform: TestPlatform,
    #[service(Greeter)]
    greeter: G,
}
```

Apache License 2.0.
