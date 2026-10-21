use std::sync::{Mutex, PoisonError};

use rand_core::{Rng, SeedableRng};

use crate::Random;

/// Any seedable `rand_core` generator, behind a mutex so it can be shared: each
/// [`rng`](Random::rng) is a generator forked from it (`SeedableRng::fork`).
impl<R: SeedableRng + Rng> Random for Mutex<R> {
    type Rng = R;

    fn rng(&self) -> R {
        self.lock().unwrap_or_else(PoisonError::into_inner).fork()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SeededRng;

    #[test]
    fn a_shared_generator_forks() {
        let shared = Mutex::new(SeededRng::new(9));
        let (mut a, mut b) = (shared.rng(), shared.rng());
        assert_ne!(a.next_u64(), b.next_u64(), "each fork is seeded from the next numbers");
    }
}
