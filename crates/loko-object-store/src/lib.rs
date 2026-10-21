//! A loko [`Store`] over an [`object_store`] client: S3, GCS, Azure, local files, memory, and
//! anything else that implements `ObjectStore`. Applications enable the backends they need on their
//! own `object_store` dependency and hand the client to [`ObjectStoreAdapter::new`], or build one
//! from a URL with [`ObjectStoreAdapter::from_url`].
//!
//! loko names objects by absolute URI (`s3://bucket/warehouse/t/metadata/v1.metadata.json`);
//! the adapter maps them to paths under its root (`s3://bucket`).
//!
//! ```
//! use std::sync::Arc;
//!
//! use loko_object_store::ObjectStoreAdapter;
//! use object_store::memory::InMemory;
//!
//! let store = ObjectStoreAdapter::new(Arc::new(InMemory::new()), "memory://");
//! ```

use std::fmt;
use std::future::Future;
use std::sync::Arc;

use futures_util::TryStreamExt;
use loko_store::{Error, Listed, Result, Store, Upload};
use object_store::path::Path;
use object_store::{MultipartUpload, ObjectStore, ObjectStoreExt, PutMode, PutOptions, PutPayload};
use url::Url;

/// A [`Store`] over an `object_store` client whose objects have URIs under `root`.
#[derive(Clone)]
pub struct ObjectStoreAdapter {
    store: Arc<dyn ObjectStore>,
    root: String,
    /// The root as shown in errors and `Debug`, without a password.
    shown: String,
}

impl fmt::Debug for ObjectStoreAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectStoreAdapter").field("store", &self.store).field("root", &self.shown).finish()
    }
}

fn store_error(e: object_store::Error) -> Error {
    match e {
        object_store::Error::NotFound { path, .. } => Error::NotFound(path),
        object_store::Error::NotImplemented { operation, implementer } => {
            Error::Unsupported(format!("{implementer} has no {operation}"))
        }
        other => Error::Store(other.to_string()),
    }
}

impl ObjectStoreAdapter {
    /// `store`, whose objects have URIs under `root`: the object at path `a/b` is `<root>/a/b`. The
    /// root is the store's scheme and bucket (`s3://bucket`, `gs://bucket`, `memory://`), or
    /// `file:///` for a local file system rooted at `/`.
    pub fn new(store: Arc<dyn ObjectStore>, root: impl Into<String>) -> Self {
        let mut root = root.into();
        if !root.ends_with('/') {
            root.push('/');
        }
        let shown = redacted(&root);
        Self { store, root, shown }
    }

    /// The store for `url`'s scheme and bucket (`s3://bucket/…`), configured with `options`
    /// (`object_store`'s keys, such as `aws_region`). Its backend must be enabled on
    /// `object_store`.
    pub fn from_url<I, K, V>(url: &str, options: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let parsed = Url::parse(url).map_err(|e| Error::Invalid(format!("{url}: {e}")))?;
        let root = root_of(&parsed);
        let root_url = Url::parse(&root).map_err(|e| Error::Invalid(format!("{root}: {e}")))?;
        let (store, _) = object_store::parse_url_opts(&root_url, options).map_err(store_error)?;
        Ok(Self::new(Arc::from(store), root))
    }

    /// The path of the object at `uri`.
    fn path(&self, uri: &str) -> Result<Path> {
        let rest = uri
            .strip_prefix(&self.root)
            .ok_or_else(|| Error::Invalid(format!("{uri} is outside this store ({})", self.shown)))?;
        Path::parse(rest).map_err(|e| Error::Invalid(format!("{uri}: {e}")))
    }

    /// The URI of the object at `path`.
    fn uri(&self, path: &Path) -> String {
        format!("{}{path}", self.root)
    }
}

