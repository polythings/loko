# loko-macros

[loko](https://github.com/polythings/loko)'s macros. Use them through the `loko` crate.

- `#[service]` on a trait `X` adds a trait `HasX` (a type that holds an `X`, with an accessor) and implements `X` for
  every `HasX`, forwarding each method, associated type, and constant. Methods take `&self`; the trait can't be
  generic.
- `#[loko::bundle]` implements `HasX` for each `#[loko::has(X, …)]` on a struct's fields, so the struct is each of
  those services. It names `HasX` next to the path given: `#[loko::has(loko::Clock)]` uses `loko::HasClock`, and a bare
  `#[loko::has(Clock)]` needs `HasClock` in scope (`loko::prelude::*` brings in both).
- `#[wraps(field: Type)]` on `impl X for Wrapper` makes a wrapper: the methods written replace `X`'s, and the rest
  forward to `self.field`. A wrapper keeps the inner service's associated types.

```rust
use loko::prelude::*;

#[loko::service]
trait Greeter {
    fn greet(&self, name: &str) -> String;
}

#[loko::bundle]
struct App<G> {
    #[loko::has(Clock, Random)]
    platform: TestPlatform,
    #[loko::has(Greeter)]
    greeter: G,
}

struct Excited<G>(G);

#[loko::wraps(0: G)]
impl<G: Greeter> Greeter for Excited<G> {
    fn greet(&self, name: &str) -> String {
        format!("{}!", self.0.greet(name))
    }
}
```

Apache License 2.0.
