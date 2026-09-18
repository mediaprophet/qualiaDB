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
    pub allocated_blocks: Vec<u32>,
}

impl AdmittedCurrencies {
    /// Attempt to reserve all required currencies for a request.
    ///
    /// Evaluates:
    /// 1. Byte budget and cumulative COW headroom via `MemoryPoolBudget::reserve_with_active_cow`.
    /// 2. Physical block availability in `BlockPool`.
    /// 3. Atomically allocates blocks from `BlockPool` into an active reservation ledger.
    ///
    /// Fails closed: if any check or allocation fails, all allocated blocks are rolled back.
    pub fn reserve(
        budget: &MemoryPoolBudget,
        pool: &mut BlockPool,
        input_tokens: u32,
        reserved_output_tokens: u32,
        cow_blocks: u32,
        active_cow_blocks: u32,
        active_reserved_bytes: u64,
    ) -> Result<Self, AdmissionError> {
        // Step 1: Byte budget and cumulative COW headroom validation
        let reservation = budget.reserve_with_active_cow(
            input_tokens,
            reserved_output_tokens,
            cow_blocks,
            active_cow_blocks,
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

        // Step 3: Atomic physical allocation
        let mut allocated_blocks = Vec::with_capacity(total_blocks_needed as usize);
        for _ in 0..total_blocks_needed {
            match pool.allocate() {
                Ok(block) => allocated_blocks.push(block),
                Err(err) => {
                    // Rollback on partial failure
                    for b in allocated_blocks {
                        let _ = pool.release(b);
                    }
                    return Err(AdmissionError::Pool(err));
                }
            }
        }

        Ok(Self {
            reservation,
            allocated_blocks,
        })
    }

    /// Explicitly release all reserved physical blocks back to the pool.
    pub fn release(&mut self, pool: &mut BlockPool) {
        for block in self.allocated_blocks.drain(..) {
            let _ = pool.release(block);
        }
    }

    pub fn allocated_blocks(&self) -> &[u32] {
        &self.allocated_blocks
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
    fn test_currency_reservation_success_and_release() {
        let budget = fixture_budget();
        let mut pool = BlockPool::new(20);

        // 32 tokens = 2 blocks, plus 1 COW block = 3 blocks needed.
        let currencies = AdmittedCurrencies::reserve(&budget, &mut pool, 16, 16, 1, 0, 0);
        assert!(currencies.is_ok());
        let mut c = currencies.unwrap();
        assert_eq!(c.reservation.required_blocks, 2);
        assert_eq!(c.reservation.cow_transient_blocks, 1);
        assert_eq!(c.allocated_blocks.len(), 3);
        assert_eq!(pool.free_count(), 17);

        // Explicit release restores pool
        c.release(&mut pool);
        assert_eq!(pool.free_count(), 20);
        assert!(c.allocated_blocks.is_empty());
    }

    #[test]
    fn test_currency_reservation_insufficient_physical_blocks() {
        let budget = fixture_budget();
        let mut pool = BlockPool::new(2); // Only 2 free blocks

        // 32 tokens = 2 blocks, plus 1 COW block = 3 blocks needed > 2 free
        let err = AdmittedCurrencies::reserve(&budget, &mut pool, 16, 16, 1, 0, 0);
        assert!(matches!(
            err,
            Err(AdmissionError::InsufficientBlocks {
                requested: 3,
                available: 2
            })
        ));
        // Ensure no blocks were leaked
        assert_eq!(pool.free_count(), 2);
    }

    #[test]
    fn test_currency_reservation_cumulative_cow_headroom_exceeded() {
        let budget = fixture_budget();
        let mut pool = BlockPool::new(20);

        // Max COW blocks is 4. If 3 active, requesting 2 more = 5 > 4 -> rejected.
        let err = AdmittedCurrencies::reserve(&budget, &mut pool, 16, 16, 2, 3, 0);
        assert!(matches!(
            err,
            Err(AdmissionError::Budget(BudgetError::InsufficientCowHeadroom {
                requested_blocks: 5,
                max_cow_blocks: 4
            }))
        ));
        assert_eq!(pool.free_count(), 20);
    }
}
