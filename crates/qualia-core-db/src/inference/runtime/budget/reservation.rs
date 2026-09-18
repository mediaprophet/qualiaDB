//! Checked byte arithmetic and admission reservation.
//!
//! Performs exact-fit, one-byte-over, overflow-safe admission calculations,
//! reserving both input and output tokens, COW transients, and page fragmentation headroom.

use super::pools::MemoryPoolBudget;
use super::BudgetError;

/// An admitted request's checked memory reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestReservation {
    /// Number of prompt tokens admitted.
    pub input_tokens: u32,
    /// Number of output tokens strictly reserved for autoregressive generation.
    pub reserved_output_tokens: u32,
    /// Physical blocks required to house (input + output) tokens.
    pub required_blocks: u32,
    /// Transient blocks reserved for copy-on-write page forks.
    pub cow_transient_blocks: u32,
    /// Total bytes reserved on the device for this request.
    pub reserved_bytes: u64,
}

impl MemoryPoolBudget {
    /// Check whether a request can be admitted within the pool budget.
    ///
    /// Evaluates:
    /// 1. `input_tokens + reserved_output_tokens` (checked arithmetic)
    /// 2. Ceiling block conversion via block geometry (fragmented page boundary)
    /// 3. COW transient headroom allocation
    /// 4. Global physical block limit
    /// 5. Global dynamic KV byte ceiling
    /// Check whether a request can be admitted within the pool budget without active COW tracking.
    pub fn reserve(
        &self,
        input_tokens: u32,
        reserved_output_tokens: u32,
        cow_blocks: u32,
        active_reserved_bytes: u64,
    ) -> Result<RequestReservation, BudgetError> {
        self.reserve_with_active_cow(
            input_tokens,
            reserved_output_tokens,
            cow_blocks,
            0,
            active_reserved_bytes,
        )
    }

    /// Check whether a request can be admitted within the pool budget with cumulative active COW headroom tracking.
    pub fn reserve_with_active_cow(
        &self,
        input_tokens: u32,
        reserved_output_tokens: u32,
        cow_blocks: u32,
        active_cow_blocks: u32,
        active_reserved_bytes: u64,
    ) -> Result<RequestReservation, BudgetError> {
        let total_tokens = input_tokens
            .checked_add(reserved_output_tokens)
            .ok_or(BudgetError::IntegerOverflow)?;

        let required_blocks = self.geometry.blocks_for_tokens(total_tokens)?;

        let total_blocks = required_blocks
            .checked_add(cow_blocks)
            .ok_or(BudgetError::IntegerOverflow)?;

        if total_blocks > self.max_physical_blocks {
            return Err(BudgetError::PoolExhausted {
                requested_blocks: total_blocks,
                max_blocks: self.max_physical_blocks,
            });
        }

        let total_cow = active_cow_blocks
            .checked_add(cow_blocks)
            .ok_or(BudgetError::IntegerOverflow)?;

        if total_cow > self.max_cow_transient_blocks {
            return Err(BudgetError::InsufficientCowHeadroom {
                requested_blocks: total_cow,
                max_cow_blocks: self.max_cow_transient_blocks,
            });
        }

        let needed_bytes = self.geometry.bytes_for_blocks(total_blocks)?;
        let total_reserved = active_reserved_bytes
            .checked_add(needed_bytes)
            .ok_or(BudgetError::IntegerOverflow)?;

        let available = self.available_kv_bytes();
        if total_reserved > available {
            return Err(BudgetError::InsufficientBudget {
                required_bytes: total_reserved,
                available_bytes: available,
            });
        }

        Ok(RequestReservation {
            input_tokens,
            reserved_output_tokens,
            required_blocks,
            cow_transient_blocks: cow_blocks,
            reserved_bytes: needed_bytes,
        })
    }
}
