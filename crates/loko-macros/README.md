# loko-macros

[loko](https://github.com/polythings/loko)'s macros. Use them through the `loko` crate.

- `#[resource]` on a trait `X` adds a trait `HasX` (a type that holds an `X`, with an accessor) and implements `X` for
  every `HasX`, forwarding each method, associated type, and constant. Methods take `&self`; the trait can't be
  generic.
- `#[derive(Provides)]` implements `HasX` for each `#[provides(X, …)]` on a struct's fields, so the struct is each of
  those resources. It names `HasX` next to the path given: `#[provides(loko::Clock)]` uses `loko::HasClock`, and a bare
  `#[provides(Clock)]` needs `HasClock` in scope (`loko::prelude::*` brings in both).
- `#[wraps(field: Type)]` on `impl X for Wrapper` makes a wrapper: the methods written replace `X`'s, and the rest
  forward to `self.field`. A wrapper keeps the inner resource's associated types.

```rust
use loko::prelude::*;

#[loko::resource]
trait Greeter {
    fn greet(&self, name: &str) -> String;
}

#[derive(Provides)]
struct App<G> {
    #[provides(Clock, Random)]
    platform: TestPlatform,
    #[provides(Greeter)]
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
