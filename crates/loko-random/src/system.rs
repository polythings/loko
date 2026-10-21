use crate::Random;

/// The operating system's random number generator (feature `getrandom`, native targets and WASI).
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRandom;

impl Random for OsRandom {
    fn next_u64(&self) -> u64 {
        // The operating system's generator only fails when it isn't available at all.
        getrandom::u64().expect("the operating system's random number generator")
    }

    fn fill(&self, dest: &mut [u8]) {
        getrandom::fill(dest).expect("the operating system's random number generator");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_random_numbers_differ() {
        assert_ne!(OsRandom.next_u64(), OsRandom.next_u64());
        let mut bytes = [0u8; 32];
        OsRandom.fill(&mut bytes);
        assert!(bytes.iter().any(|&b| b != 0));
    }
}
