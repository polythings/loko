use crate::{Error, HasStore, Listed, Result, Store};

/// A store that passes on only requests for URIs under a prefix (`s3://bucket/tenant-a/`), and
/// refuses the rest with [`Error::Invalid`]: to keep code to one table or one tenant's data.
/// `.` and `..` path segments are refused, so a URI can't climb out of the prefix.
#[derive(Clone, Debug, Default)]
pub struct PrefixedStore<S> {
    inner: S,
    prefix: String,
}

impl<S> PrefixedStore<S> {
    /// `inner`, kept to URIs under `prefix`. A `/` is added to the prefix if it has none, so
    /// `s3://bucket/a` doesn't let `s3://bucket/ab/…` through.
    pub fn new(inner: S, prefix: impl Into<String>) -> Self {
        let mut prefix = prefix.into();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        Self { inner, prefix }
    }

    /// The prefix requests must be under.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The store requests go to.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    fn check(&self, uri: &str) -> Result<()> {
        match uri.strip_prefix(&self.prefix) {
            Some(rest) if !rest.split('/').any(|segment| segment == "." || segment == "..") => Ok(()),
            _ => Err(Error::Invalid(format!("{uri} is outside {}", self.prefix))),
        }
    }
}

#[loko_macros::wraps(inner: S)]
impl<S: Store> Store for PrefixedStore<S> {
    async fn get(&self, uri: &str) -> Result<Option<Vec<u8>>> {
        self.check(uri)?;
        self.inner.get(uri).await
    }

    async fn get_range(&self, uri: &str, start: u64, len: u64) -> Result<Vec<u8>> {
        self.check(uri)?;
        self.inner.get_range(uri, start, len).await
    }

    async fn exists(&self, uri: &str) -> Result<bool> {
        self.check(uri)?;
        self.inner.exists(uri).await
    }

    async fn put(&self, uri: &str, bytes: Vec<u8>) -> Result<()> {
        self.check(uri)?;
        self.inner.put(uri, bytes).await
    }

    async fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> Result<bool> {
        self.check(uri)?;
        self.inner.put_if_absent(uri, bytes).await
    }

    async fn upload(&self, uri: &str) -> Result<S::Upload> {
        self.check(uri)?;
        self.inner.upload(uri).await
    }

    async fn list(&self, prefix: &str) -> Result<Vec<Listed>> {
        self.check(prefix)?;
        self.inner.list(prefix).await
    }

    async fn delete(&self, uris: &[String]) -> Result<()> {
        uris.iter().try_for_each(|uri| self.check(uri))?;
        self.inner.delete(uris).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryStore, now};

    #[test]
    fn only_uris_under_the_prefix_pass() {
        let store = PrefixedStore::new(MemoryStore::new(), "memory://wh/a");
        assert_eq!(store.prefix(), "memory://wh/a/");
        now(store.put("memory://wh/a/t/1", b"x".to_vec())).unwrap();
        assert!(now(store.exists("memory://wh/a/t/1")).unwrap());
        assert_eq!(now(store.list("memory://wh/a/t/")).unwrap().len(), 1);
        for outside in ["memory://wh/b/t/1", "memory://wh/ab/t/1", "memory://wh/a/../b/t/1", "memory://wh/a/./t/1"] {
            assert!(matches!(now(store.get(outside)), Err(Error::Invalid(_))), "{outside}");
        }
        assert!(matches!(now(store.list("memory://wh/")), Err(Error::Invalid(_))));
        let delete = ["memory://wh/a/t/1".to_string(), "memory://wh/b/t/1".to_string()];
        assert!(matches!(now(store.delete(&delete)), Err(Error::Invalid(_))));
        assert_eq!(store.inner().uris(), ["memory://wh/a/t/1"], "a refused delete deletes nothing");
    }
}
