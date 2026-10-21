use std::sync::{Mutex, PoisonError};

use crate::Random;

/// Any `rand_core` generator, behind a mutex so it can be shared (feature `rand`).
impl<R: rand_core::Rng> Random for Mutex<R> {
    fn next_u64(&self) -> u64 {
        self.lock().unwrap_or_else(PoisonError::into_inner).next_u64()
    }

    fn fill(&self, dest: &mut [u8]) {
        self.lock().unwrap_or_else(PoisonError::into_inner).fill_bytes(dest);
    }
}
