# loko-store

[loko](https://github.com/polythings/loko)'s `Store` service: objects by absolute URI, passed to code instead of
reached through a particular client, so the same code works over S3, a local directory, memory, or a JavaScript
binding.

- `Store`: `get`, `get_range`, `exists`, `put`, `put_if_absent` (a conditional write: `false` when an object is
  there), `upload` (multipart: parts in order, then complete or abort), `list` (by string prefix), and `delete`
  (missing objects are not an error). The futures aren't required to be `Send`, so a store can wrap JavaScript
  promises.
- `MemoryStore`: objects in memory, for tests. Clones share the objects.
- `Error`: `NotFound`, `Invalid`, `Unsupported`, and `Store` (the backend's error, as text).
- [`loko-object-store`](https://crates.io/crates/loko-object-store) implements `Store` over an `object_store` client.

Apache License 2.0.
