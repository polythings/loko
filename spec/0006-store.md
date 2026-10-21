---
number: 6
title: Store
status: done
updated: 2026-10-21
versions: [unreleased]
related: [1]
issues: [6]
---

# 6. Store

**Done** · updated 2026-10-21 · versions unreleased · milestone: [0.2](milestones/v0-2.md)

## Summary

Objects by absolute URI. Code that reads and writes files in object storage takes a `Store`, so the same code works
over S3, a local directory, memory, or a JavaScript binding.

## Design

- **`Store`:** `get` (the object or `None`), `get_range`, `exists`, `put`, `put_if_absent` (a conditional write,
  `false` when an object is there: the commit primitive of table formats), `upload` (a multipart `Upload`: parts in
  order, then `complete` or `abort`), `list` (by string prefix, not only by directory), and `delete` (missing objects
  are not an error). Paths are absolute URIs.
- **`Error`:** `NotFound`, `Invalid`, `Unsupported`, and `Store` (the backend's error, as text); `non_exhaustive`.
  Libraries with their own error type convert from it.
- **`MemoryStore`:** objects in memory, for tests. Clones share the objects.
- **Wrappers** ([spec 1](0001-resources.md)), each a `Store` around another; they stack:
  - `CountingStore`: counts the calls of each kind it passes on (`counts()`, `reset()`); clones share the counts.
  - `ReadOnlyStore`: reads through; `put`, `put_if_absent`, `upload`, and `delete` return `Unsupported`.
  - `PrefixedStore`: passes on only URIs under its prefix (a `/` is added if missing), refusing the rest and any `.`
    or `..` segment with `Invalid`. A `delete` with one URI outside deletes nothing.
- **`ObjectStoreAdapter`** (`loko-object-store`): a `Store` over any `object_store` client, from a client and a root
  URI or from a URL with options (the root is everything before the path: scheme, userinfo such as Azure's
  filesystem, host, and port; a password shows as `***` in errors and `Debug`). Put-if-absent is
  `PutMode::Create`. An empty range, or one starting past the end, reads as no bytes, as in `MemoryStore`. Listing a
  prefix lists its directory from an offset just before the prefix, so `…/metadata/v` skips the manifests before it. The crate enables no backends; the application
  turns on the ones it needs on its own `object_store` dependency.
- **Not in the platforms:** a program bundles a store with a platform ([spec 7](0007-platforms.md)).

## Decisions

| ID | Decision | Status |
|---|---|---|
| D13 | `Store` and the `object_store` adapter are loko crates of their own, so any library can take a store without a storage client; a library with its own error type converts from `loko_store::Error`. | Accepted |

## Open questions and issues

- [#6](https://github.com/polythings/loko/issues/6) Store adapter for OpenDAL (loko-opendal)

## History

- 2026-10-20: built for 0.2.
- 2026-10-21: review fixes: `from_url` keeps the port, empty and past-the-end ranges read as no bytes,
  and listing starts from an offset. Wrappers `CountingStore`, `ReadOnlyStore`, and `PrefixedStore` added.
