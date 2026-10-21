//! loko's `Env` service: environment variables, passed to code instead of read from the process, so
//! tests set their own and code in a browser (which has none) gets them from a map.
//!
//! - [`Env`]: `var` and `vars`.
//! - `SystemEnv`: the process's variables (native targets and WASI).
//! - [`MapEnv`]: variables in a map, for tests and for hosts without a process environment.
//!
//! ```
//! use loko_env::{Env, MapEnv};
//!
//! let env = MapEnv::new().with("REGION", "us-east-2");
//! assert_eq!(env.var("REGION").as_deref(), Some("us-east-2"));
//! assert_eq!(env.var("MISSING"), None);
//! ```

mod map;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod system;

pub use map::MapEnv;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use system::SystemEnv;

/// Environment variables.
pub trait Env {
    /// The variable `name`, or `None` if it isn't set (or isn't valid Unicode).
    fn var(&self, name: &str) -> Option<String>;

    /// Every variable, as `(name, value)` pairs.
    fn vars(&self) -> Vec<(String, String)>;
}

/// A type that holds a [`Env`], and so is one: `#[loko::bundle]` implements this for a field,
/// and `#[loko::wraps]` for a wrapper that changes some methods. It's what `#[loko::service]`
/// generates, written out so this crate needs no macros.
pub trait HasEnv {
    /// The service held.
    type Env: Env;

    /// The [`Env`] this holds.
    fn env(&self) -> &Self::Env;

    /// Forwards [`Env::var`] to the held service; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_var(&self, name: &str) -> Option<String> {
        self.env().var(name)
    }

    /// Forwards [`Env::vars`] to the held service; a `#[loko::wraps]` implementation may replace it.
    #[doc(hidden)]
    #[inline]
    fn wrapped_vars(&self) -> Vec<(String, String)> {
        self.env().vars()
    }
}

impl<H: HasEnv> Env for H {
    #[inline]
    fn var(&self, name: &str) -> Option<String> {
        self.wrapped_var(name)
    }

    #[inline]
    fn vars(&self) -> Vec<(String, String)> {
        self.wrapped_vars()
    }
}
