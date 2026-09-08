//! Seeded entropy for tests. Production paths must never select this.

use crate::net::qdnf::errors::QdnfError;

/// Which generator a caller asked for. `Test` exists only under `cfg(test)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropySource {
    Os,
    #[cfg(test)]
    Test,
}

/// Deterministic LCG used by harness tests. Product code must not call this.
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

/// Production entropy. Always OS bytes; never a seeded LCG.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProductionEntropy;

impl ProductionEntropy {
    pub const fn new() -> Self {
        Self
    }

    pub const fn source(self) -> EntropySource {
        EntropySource::Os
    }

    pub fn fill(&self, out: &mut [u8]) -> Result<(), QdnfError> {
        fill_os(out)
    }
}

/// Production entropy. Test code must not call this from SeededEntropy.
pub fn fill_os(out: &mut [u8]) -> Result<(), QdnfError> {
    getrandom::fill(out).map_err(|_| QdnfError::EntropyFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_entropy_is_not_used_as_os_entropy() {
        let mut seeded = SeededEntropy::new(7);
        let mut expected = [0u8; 16];
        SeededEntropy::new(7).fill(&mut expected).unwrap();

        let mut os_bytes = [0u8; 16];
        ProductionEntropy.fill(&mut os_bytes).unwrap();

        let mut from_seeded = [0u8; 16];
        seeded.fill(&mut from_seeded).unwrap();

        assert_eq!(from_seeded, expected);
        assert_ne!(os_bytes, expected);
        assert_eq!(ProductionEntropy.source(), EntropySource::Os);
        assert_eq!(EntropySource::Test, EntropySource::Test);
    }

    #[test]
    fn seeded_fill_is_repeatable() {
        let mut a = [0u8; 8];
        let mut b = [0u8; 8];
        SeededEntropy::new(99).fill(&mut a).unwrap();
        SeededEntropy::new(99).fill(&mut b).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn fill_os_does_not_advance_seeded_state() {
        let mut seeded = SeededEntropy::new(3);
        let mut before = [0u8; 4];
        SeededEntropy::new(3).fill(&mut before).unwrap();

        let mut os = [0u8; 4];
        fill_os(&mut os).unwrap();

        let mut after = [0u8; 4];
        seeded.fill(&mut after).unwrap();
        assert_eq!(after, before);
    }
}
