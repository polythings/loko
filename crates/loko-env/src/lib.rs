//! loko's `Env` resource: environment variables, passed to code instead of read from the process, so
//! tests set their own and code in a browser (which has none) gets them from a map.
//!
//! - [`Env`]: `var` and `vars`.
//! - [`SystemEnv`]: the process's variables.
//! - [`MapEnv`]: variables in a map, for tests and for hosts without a process environment.
//!
//! ```
//! use loko_env::{Env, MapEnv};
//!
//! let env = MapEnv::new().with("REGION", "us-east-2");
//! assert_eq!(env.var("REGION").as_deref(), Some("us-east-2"));
//! assert_eq!(env.var("MISSING"), None);
//! ```

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Environment variables.
#[loko_macros::resource]
pub trait Env {
    /// The variable `name`, or `None` if it isn't set (or isn't valid Unicode).
    fn var(&self, name: &str) -> Option<String>;

    /// Every variable, as `(name, value)` pairs.
    fn vars(&self) -> Vec<(String, String)>;
}

/// The process's environment variables (`std::env`). On targets without one
/// (`wasm32-unknown-unknown`) nothing is set.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEnv;

impl Env for SystemEnv {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn vars(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
}

/// Environment variables in a map. Clones share the map.
#[derive(Clone, Debug, Default)]
pub struct MapEnv {
    vars: Arc<Mutex<BTreeMap<String, String>>>,
}

impl MapEnv {
    /// No variables.
    pub fn new() -> Self {
        Self::default()
    }

    /// These variables and `name` set to `value`.
    #[must_use]
    pub fn with(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.set(name, value);
        self
    }

    /// Sets `name` to `value`.
    pub fn set(&self, name: impl Into<String>, value: impl Into<String>) {
        self.map().insert(name.into(), value.into());
    }

    /// Unsets `name`.
    pub fn remove(&self, name: &str) {
        self.map().remove(name);
    }

    fn map(&self) -> MutexGuard<'_, BTreeMap<String, String>> {
        self.vars.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for MapEnv {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let env = Self::new();
        for (k, v) in iter {
            env.set(k, v);
        }
        env
    }
}

impl Env for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.map().get(name).cloned()
    }

    fn vars(&self) -> Vec<(String, String)> {
        self.map().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_envs_set_and_unset() {
        let env: MapEnv = [("A", "1"), ("B", "2")].into_iter().collect();
        let shared = env.clone();
        shared.set("C", "3");
        shared.remove("A");
        assert_eq!(env.vars(), [("B".to_string(), "2".to_string()), ("C".to_string(), "3".to_string())]);
    }

    #[test]
    fn the_system_env_reads_the_process() {
        // PATH is set wherever these tests run.
        assert!(SystemEnv.var("PATH").is_some());
        assert!(SystemEnv.vars().iter().any(|(k, _)| k == "PATH"));
    }
}
