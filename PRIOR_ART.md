# Prior art

loko stands on other open-source projects and learns from more. This file credits the ideas we took. We take ideas,
never code: every line of loko is our own. How we credit is in [CONTRIBUTING.md](CONTRIBUTING.md#credit).

## Foundations

| Project | License | What loko uses |
|---|---|---|
| [tokio](https://github.com/tokio-rs/tokio) | MIT | The timer behind `TokioClock` |
| [getrandom](https://github.com/rust-random/getrandom) | Apache-2.0 / MIT | The operating system's generator behind `OsRandom` |
| [rand_core](https://github.com/rust-random/rand) | Apache-2.0 / MIT | The generator trait `Random` accepts (feature `rand`) |
| [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) (`js-sys`, `wasm-bindgen-futures`) | Apache-2.0 / MIT | The JavaScript implementations |
| [object_store](https://github.com/apache/arrow-rs-object-store) | Apache-2.0 / MIT | The stores `loko-object-store` adapts |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | Apache-2.0 / MIT | The macros |

## Ideas

| Idea | From | License of the source | Where in loko |
|---|---|---|---|
| The time, randomness, the console, and the system as resources passed to code, each with a test implementation (a clock that moves when told) | [ZIO](https://github.com/zio/zio) (`Clock`, `Random`, `Console`, `System`, `TestClock`) | Apache-2.0 | The resource crates and `TestPlatform` |
| An environment of resources described by "has a" traits (`HasX`), so code asks for the resources it needs and any environment holding them qualifies | [RIO](https://github.com/commercialhaskell/rio) (`HasLogFunc` and the "Has" type class pattern) | MIT | `#[resource]`'s `HasX` traits |
| Implementing a trait for a struct by forwarding to a field, chosen with a derive and a field attribute | [ambassador](https://github.com/hobofan/ambassador) | Apache-2.0 / MIT | `#[derive(Provides)]` and `#[wraps]` |
| Wrappers that are the trait they wrap, so they stack | [tower](https://github.com/tower-rs/tower) (`Layer`) | MIT | `#[wraps]`, `CountingStore`, `ReadOnlyStore`, `PrefixedStore`, `OffsetClock` |
| SplitMix64: a 64-bit state advanced by a constant and mixed with two multiply-xorshift rounds | Steele, Lea, and Flood, "Fast Splittable Pseudorandom Number Generators" (OOPSLA 2014) | A published algorithm | `SeededRandom` |
| A storage interface with no I/O library of its own, supplied by the application | [delta-kernel-rs](https://github.com/delta-io/delta-kernel-rs) (its `Engine` trait), by way of lakekit | Apache-2.0 | `Store` |
| One crate per backend under an umbrella crate with a feature each | [arrow-rs](https://github.com/apache/arrow-rs), [Apache OpenDAL](https://github.com/apache/opendal) | Apache-2.0 | The workspace layout |
