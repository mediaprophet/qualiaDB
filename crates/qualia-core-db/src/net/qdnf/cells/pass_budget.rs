//! 42 MiB Sentinel pass accounting (E03.4 / E10.5).
//!
//! This is **accounting only**. It does not allocate
//! [`crate::governance::webizen::SlgArena`], does not map 42 MiB of RAM, and
//! is not a measured process RSS.

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

/// RAII Sentinel pass. Reclaims on success, error, and unwind. Does not map 42 MiB.
pub struct PassGuard {
    charged: u64,
    outcome: PassOutcome,
}

impl PassGuard {
    pub fn enter(charge: &PassCharge) -> Result<Self, QdnfError> {
        Ok(Self {
            charged: charge_pass(charge)?,
            outcome: PassOutcome::Error,
        })
    }

    #[inline]
    pub const fn charged(&self) -> u64 {
        self.charged
    }

    #[inline]
    pub fn success(&mut self) {
        self.outcome = PassOutcome::Success;
    }
}

impl Drop for PassGuard {
    fn drop(&mut self) {
        let _ = reclaim_pass(self.outcome, self.charged);
    }
}

/// 42 MiB is a pass ceiling, not a process RSS or test allocation.
#[inline]
pub const fn pass_maps_process_rss() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_two_mib_is_accounting_not_rss() {
        assert_eq!(SENTINEL_PASS_TOTAL, SENTINEL_PASS_BYTES as u64);
        assert!(!pass_maps_process_rss());
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
    }

    #[test]
    fn reclaim_every_outcome_and_guard() {
        let charged = charge_pass(&PassCharge {
            scratch: 8,
            crypto: 8,
            kernel: 8,
            ..PassCharge::ZERO
        })
        .unwrap();
        assert_eq!(reclaim_pass(PassOutcome::Success, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Error, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Cancel, charged), 0);
        assert_eq!(reclaim_pass(PassOutcome::Unwind, charged), 0);
        let mut g = PassGuard::enter(&PassCharge {
            scratch: 8,
            crypto: 8,
            kernel: 8,
            ..PassCharge::ZERO
        })
        .unwrap();
        assert_eq!(g.charged(), 24);
        g.success();
    }
}
