//! loko's `Env` resource: environment variables, passed to code instead of read from the process, so
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
#[loko_macros::resource]
pub trait Env {
    /// The variable `name`, or `None` if it isn't set (or isn't valid Unicode).
    fn var(&self, name: &str) -> Option<String>;

    /// Every variable, as `(name, value)` pairs.
    fn vars(&self) -> Vec<(String, String)>;
}
