//! Pre-authentication amplification budget.
//!
//! Unauthenticated discovery and handshake bytes/work are charged **before**
//! neighbor state or reassembly buffers are occupied. Excess is
//! [`QdnfError::BudgetExhausted`], not a larger table.

use crate::net::qdnf::errors::QdnfError;

/// Default unauthenticated byte cap (a few beacons / cookies, not a flood).
pub const MAX_UNAUTH_BYTES: u64 = 4096;
/// Default unauthenticated work units (one HMAC / beacon decode each).
pub const MAX_UNAUTH_WORK: u64 = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreAuthBudget {
    cap_bytes: u64,
    cap_work: u64,
    used_bytes: u64,
    used_work: u64,
}

impl PreAuthBudget {
    pub const fn new() -> Self {
        Self::with_caps(MAX_UNAUTH_BYTES, MAX_UNAUTH_WORK)
    }

    pub const fn with_caps(cap_bytes: u64, cap_work: u64) -> Self {
        Self {
            cap_bytes,
            cap_work,
            used_bytes: 0,
            used_work: 0,
        }
    }

    #[inline]
    pub const fn used_bytes(self) -> u64 {
        self.used_bytes
    }

    #[inline]
    pub const fn used_work(self) -> u64 {
        self.used_work
    }

    #[inline]
    pub const fn remaining_bytes(self) -> u64 {
        self.cap_bytes.saturating_sub(self.used_bytes)
    }

    /// Charge unauthenticated bytes and work. On failure the counters are
    /// unchanged (no partial charge of the overflowing request).
    pub fn charge(&mut self, bytes: u64, work: u64) -> Result<(), QdnfError> {
        let new_bytes = self.used_bytes.checked_add(bytes).ok_or(QdnfError::Range)?;
        let new_work = self.used_work.checked_add(work).ok_or(QdnfError::Range)?;
        if new_bytes > self.cap_bytes || new_work > self.cap_work {
            return Err(QdnfError::BudgetExhausted);
        }
        self.used_bytes = new_bytes;
        self.used_work = new_work;
        Ok(())
    }
}

impl Default for PreAuthBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// Admit an unauthenticated frame: charge first, then the caller may copy.
pub fn admit_unauthenticated(
    budget: &mut PreAuthBudget,
    bytes: u64,
    work: u64,
) -> Result<(), QdnfError> {
    budget.charge(bytes, work)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplification_budget_exhausts_without_partial_charge() {
        let mut budget = PreAuthBudget::with_caps(64, 4);
        assert_eq!(admit_unauthenticated(&mut budget, 64, 1), Ok(()));
        assert_eq!(budget.used_bytes(), 64);
        assert_eq!(
            admit_unauthenticated(&mut budget, 1, 0),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(budget.used_bytes(), 64);
        assert_eq!(budget.remaining_bytes(), 0);
    }

    #[test]
    fn work_cap_exhausts_independently() {
        let mut budget = PreAuthBudget::with_caps(4096, 2);
        budget.charge(10, 2).unwrap();
        assert_eq!(budget.charge(10, 1), Err(QdnfError::BudgetExhausted));
        assert_eq!(budget.used_work(), 2);
        assert_eq!(budget.used_bytes(), 10);
    }

    #[test]
    fn overflow_is_range_not_wrap() {
        let mut budget = PreAuthBudget::with_caps(u64::MAX, u64::MAX);
        budget.charge(1, 0).unwrap();
        assert_eq!(budget.charge(u64::MAX, 0), Err(QdnfError::Range));
        assert_eq!(budget.used_bytes(), 1);
    }
}
