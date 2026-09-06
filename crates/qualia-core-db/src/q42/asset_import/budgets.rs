//! Byte / record / chunk budgets for bounded import jobs.

use super::error::ImportError;
use crate::q42::asset_envelope::SENTINEL_PASS_BUDGET_BYTES;

/// Cold-construction budgets for one import job.
///
/// `chunk_byte_budget` is the per-`feed_chunk` pass ceiling and must stay within
/// [`SENTINEL_PASS_BUDGET_BYTES`] (42 MiB). `max_bytes` / `max_records` bound the
/// whole job across chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportBudgets {
    pub max_bytes: u64,
    pub max_records: u64,
    pub chunk_byte_budget: u64,
}

impl ImportBudgets {
    /// Single-pass job capped at the Sentinel budget.
    pub fn sentinel_pass() -> Self {
        Self {
            max_bytes: SENTINEL_PASS_BUDGET_BYTES,
            max_records: u64::MAX / 4,
            chunk_byte_budget: SENTINEL_PASS_BUDGET_BYTES,
        }
    }

    /// Validate positivity and the 42 MiB chunk pass ceiling.
    pub fn validate(self) -> Result<Self, ImportError> {
        if self.max_bytes == 0 || self.max_records == 0 || self.chunk_byte_budget == 0 {
            return Err(ImportError::InvalidBudgets);
        }
        if self.chunk_byte_budget > SENTINEL_PASS_BUDGET_BYTES {
            return Err(ImportError::ChunkBudgetExceeded);
        }
        Ok(self)
    }
}
