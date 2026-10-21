---
number: 6
title: Store
status: done
updated: 2026-10-20
versions: [unreleased]
related: [1]
issues: [6]
---

# 6. Store

**Done** · updated 2026-10-20 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

Objects by absolute URI. Code that reads and writes files in object storage takes a `Store`, so the same code works
over S3, a local directory, memory, or a JavaScript binding. It started as lakekit's `Store`, where table formats
read and write through it.

## Design

- **`Store`:** `get` (the object or `None`), `get_range`, `exists`, `put`, `put_if_absent` (a conditional write,
  `false` when an object is there: the commit primitive of table formats), `upload` (a multipart `Upload`: parts in
  order, then `complete` or `abort`), `list` (by string prefix, not only by directory), and `delete` (missing objects
  are not an error). Paths are absolute URIs.
- **`Error`:** `NotFound`, `Invalid`, `Unsupported`, and `Store` (the backend's error, as text); `non_exhaustive`.
  Libraries with their own error type convert from it.
- **`MemoryStore`:** objects in memory, for tests. Clones share the objects.
- **`ObjectStoreAdapter`** (`loko-object-store`): a `Store` over any `object_store` client, from a client and a root
  URI or from a URL with options. Put-if-absent is `PutMode::Create`. The crate enables no backends; the application
  turns on the ones it needs on its own `object_store` dependency.
- **Not in the platforms:** a program bundles a store with a platform ([spec 7](0007-platforms.md)).

## Decisions

| ID | Decision | Status |
|---|---|---|
| D13 | `Store` and the `object_store` adapter moved from lakekit to loko, so any project can use them; lakekit keeps its own error type, converting from `loko_store::Error`. | Accepted |

## Open questions and issues

- [#6](https://github.com/polythings/loko/issues/6) Store adapter for OpenDAL (loko-opendal)

## History

- 2026-10-20: built for 0.2. Moved from lakekit (`lakekit-core`'s `Store` and `MemoryStore`, and `lakekit-object-store`).
