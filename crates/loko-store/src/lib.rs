//! loko's `Store` resource: objects by URI, passed to code instead of reached through a particular
//! client, so the same code works over S3, a local directory, memory, or a JavaScript binding.
//!
//! - [`Store`]: get, ranged get, exists, put, put-if-absent (a conditional write), multipart upload,
//!   list by prefix, delete.
//! - [`MemoryStore`]: objects in memory, for tests.
//! - Wrappers, each a `Store` around another: [`CountingStore`] counts requests, [`ReadOnlyStore`]
//!   refuses writes, and [`PrefixedStore`] keeps to URIs under a prefix. They stack.
//! - `loko-object-store`: a `Store` over an `object_store` client (S3, GCS, Azure, local files).
//!
//! ```
//! use loko_store::{MemoryStore, Store};
//!
//! # block_on(async {
//! let store = MemoryStore::new();
//! assert!(store.put_if_absent("memory://t/a", b"1".to_vec()).await?);
//! assert!(!store.put_if_absent("memory://t/a", b"2".to_vec()).await?, "the first write wins");
//! # Ok::<(), loko_store::Error>(())
//! # }).unwrap();
//! # // MemoryStore's futures are ready at once, so one poll runs them.
//! # fn block_on<F: std::future::Future>(f: F) -> F::Output {
//! #     let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
//! #     let mut f = std::pin::pin!(f);
//! #     loop { if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
//! # }
//! ```

mod counting;
mod memory;
mod prefixed;
mod read_only;
mod store;

pub use counting::{CountingStore, StoreCounts};
pub use memory::{MemoryStore, MemoryUpload};
pub use prefixed::PrefixedStore;
pub use read_only::ReadOnlyStore;
pub use store::{HasStore, Listed, Store, Upload};

/// An error of a store.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The object isn't there.
    #[error("not found: {0}")]
    NotFound(String),
    /// The request doesn't fit the store (a URI outside it, a bad path).
    #[error("invalid: {0}")]
    Invalid(String),
    /// The store can't do this (a backend without conditional writes).
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// The store failed.
    #[error("store: {0}")]
    Store(String),
}

/// `Result` with [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Runs a future that is ready at once, as the memory store's are.
#[cfg(test)]
fn now<F: std::future::Future>(f: F) -> F::Output {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
