# loko-env

[loko](https://github.com/polythings/loko)'s `Env` resource: environment variables, passed to code instead of read from
the process, so tests set their own and code in a browser (which has none) reads them from a map.

- `Env`: `var` and `vars`.
- `SystemEnv`: the process's variables.
- `MapEnv`: variables in a map, for tests and for hosts without a process environment. Clones share the map.

```rust
use loko_env::{Env, MapEnv};

let env = MapEnv::new().with("REGION", "us-east-2");
assert_eq!(env.var("REGION").as_deref(), Some("us-east-2"));
```

Apache License 2.0.
