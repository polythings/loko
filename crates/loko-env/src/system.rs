use crate::Env;

/// The process's environment variables (`std::env`), on native targets and WASI. Variables whose
/// name or value isn't valid Unicode are left out, as `var` leaves them out.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEnv;

impl Env for SystemEnv {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn vars(&self) -> Vec<(String, String)> {
        // `std::env::vars` panics on the first variable that isn't Unicode.
        std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_env_reads_the_process() {
        // PATH is set wherever these tests run.
        assert!(SystemEnv.var("PATH").is_some());
        assert!(SystemEnv.vars().iter().any(|(k, _)| k == "PATH"));
    }
}
