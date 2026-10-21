use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{HasStore, Listed, Result, Store};

/// How many calls of each kind a [`CountingStore`] passed on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct StoreCounts {
    /// `get` calls.
    pub get: u64,
    /// `get_range` calls.
    pub get_range: u64,
    /// `exists` calls.
    pub exists: u64,
    /// `put` calls.
    pub put: u64,
    /// `put_if_absent` calls.
    pub put_if_absent: u64,
    /// `upload` calls (uploads started).
    pub upload: u64,
    /// `list` calls.
    pub list: u64,
    /// `delete` calls (each may delete several objects).
    pub delete: u64,
}

impl StoreCounts {
    /// All calls.
    pub fn total(&self) -> u64 {
        self.get + self.get_range + self.exists + self.put + self.put_if_absent + self.upload + self.list + self.delete
    }
}

/// The calls of each kind, in [`StoreCounts`] order.
#[derive(Debug, Default)]
struct Counters([AtomicU64; 8]);

/// A store that counts the calls it passes on to another: to show what reading a table costs, or to
/// check in a test that a cache saves requests. Clones share the counts.
#[derive(Clone, Debug, Default)]
pub struct CountingStore<S> {
    inner: S,
    counters: Arc<Counters>,
}

impl<S> CountingStore<S> {
    /// Counts the calls to `inner`, from zero.
    pub fn new(inner: S) -> Self {
        Self { inner, counters: Arc::default() }
    }

    /// The calls so far.
    pub fn counts(&self) -> StoreCounts {
        let [get, get_range, exists, put, put_if_absent, upload, list, delete] =
            self.counters.0.each_ref().map(|c| c.load(Ordering::Relaxed));
        StoreCounts { get, get_range, exists, put, put_if_absent, upload, list, delete }
    }

    /// Starts the counts again from zero.
    pub fn reset(&self) {
        for counter in &self.counters.0 {
            counter.store(0, Ordering::Relaxed);
        }
    }

    /// The store counted.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    fn count(&self, kind: usize) {
        self.counters.0[kind].fetch_add(1, Ordering::Relaxed);
    }
}

#[loko_macros::wraps(inner: S)]
impl<S: Store> Store for CountingStore<S> {
    fn get(&self, uri: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> {
        self.count(0);
        self.inner.get(uri)
    }

    fn get_range(&self, uri: &str, start: u64, len: u64) -> impl Future<Output = Result<Vec<u8>>> {
        self.count(1);
        self.inner.get_range(uri, start, len)
    }

    fn exists(&self, uri: &str) -> impl Future<Output = Result<bool>> {
        self.count(2);
        self.inner.exists(uri)
    }

    fn put(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        self.count(3);
        self.inner.put(uri, bytes)
    }

    fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<bool>> {
        self.count(4);
        self.inner.put_if_absent(uri, bytes)
    }

    fn upload(&self, uri: &str) -> impl Future<Output = Result<S::Upload>> {
        self.count(5);
        self.inner.upload(uri)
    }

    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>> {
        self.count(6);
        self.inner.list(prefix)
    }

    fn delete(&self, uris: &[String]) -> impl Future<Output = Result<()>> {
        self.count(7);
        self.inner.delete(uris)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryStore, now};

    #[test]
    fn calls_are_counted_by_kind() {
        let store = CountingStore::new(MemoryStore::new());
        now(store.put("memory://t/a", b"abc".to_vec())).unwrap();
        now(store.get("memory://t/a")).unwrap();
        now(store.get_range("memory://t/a", 0, 1)).unwrap();
        now(store.get_range("memory://t/a", 1, 1)).unwrap();
        now(store.list("memory://t/")).unwrap();
        let shared = store.clone();
        assert_eq!(
            shared.counts(),
            StoreCounts { put: 1, get: 1, get_range: 2, list: 1, ..StoreCounts::default() },
            "clones share the counts"
        );
        assert_eq!(store.counts().total(), 5);
        store.reset();
        assert_eq!(store.counts(), StoreCounts::default());
        assert_eq!(store.inner().uris(), ["memory://t/a"]);
    }
}
