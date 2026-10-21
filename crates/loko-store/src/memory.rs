use std::collections::BTreeMap;
use std::future::{Future, ready};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::{Error, Listed, Result, Store, Upload};

/// A [`Store`] in memory, for tests and examples. Clones share the same objects.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// The URIs stored, sorted.
    pub fn uris(&self) -> Vec<String> {
        self.objects().keys().cloned().collect()
    }

    fn objects(&self) -> MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        lock(&self.objects)
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// An upload to a [`MemoryStore`].
#[derive(Debug)]
pub struct MemoryUpload {
    store: MemoryStore,
    uri: String,
    bytes: Vec<u8>,
}

impl Upload for MemoryUpload {
    fn put_part(&mut self, bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        self.bytes.extend(bytes);
        ready(Ok(()))
    }

    fn complete(self) -> impl Future<Output = Result<()>> {
        self.store.objects().insert(self.uri, self.bytes);
        ready(Ok(()))
    }

    fn abort(self) -> impl Future<Output = ()> {
        ready(())
    }
}

impl Store for MemoryStore {
    type Upload = MemoryUpload;

    fn get(&self, uri: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> {
        ready(Ok(self.objects().get(uri).cloned()))
    }

    fn get_range(&self, uri: &str, start: u64, len: u64) -> impl Future<Output = Result<Vec<u8>>> {
        let objects = self.objects();
        ready(objects.get(uri).ok_or_else(|| Error::NotFound(uri.to_string())).map(|bytes| {
            let start = usize::try_from(start).unwrap_or(usize::MAX).min(bytes.len());
            let end = start.saturating_add(usize::try_from(len).unwrap_or(usize::MAX)).min(bytes.len());
            bytes[start..end].to_vec()
        }))
    }

    fn exists(&self, uri: &str) -> impl Future<Output = Result<bool>> {
        ready(Ok(self.objects().contains_key(uri)))
    }

    fn put(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        self.objects().insert(uri.to_string(), bytes);
        ready(Ok(()))
    }

    fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<bool>> {
        let mut objects = self.objects();
        let absent = !objects.contains_key(uri);
        if absent {
            objects.insert(uri.to_string(), bytes);
        }
        ready(Ok(absent))
    }

    fn upload(&self, uri: &str) -> impl Future<Output = Result<MemoryUpload>> {
        ready(Ok(MemoryUpload { store: self.clone(), uri: uri.to_string(), bytes: vec![] }))
    }

    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>> {
        let listed = self
            .objects()
            .range(prefix.to_string()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| Listed { uri: k.clone(), size: v.len() as u64 })
            .collect();
        ready(Ok(listed))
    }

    fn delete(&self, uris: &[String]) -> impl Future<Output = Result<()>> {
        let mut objects = self.objects();
        for uri in uris {
            objects.remove(uri);
        }
        ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::now;

    #[test]
    fn objects_round_trip() {
        let store = MemoryStore::new();
        assert_eq!(now(store.get("memory://t/a")).unwrap(), None);
        now(store.put("memory://t/a", b"hello".to_vec())).unwrap();
        assert_eq!(now(store.get_range("memory://t/a", 1, 3)).unwrap(), b"ell");
        assert!(matches!(now(store.get_range("memory://t/b", 0, 1)), Err(Error::NotFound(_))));
        assert!(now(store.put_if_absent("memory://t/b", b"1".to_vec())).unwrap());
        assert!(!now(store.put_if_absent("memory://t/b", b"2".to_vec())).unwrap());

        let mut upload = now(store.upload("memory://t/c")).unwrap();
        now(upload.put_part(b"ab".to_vec())).unwrap();
        now(upload.put_part(b"cd".to_vec())).unwrap();
        now(upload.complete()).unwrap();
        assert_eq!(now(store.get("memory://t/c")).unwrap().as_deref(), Some(&b"abcd"[..]));

        let listed: Vec<String> = now(store.list("memory://t/")).unwrap().into_iter().map(|o| o.uri).collect();
        assert_eq!(listed, ["memory://t/a", "memory://t/b", "memory://t/c"]);
        now(store.delete(&["memory://t/a".into(), "memory://t/missing".into()])).unwrap();
        assert_eq!(store.uris(), ["memory://t/b", "memory://t/c"]);
        assert!(store.clone().uris().len() == 2, "clones share the objects");
    }
}
