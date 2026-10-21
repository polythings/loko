use std::future::{Future, ready};

use crate::{Error, HasStore, Result, Store};

/// A store that reads through to another and refuses writes with [`Error::Unsupported`]: for a
/// viewer that must not change the tables it opens.
#[derive(Clone, Debug, Default)]
pub struct ReadOnlyStore<S> {
    inner: S,
}

impl<S> ReadOnlyStore<S> {
    /// Reads from `inner`, never writes.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    /// The store read.
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

fn refused(uri: &str) -> Error {
    Error::Unsupported(format!("{uri}: the store is read-only"))
}

impl<S: Store> HasStore for ReadOnlyStore<S> {
    type Store = S;

    fn store(&self) -> &S {
        &self.inner
    }

    fn wrapped_put(&self, uri: &str, _bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        ready(Err(refused(uri)))
    }

    fn wrapped_put_if_absent(&self, uri: &str, _bytes: Vec<u8>) -> impl Future<Output = Result<bool>> {
        ready(Err(refused(uri)))
    }

    fn wrapped_upload(&self, uri: &str) -> impl Future<Output = Result<S::Upload>> {
        ready(Err(refused(uri)))
    }

    fn wrapped_delete(&self, uris: &[String]) -> impl Future<Output = Result<()>> {
        ready(Err(refused(uris.first().map_or("", String::as_str))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryStore, now};

    #[test]
    fn reads_pass_and_writes_are_refused() {
        let memory = MemoryStore::new();
        now(memory.put("memory://t/a", b"abc".to_vec())).unwrap();
        let store = ReadOnlyStore::new(memory.clone());
        assert_eq!(now(store.get("memory://t/a")).unwrap().as_deref(), Some(&b"abc"[..]));
        assert_eq!(now(store.list("memory://t/")).unwrap().len(), 1);
        assert!(matches!(now(store.put("memory://t/b", vec![])), Err(Error::Unsupported(_))));
        assert!(matches!(now(store.put_if_absent("memory://t/b", vec![])), Err(Error::Unsupported(_))));
        assert!(matches!(now(store.upload("memory://t/b")), Err(Error::Unsupported(_))));
        assert!(matches!(now(store.delete(&["memory://t/a".into()])), Err(Error::Unsupported(_))));
        assert_eq!(memory.uris(), ["memory://t/a"], "nothing changed");
    }
}
