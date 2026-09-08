//! Seeded entropy for tests. Production paths must never select this.

use crate::net::qdnf::errors::QdnfError;

pub struct SeededEntropy {
    state: u64,
}

impl SeededEntropy {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    pub fn fill(&mut self, out: &mut [u8]) -> Result<(), QdnfError> {
        for byte in out.iter_mut() {
            self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (self.state >> 32) as u8;
        }
        Ok(())
    }
}

/// Production entropy. Test code must not call this from SeededEntropy.
pub fn fill_os(out: &mut [u8]) -> Result<(), QdnfError> {
    getrandom::fill(out).map_err(|_| QdnfError::EntropyFailure)
}
