# Contributing to loko

This is the code guide for everyone who changes this repository, people and AI agents alike. Formatters and linters
enforce the mechanical rules (see [Checks](#checks)). This guide covers what they can't: how code should read.
[spec/](spec/README.md) holds the design.

## Principles

1. **Readable first.** Someone new should understand a function from its name, its signature, and one pass through
   the body.
2. **One crate per service.** A service's crate holds its trait and its implementations, and depends on no other
   service's crate, so a library can take just the service it needs. The `loko` crate re-exports them all and holds
   what combines them (the platforms).
3. **Small traits.** A service has the few methods its callers need, and provided methods for what follows from them
   (`now_ms` from `now`). Implementations are where the variety goes.
4. **Every target.** Each service has a test implementation that runs anywhere, a system one where there is a system
   (native and WASI), and a JavaScript one for `wasm32-unknown-unknown` where it makes sense.
5. **One definition of each thing.** Before writing a helper, constant, or type, look for an existing one.
6. **Behaviour, spec, and tests change together.** A change users can see updates its spec, adds or updates tests, and
   adds a line to [CHANGELOG.md](CHANGELOG.md).

## Comments and docs

- **Public items get a short doc comment**: what it does, and anything a caller must know (errors, units, which
  targets, whether it blocks). One to three lines is usually enough.
- **Comments explain why, not what.** Explain a non-obvious choice or a workaround.
- **No long comments.** If an explanation needs a page, it belongs in a spec. Link to it from the code.

## Credit

loko learns from other projects' ideas, never from copying their code. [PRIOR_ART.md](PRIOR_ART.md) lists the ideas
we took.

- **Never copy code**, whatever its license: not by pasting, and not by translating it into another language.
- **Credit ideas from open-source projects** with a row in PRIOR_ART.md and a short comment where the idea is
  implemented.
- **Don't name open-core, dual-licensed, or source-available projects, or commercial products, as a source.**

## Rust

- **loko's own services need no macros.** Their crates write out what `#[service]` generates (`HasX`, its
  `wrapped_` methods, and the blanket impl) and implement their wrappers' `HasX` by hand, so a library can depend on
  `loko-clock` without `syn`. Keep that code in the macro's shape; the tests in `crates/loko/tests` use it with
  `#[loko::bundle]` and `#[loko::wraps]`.
- **Services:** a trait marked `#[loko_macros::service]` (or written out, as above), methods on `&self`, async methods returning `impl Future`
  without `Send`. Implementations that hold state share it between clones (`Arc<Mutex<…>>`).
- **State in services:** methods take `&self`, so state that changes sits behind a `Mutex` or an atomic (counters).
  Lock only for the synchronous part of a call: never hold a `std::sync::Mutex` guard or a `RefCell` borrow across
  `.await` (Clippy's `await_holding_lock` and `await_holding_refcell_ref` catch it). Clones share state.
- **Errors:** what the platform gives (`io::Result` for the console), or the crate's own `non_exhaustive` error enum
  (`loko_store::Error`). No `unwrap()` outside tests; a poisoned lock is recovered, not a panic.
- **Layout:** the trait in `lib.rs`, and each implementation in a module named for where it runs: `testing.rs`,
  `system.rs` (native and WASI), `js.rs`, or a runtime's (`tokio_clock.rs`). Each module has its own tests.
  A wrapper gets a module named for what it does (`offset.rs`, `counting.rs`) and is written with
  `#[loko_macros::wraps]`.
- **Targets:** a module that only builds on some targets is gated once, on its `mod` line and its `pub use`:
  JavaScript code under `#[cfg(all(feature = "js", target_arch = "wasm32", target_os = "unknown"))]`, system code under
  `#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]`.
- **Suppressing a lint:** `#[expect(lint, reason = "...")]` with a real reason. Never `#[allow]`.

## Tests

- **A bug fix starts with a test that fails**, then the fix.
- **Tests run offline** and use the test implementations; a test of a system implementation checks only what holds on
  every machine.

## Checks

Install once: `uv tool install prek && prek install --install-hooks`. Hooks then run on commit and on push. Before you
push, or before calling work done:

```bash
prek run --all-files --hook-stage pre-push   # rustfmt, clippy (native, wasm32-unknown-unknown, wasm32-wasip2)
cargo test --workspace --all-features        # unit and doc tests
```

## Dependencies

Keep dependencies on their newest versions; check the latest release when adding one. Actions are pinned to a commit
with the version in a comment (`uses: actions/checkout@<sha> # v7.0.1`).

## Commits

- **Subject:** a short imperative sentence with the area first, for example `Clock: …`, `Macros: …`, or `Spec: …`.
  Add a body when the why isn't obvious.
- **Scope:** one logical change per commit.

## Changelog

[CHANGELOG.md](CHANGELOG.md) is for users, not a list of commits. Anything a user can notice gets one line under
`## Unreleased`, in `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, or `Security`, in the same commit as the
change.

## License

Contributions are licensed under the Apache License 2.0, like the rest of the project (see [LICENSE](LICENSE)).
