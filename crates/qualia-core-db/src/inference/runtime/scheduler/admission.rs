//! Currency reservation and strict pre-admission gating.
//!
//! Enforces that all memory pool currencies (physical blocks, COW transients,
//! byte budgets) are fully checked and reserved before a request is admitted
//! into the scheduler or dispatched to device execution.

use crate::inference::runtime::budget::{BudgetError, MemoryPoolBudget, RequestReservation};
use crate::inference::runtime::kv::paged::{BlockPool, PoolError};

/// Error during currency reservation or admission gating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    Budget(BudgetError),
    Pool(PoolError),
    InsufficientBlocks {
        requested: u32,
        available: usize,
    },
}

impl From<BudgetError> for AdmissionError {
    fn from(err: BudgetError) -> Self {
        Self::Budget(err)
    }
}

impl From<PoolError> for AdmissionError {
    fn from(err: PoolError) -> Self {
        Self::Pool(err)
    }
}

/// A verified and committed reservation across both budget and physical block pool.
#[derive(Debug)]
pub struct AdmittedCurrencies {
    pub reservation: RequestReservation,
}

impl AdmittedCurrencies {
    /// Attempt to reserve all required currencies for a request.
    ///
    /// Evaluates:
    /// 1. Byte budget and COW headroom via `MemoryPoolBudget::reserve`.
    /// 2. Physical block availability in `BlockPool`.
    ///
    /// Fails closed: if either check fails, no allocations are performed or retained.
    pub fn reserve(
        budget: &MemoryPoolBudget,
        pool: &BlockPool,
        input_tokens: u32,
        reserved_output_tokens: u32,
        cow_blocks: u32,
        active_reserved_bytes: u64,
    ) -> Result<Self, AdmissionError> {
        // Step 1: Byte budget and arithmetic validation
        let reservation = budget.reserve(
            input_tokens,
            reserved_output_tokens,
            cow_blocks,
            active_reserved_bytes,
        )?;

        // Step 2: Physical block availability check
        let total_blocks_needed = reservation
            .required_blocks
            .checked_add(reservation.cow_transient_blocks)
            .ok_or(AdmissionError::Budget(BudgetError::IntegerOverflow))?;

        if (total_blocks_needed as usize) > pool.free_count() {
            return Err(AdmissionError::InsufficientBlocks {
                requested: total_blocks_needed,
                available: pool.free_count(),
            });
        }

        Ok(Self { reservation })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::runtime::budget::{BlockGeometry, ModelMemoryProfile};

    fn fixture_budget() -> MemoryPoolBudget {
        let profile = ModelMemoryProfile::new(100, 100, 100, 100).unwrap();
        let geometry = BlockGeometry::new(16, 64).unwrap();
        MemoryPoolBudget::new(2000, profile, geometry, 20, 4).unwrap()
    }

    #[test]
    fn test_currency_reservation_success() {
        let budget = fixture_budget();
        let pool = BlockPool::new(20);

        // 32 tokens = 2 blocks, plus 1 COW block = 3 blocks needed.
        let currencies = AdmittedCurrencies::reserve(&budget, &pool, 16, 16, 1, 0);
        assert!(currencies.is_ok());
        let c = currencies.unwrap();
        assert_eq!(c.reservation.required_blocks, 2);
        assert_eq!(c.reservation.cow_transient_blocks, 1);
    }

    #[test]
    fn test_currency_reservation_insufficient_physical_blocks() {
        let budget = fixture_budget();
        let pool = BlockPool::new(2); // Only 2 free blocks

        // 32 tokens = 2 blocks, plus 1 COW block = 3 blocks needed > 2 free
        let err = AdmittedCurrencies::reserve(&budget, &pool, 16, 16, 1, 0);
        assert!(matches!(
            err,
            Err(AdmissionError::InsufficientBlocks {
                requested: 3,
                available: 2
            })
        ));
    }
}