/// The root of `url`: everything before the path, so its scheme, userinfo, host, and port
/// (`abfss://filesystem@account.dfs.core.windows.net/`, `http://localhost:9000/`), which the
/// URIs of its objects start with. `file:///data/t` gives `file:///`, a file system rooted at `/`.
fn root_of(url: &Url) -> String {
    let Some(host) = url.host_str().filter(|h| !h.is_empty()) else {
        return format!("{}:///", url.scheme());
    };
    let userinfo = match (url.username(), url.password()) {
        ("", None) => String::new(),
        (user, None) => format!("{user}@"),
        (user, Some(password)) => format!("{user}:{password}@"),
    };
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    format!("{}://{userinfo}{host}{port}/", url.scheme())
}

/// `root` with any password replaced by `***`, for errors and `Debug`.
fn redacted(root: &str) -> String {
    let Ok(mut url) = Url::parse(root) else { return root.to_string() };
    if url.password().is_none() || url.set_password(Some("***")).is_err() {
        return root.to_string();
    }
    url.to_string()
}

/// An offset for listing directory `dir` that skips objects sorting before `name`, the start of the
/// object names wanted. `object_store` lists the objects after the offset, so the offset is `name`
/// with its last character one lower (`v` → `u`), which still keeps an object named exactly `name`.
/// Only for letters and digits, whose predecessor is a letter or digit that paths keep as it is.
fn list_offset(dir: &str, name: &str) -> Option<Path> {
    let last = name.chars().next_back()?;
    if !last.is_ascii_alphanumeric() || matches!(last, '0' | 'A' | 'a') {
        return None;
    }
    let lower = char::from(u8::try_from(last).ok()? - 1);
    let start = &name[..name.len() - 1];
    let offset = if dir.is_empty() { format!("{start}{lower}") } else { format!("{dir}/{start}{lower}") };
    Path::parse(offset).ok()
}

/// A multipart upload through `object_store`.
#[derive(Debug)]
pub struct ObjectStoreUpload(Box<dyn MultipartUpload>);

impl Upload for ObjectStoreUpload {
    async fn put_part(&mut self, bytes: Vec<u8>) -> Result<()> {
        self.0.put_part(PutPayload::from(bytes)).await.map_err(store_error)
    }

    async fn complete(mut self) -> Result<()> {
        self.0.complete().await.map(drop).map_err(store_error)
    }

    async fn abort(mut self) {
        let _ = self.0.abort().await;
    }
}

impl Store for ObjectStoreAdapter {
    type Upload = ObjectStoreUpload;

    async fn get(&self, uri: &str) -> Result<Option<Vec<u8>>> {
        match self.store.get(&self.path(uri)?).await {
            Ok(found) => Ok(Some(found.bytes().await.map_err(store_error)?.to_vec())),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(store_error(e)),
        }
    }

    async fn get_range(&self, uri: &str, start: u64, len: u64) -> Result<Vec<u8>> {
        // `object_store` rejects empty ranges and ranges starting past the end, which a `Store`
        // answers with no bytes (as `MemoryStore` does) when the object exists.
        let path = self.path(uri)?;
        if len == 0 {
            return self.store.head(&path).await.map(|_| Vec::new()).map_err(store_error);
        }
        match self.store.get_range(&path, start..start.saturating_add(len)).await {
            Ok(bytes) => Ok(bytes.to_vec()),
            Err(e @ object_store::Error::NotFound { .. }) => Err(store_error(e)),
            Err(e) => match self.store.head(&path).await {
                Ok(meta) if start >= meta.size => Ok(Vec::new()),
                _ => Err(store_error(e)),
            },
        }
    }

