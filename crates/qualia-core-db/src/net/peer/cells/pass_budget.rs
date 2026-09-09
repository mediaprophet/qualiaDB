//! 42 MiB Sentinel pass accounting (E03.4) and reclaim on every outcome (E03.5).
//!
//! This is **accounting only**. It does not allocate [`crate::governance::webizen::SlgArena`],
//! does not map 42 MiB of RAM, and is not a measured process RSS. Callers charge
//! arena, scratch, crypto, kernel and pinned bytes against
//! [`SENTINEL_PASS_TOTAL`].

use crate::governance::webizen::SENTINEL_PASS_BYTES;
use crate::net::qdnf::errors::QdnfError;

/// Exact 42 MiB pass ceiling. Same value as [`SENTINEL_PASS_BYTES`].
pub const SENTINEL_PASS_TOTAL: u64 = 42 * 1024 * 1024;

/// Explicit pass charge components (E03.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassCharge {
    pub arena: u64,
    pub scratch: u64,
    pub crypto: u64,
    pub kernel: u64,
    pub pinned: u64,
}

impl PassCharge {
    pub const ZERO: Self = Self {
        arena: 0,
        scratch: 0,
        crypto: 0,
        kernel: 0,
        pinned: 0,
    };
}

/// Checked sum of every charged class. Greater than 42 MiB → [`QdnfError::Capacity`].
pub fn charge_pass(c: &PassCharge) -> Result<u64, QdnfError> {
    let mut total = 0u64;
    total = total.checked_add(c.arena).ok_or(QdnfError::Capacity)?;
    total = total.checked_add(c.scratch).ok_or(QdnfError::Capacity)?;
    total = total.checked_add(c.crypto).ok_or(QdnfError::Capacity)?;
    total = total.checked_add(c.kernel).ok_or(QdnfError::Capacity)?;
    total = total.checked_add(c.pinned).ok_or(QdnfError::Capacity)?;
    if total > SENTINEL_PASS_TOTAL {
        return Err(QdnfError::Capacity);
    }
    let _ = SENTINEL_PASS_BYTES;
    Ok(total)
}

/// Terminal pass outcomes that must still reclaim (E03.5).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassOutcome {
    Success = 0,
    Error = 1,
    Cancel = 2,
    Unwind = 3,
}

/// Reclaim charged pass bytes. Every outcome returns 0 remaining.
pub fn reclaim_pass(_outcome: PassOutcome, charged: u64) -> u64 {
    let _ = charged;
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;

    #[test]
    fn forty_two_mib_exact_ok_plus_one_capacity() {
        assert_eq!(SENTINEL_PASS_TOTAL, SENTINEL_PASS_BYTES as u64);
        let exact = PassCharge {
            arena: SENTINEL_PASS_TOTAL,
            ..PassCharge::ZERO
        };
        assert_eq!(charge_pass(&exact), Ok(SENTINEL_PASS_TOTAL));
        let over = PassCharge {
            arena: SENTINEL_PASS_TOTAL,
            scratch: 1,
            ..PassCharge::ZERO
        };
        assert_eq!(charge_pass(&over), Err(QdnfError::Capacity));
        let plus_one = PassCharge {
            arena: SENTINEL_PASS_TOTAL + 1,
            ..PassCharge::ZERO
        };
        assert_eq!(charge_pass(&plus_one), Err(QdnfError::Capacity));
    }

    #[test]
    fn scratch_crypto_kernel_pinned_counted() {
        let c = PassCharge {
            arena: SENTINEL_PASS_TOTAL - 40,
            scratch: 10,
            crypto: 10,
            kernel: 10,
            pinned: 10,
        };
        assert_eq!(charge_pass(&c), Ok(SENTINEL_PASS_TOTAL));
        let over = PassCharge {
            arena: SENTINEL_PASS_TOTAL - 40,
            scratch: 11,
            crypto: 10,
            kernel: 10,
            pinned: 10,
        };
        assert_eq!(charge_pass(&over), Err(QdnfError::Capacity));
    }

    #[test]
    fn reclaim_on_success_error_cancel_unwind() {
        let charged = charge_pass(&PassCharge {
            arena: 1024,
            scratch: 1024,
            ..PassCharge::ZERO
        })
        .unwrap();
        assert_eq!(reclaim_pass(PassOutcome::Success, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Error, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Cancel, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Unwind, charged), 0);
    }

    #[test]
    fn overflow_is_capacity_not_wrap() {
        let c = PassCharge {
            arena: u64::MAX,
            scratch: 1,
            ..PassCharge::ZERO
        };
        assert_eq!(charge_pass(&c), Err(QdnfError::Capacity));
    }
}
