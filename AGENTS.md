# Agent instructions

loko: resources for Rust programs (`Clock`, `Random`, `Env`, `Console`, `Store`) passed to code as traits, with test,
system, and JavaScript implementations. One crate per resource (`crates/loko-<resource>`), the derive macros
(`crates/loko-macros`), and the `loko` crate, which re-exports them and adds the platforms. The polythings projects
(Timelake, lakekit, lakeglass) use it.

## Before changing code

- Follow [CONTRIBUTING.md](CONTRIBUTING.md). It is the code guide, and it applies to you.
- For anything beyond a small fix, check the specs ([spec/README.md](spec/README.md)). Agree on the design with the user
  before building something new, and record the decision in the spec's Decisions table.

## Rules

- **Done means checked.** Before you report work as finished, run `prek run --all-files --hook-stage pre-push` and
  `cargo test --workspace --all-features`. Report failures as failures.
- **Fix the code, not the checks.** Don't loosen lint or format configs. Don't add `#[allow]`. Use
  `#[expect(..., reason = "...")]` only when the tool is genuinely wrong.
- **Keep things in sync:** a change in behaviour updates its spec (status, design, history), the crate's README, the
  tests, and `CHANGELOG.md` (under `Unreleased`) in the same commit.
- **Everything builds for WASM.** Every crate builds for `wasm32-unknown-unknown` and `wasm32-wasip2`. JavaScript
  implementations are gated to `wasm32-unknown-unknown` and feature `js`; system ones to targets that have a system.
  CI checks all three targets.
- **Every resource is a `#[loko_macros::resource]` trait** whose methods take `&self` and return `impl Future` without a
  `Send` bound when they are async.
- **Test implementations are deterministic** and their clones share state.
- **Ideas, not code.** Never copy or translate another project's code. Credit ideas in `PRIOR_ART.md`
  (CONTRIBUTING.md, "Credit").
- **Tests run offline.** No network, cloud accounts, or real buckets in tests.
- **Never commit secrets** or `.env*` files.