    async fn exists(&self, uri: &str) -> Result<bool> {
        match self.store.head(&self.path(uri)?).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(store_error(e)),
        }
    }

    async fn put(&self, uri: &str, bytes: Vec<u8>) -> Result<()> {
        self.store.put(&self.path(uri)?, PutPayload::from(bytes)).await.map(drop).map_err(store_error)
    }

    async fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> Result<bool> {
        let options = PutOptions { mode: PutMode::Create, ..PutOptions::default() };
        match self.store.put_opts(&self.path(uri)?, PutPayload::from(bytes), options).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::AlreadyExists { .. } | object_store::Error::Precondition { .. }) => Ok(false),
            Err(e) => Err(store_error(e)),
        }
    }

    async fn upload(&self, uri: &str) -> Result<ObjectStoreUpload> {
        Ok(ObjectStoreUpload(self.store.put_multipart(&self.path(uri)?).await.map_err(store_error)?))
    }

    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>> {
        // `object_store` lists by directory; loko by string prefix (`…/metadata/v`). List the
        // directory the prefix is in, from just before the prefix, and keep what starts with it.
        let split = prefix.strip_prefix(&self.root).map(|rest| rest.rsplit_once('/').unwrap_or(("", rest)));
        async move {
            let (relative, name) =
                split.ok_or_else(|| Error::Invalid(format!("{prefix} is outside this store ({})", self.shown)))?;
            let dir = Path::parse(relative).map_err(|e| Error::Invalid(format!("{prefix}: {e}")))?;
            let dir = (!relative.is_empty()).then_some(&dir);
            let listing = match list_offset(relative, name) {
                Some(offset) => self.store.list_with_offset(dir, &offset),
                None => self.store.list(dir),
            };
            let objects: Vec<_> = listing.try_collect().await.map_err(store_error)?;
            Ok(objects
                .into_iter()
                .map(|o| Listed { uri: self.uri(&o.location), size: o.size })
                .filter(|o| o.uri.starts_with(prefix))
                .collect())
        }
    }

    async fn delete(&self, uris: &[String]) -> Result<()> {
        let deletes = uris.iter().map(|uri| async move {
            match self.store.delete(&self.path(uri)?).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
                Err(e) => Err(store_error(e)),
            }
        });
        futures_util::future::try_join_all(deletes).await.map(drop)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use object_store::memory::InMemory;

    use super::*;

    fn adapter() -> ObjectStoreAdapter {
        ObjectStoreAdapter::new(Arc::new(InMemory::new()), "memory://")
    }

    #[tokio::test]
    async fn objects_round_trip() {
        let store = adapter();
        assert_eq!(store.get("memory://t/a").await.unwrap(), None);
        assert!(!store.exists("memory://t/a").await.unwrap());
        store.put("memory://t/a", b"hello".to_vec()).await.unwrap();
        assert_eq!(store.get("memory://t/a").await.unwrap().as_deref(), Some(&b"hello"[..]));
        assert_eq!(store.get_range("memory://t/a", 1, 3).await.unwrap(), b"ell");
        assert!(store.put_if_absent("memory://t/b", b"1".to_vec()).await.unwrap());
        assert!(!store.put_if_absent("memory://t/b", b"2".to_vec()).await.unwrap(), "the first write wins");
        assert_eq!(store.get("memory://t/b").await.unwrap().as_deref(), Some(&b"1"[..]));

        let mut upload = store.upload("memory://t/c").await.unwrap();
        upload.put_part(b"ab".to_vec()).await.unwrap();
        upload.put_part(b"cd".to_vec()).await.unwrap();
        upload.complete().await.unwrap();
        assert_eq!(store.get("memory://t/c").await.unwrap().as_deref(), Some(&b"abcd"[..]));

        store.delete(&["memory://t/a".into(), "memory://t/missing".into()]).await.unwrap();
        assert!(!store.exists("memory://t/a").await.unwrap());
        assert!(matches!(store.get("s3://elsewhere/x").await, Err(Error::Invalid(_))));
    }

    #[tokio::test]
    async fn lists_by_string_prefix() {
        let store = adapter();
        for name in ["v1.metadata.json", "v2.metadata.json", "version-hint.text", "snap-1.avro"] {
            store.put(&format!("memory://t/metadata/{name}"), vec![0; 3]).await.unwrap();
        }
        store.put("memory://t/data/v9.parquet", vec![]).await.unwrap();
        let mut listed: Vec<String> =
            store.list("memory://t/metadata/v").await.unwrap().into_iter().map(|o| o.uri).collect();
        listed.sort();
        assert_eq!(
            listed,
            [
                "memory://t/metadata/v1.metadata.json",
                "memory://t/metadata/v2.metadata.json",
                "memory://t/metadata/version-hint.text"
            ]
        );
        assert_eq!(store.list("memory://t/metadata/s").await.unwrap()[0].size, 3);
        assert_eq!(store.list("memory://t").await.unwrap().len(), 5, "a prefix right under the root");
    }

    #[tokio::test]
    async fn empty_and_past_the_end_ranges_are_empty() {
        let store = adapter();
        store.put("memory://t/a", b"abc".to_vec()).await.unwrap();
        assert_eq!(store.get_range("memory://t/a", 1, 0).await.unwrap(), b"");
        assert_eq!(store.get_range("memory://t/a", 3, 2).await.unwrap(), b"");
        assert_eq!(store.get_range("memory://t/a", 9, 2).await.unwrap(), b"");
        assert_eq!(store.get_range("memory://t/a", 1, 10).await.unwrap(), b"bc", "cut at the end");
        assert!(matches!(store.get_range("memory://t/missing", 0, 0).await, Err(Error::NotFound(_))));
        assert!(matches!(store.get_range("memory://t/missing", 0, 2).await, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn listing_from_an_offset_keeps_the_exact_prefix() {
        let store = adapter();
        for name in ["snap-1.avro", "u9", "v", "v1.metadata.json", "w"] {
            store.put(&format!("memory://t/metadata/{name}"), vec![]).await.unwrap();
        }
        let mut listed: Vec<String> =
            store.list("memory://t/metadata/v").await.unwrap().into_iter().map(|o| o.uri).collect();
        listed.sort();
        assert_eq!(listed, ["memory://t/metadata/v", "memory://t/metadata/v1.metadata.json"]);
    }

    #[test]
    fn offsets_sort_just_before_the_prefix() {
        assert_eq!(list_offset("t/metadata", "v").map(|p| p.to_string()).as_deref(), Some("t/metadata/u"));
        assert_eq!(list_offset("", "t").map(|p| p.to_string()).as_deref(), Some("s"));
        assert_eq!(list_offset("t", "snap-9").map(|p| p.to_string()).as_deref(), Some("t/snap-8"));
        assert!(list_offset("t", "").is_none(), "a whole directory");
        assert!(list_offset("t", "a").is_none() && list_offset("t", "x-").is_none(), "no safe predecessor");
    }

    #[test]
    fn roots_keep_what_uris_start_with() {
        let root = |url: &str| root_of(&Url::parse(url).unwrap());
        assert_eq!(root("http://localhost:9000/bucket/t"), "http://localhost:9000/");
        assert_eq!(root("https://example.com:443/t"), "https://example.com/", "443 is https's own");
        assert_eq!(root("abfss://fs@account.dfs.core.windows.net/t"), "abfss://fs@account.dfs.core.windows.net/");
        assert_eq!(root("https://user:secret@example.com/t"), "https://user:secret@example.com/");
        assert_eq!(root("s3://bucket/warehouse/t"), "s3://bucket/");
        assert_eq!(root("file:///data/t"), "file:///");
    }

    #[tokio::test]
    async fn passwords_stay_out_of_errors_and_debug() {
        let store = ObjectStoreAdapter::new(Arc::new(InMemory::new()), "memory://user:secret@host/");
        assert!(!format!("{store:?}").contains("secret"));
        let Err(Error::Invalid(message)) = store.get("s3://elsewhere/x").await else { panic!("outside the store") };
        assert!(message.contains("user:***@host") && !message.contains("secret"), "{message}");
        assert_eq!(redacted("s3://bucket/"), "s3://bucket/");
    }

    #[tokio::test]
    async fn local_files_have_file_uris() {
        let dir = tempfile::tempdir().unwrap();
        let store = ObjectStoreAdapter::from_url("file:///", std::iter::empty::<(&str, String)>()).unwrap();
        let uri = format!("file://{}/t/a.json", dir.path().display());
        store.put(&uri, b"{}".to_vec()).await.unwrap();
        assert!(dir.path().join("t/a.json").exists());
        let listed = store.list(&format!("file://{}/t/", dir.path().display())).await.unwrap();
        assert_eq!(listed[0].uri, uri);
    }
}
