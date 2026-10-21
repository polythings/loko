use std::convert::Infallible;

use rand_core::{TryCryptoRng, TryRng};

use crate::Random;

/// The operating system's random number generator (feature `getrandom`, native targets and WASI).
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRandom;

impl Random for OsRandom {
    type Rng = OsRng;

    fn rng(&self) -> OsRng {
        OsRng
    }
}

/// Numbers straight from the operating system's generator; fit for secrets.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRng;

// The operating system's generator only fails when it isn't available at all.
const UNAVAILABLE: &str = "the operating system's random number generator";

impl TryRng for OsRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(getrandom::u32().expect(UNAVAILABLE))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(getrandom::u64().expect(UNAVAILABLE))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        getrandom::fill(dst).expect(UNAVAILABLE);
        Ok(())
    }
}

impl TryCryptoRng for OsRng {}

#[cfg(test)]
mod tests {
    use rand_core::{CryptoRng, Rng};

    use super::*;

    fn token(random: &impl Random<Rng: CryptoRng>) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        random.rng().fill_bytes(&mut bytes);
        bytes
    }

    #[test]
    fn os_numbers_differ_and_are_fit_for_secrets() {
        let mut rng = OsRandom.rng();
        assert_ne!(rng.next_u64(), rng.next_u64());
        assert_ne!(token(&OsRandom), token(&OsRandom));
    }
}
