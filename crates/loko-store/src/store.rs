use std::future::Future;

use crate::Result;

/// An object found by [`Store::list`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Absolute URI.
    pub uri: String,
    /// Size in bytes.
    pub size: u64,
}

/// Where a table's objects live: an object store, a directory, memory, or a JavaScript binding.
/// Paths are absolute URIs, as table metadata records them.
///
/// The futures aren't required to be `Send`, so a store can wrap JavaScript promises.
pub trait Store {
    /// A multipart upload in progress.
    type Upload: Upload;

    /// The object at `uri`, or `None` if there is none.
    fn get(&self, uri: &str) -> impl Future<Output = Result<Option<Vec<u8>>>>;
    /// `len` bytes of the object at `uri` from `start`.
    fn get_range(&self, uri: &str, start: u64, len: u64) -> impl Future<Output = Result<Vec<u8>>>;
    /// Whether an object exists at `uri`.
    fn exists(&self, uri: &str) -> impl Future<Output = Result<bool>>;
    /// Writes (or replaces) the object at `uri`.
    fn put(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<()>>;
    /// Writes the object at `uri` unless one exists; `false` if one did. The commit primitive.
    fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<bool>>;
    /// Starts a multipart upload to `uri`.
    fn upload(&self, uri: &str) -> impl Future<Output = Result<Self::Upload>>;
    /// Objects whose URI starts with `prefix` (a string prefix, not only a directory).
    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>>;
    /// Deletes objects; missing ones are not an error.
    fn delete(&self, uris: &[String]) -> impl Future<Output = Result<()>>;
}

/// A type that holds a [`Store`], and so is one: `#[loko::bundle]` implements this for a field,
/// and `#[loko::wraps]` for a wrapper that changes some methods. It's what `#[loko::resource]`
/// generates, written out so this crate needs no macros.
pub trait HasStore {
    /// The resource held.
    type Store: Store;

    /// The [`Store`] this holds.
    fn store(&self) -> &Self::Store;

    /// Forwards [`Store::get`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_get(&self, uri: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> {
        self.store().get(uri)
    }

    /// Forwards [`Store::get_range`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_get_range(&self, uri: &str, start: u64, len: u64) -> impl Future<Output = Result<Vec<u8>>> {
        self.store().get_range(uri, start, len)
    }

    /// Forwards [`Store::exists`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_exists(&self, uri: &str) -> impl Future<Output = Result<bool>> {
        self.store().exists(uri)
    }

    /// Forwards [`Store::put`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_put(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        self.store().put(uri, bytes)
    }

    /// Forwards [`Store::put_if_absent`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<bool>> {
        self.store().put_if_absent(uri, bytes)
    }

    /// Forwards [`Store::upload`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_upload(&self, uri: &str) -> impl Future<Output = Result<<Self::Store as Store>::Upload>> {
        self.store().upload(uri)
    }

    /// Forwards [`Store::list`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>> {
        self.store().list(prefix)
    }

    /// Forwards [`Store::delete`] to the held resource; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_delete(&self, uris: &[String]) -> impl Future<Output = Result<()>> {
        self.store().delete(uris)
    }
}

impl<H: HasStore> Store for H {
    type Upload = <<H as HasStore>::Store as Store>::Upload;

    #[inline]
    fn get(&self, uri: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> {
        self.wrapped_get(uri)
    }

    #[inline]
    fn get_range(&self, uri: &str, start: u64, len: u64) -> impl Future<Output = Result<Vec<u8>>> {
        self.wrapped_get_range(uri, start, len)
    }

    #[inline]
    fn exists(&self, uri: &str) -> impl Future<Output = Result<bool>> {
        self.wrapped_exists(uri)
    }

    #[inline]
    fn put(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<()>> {
        self.wrapped_put(uri, bytes)
    }

    #[inline]
    fn put_if_absent(&self, uri: &str, bytes: Vec<u8>) -> impl Future<Output = Result<bool>> {
        self.wrapped_put_if_absent(uri, bytes)
    }

    #[inline]
    fn upload(&self, uri: &str) -> impl Future<Output = Result<Self::Upload>> {
        self.wrapped_upload(uri)
    }

    #[inline]
    fn list(&self, prefix: &str) -> impl Future<Output = Result<Vec<Listed>>> {
        self.wrapped_list(prefix)
    }

    #[inline]
    fn delete(&self, uris: &[String]) -> impl Future<Output = Result<()>> {
        self.wrapped_delete(uris)
    }
}

/// A multipart upload: parts in order, then [`Upload::complete`].
pub trait Upload {
    /// Sends the next part. Every part but the last must be at least the store's minimum (5 MiB on
    /// S3 and R2).
    fn put_part(&mut self, bytes: Vec<u8>) -> impl Future<Output = Result<()>>;
    /// Completes the upload: the object appears.
    fn complete(self) -> impl Future<Output = Result<()>>;
    /// Drops the parts sent.
    fn abort(self) -> impl Future<Output = ()>;
}
