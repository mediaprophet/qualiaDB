//! Staging admission shares the existing quota's root, usage and attempt slots.
use super::{CustodyQuota, QuotaError};
use std::path::Path;

/// Consumed once by a staging backend. This is accounting, never disclosure or
/// durable-presence authority. Dropping it does not refund the canonical quota.
#[derive(Debug)]
pub struct StagingCharge<'root> {
    root: &'root Path,
    attempt: [u8; 32],
    allowance: u64,
}
impl<'root> StagingCharge<'root> {
    pub const fn attempt_id(&self) -> [u8; 32] {
        self.attempt
    }
    pub const fn allowance(&self) -> u64 {
        self.allowance
    }
    pub(crate) const fn root(&self) -> &'root Path {
        self.root
    }
}
impl<'root, const N: usize> CustodyQuota<'root, N> {
    /// Charges the full explicit allowance before any staging filesystem work.
    /// The returned token borrows the root, not this mutable ledger.
    pub fn admit_staging(
        &mut self,
        attempt: [u8; 32],
        allowance: u64,
    ) -> Result<StagingCharge<'root>, QuotaError> {
        if attempt == [0; 32] || allowance == 0 {
            return Err(QuotaError::InvalidLimits);
        }
        super::validate_root(self.root).map_err(|_| QuotaError::InvalidScope)?;
        self.charge_u64(self.root, attempt, allowance)?;
        Ok(StagingCharge {
            root: self.root,
            attempt,
            allowance,
        })
    }
}
