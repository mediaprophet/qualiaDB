//! Joint memory budget planning and checked arithmetic.
//!
//! Provides explicit accounting for static model footprints, hardware runtime context,
//! static graph capture headroom, physical page block geometry, and admission reservations
//! (including input, reserved output, COW transients, and page boundary fragmentation).

pub mod model;
pub mod pools;
pub mod reservation;

#[cfg(test)]
mod tests;

pub use model::ModelMemoryProfile;
pub use pools::{BlockGeometry, MemoryPoolBudget};
pub use reservation::RequestReservation;

/// Errors returned during budget computation and admission reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BudgetError {
    /// Arithmetic overflow during token, block, or byte calculations.
    IntegerOverflow,
    /// Invalid block geometry (zero tokens per block or zero bytes per block).
    InvalidGeometry,
    /// Insufficient available memory for static or dynamic reservation.
    InsufficientBudget {
        required_bytes: u64,
        available_bytes: u64,
    },
    /// Requested block count exceeds physical pool capacity.
    PoolExhausted {
        requested_blocks: u32,
        max_blocks: u32,
    },
    /// Requested copy-on-write transient headroom exceeds configured limit.
    InsufficientCowHeadroom {
        requested_blocks: u32,
        max_cow_blocks: u32,
    },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IntegerOverflow => write!(f, "arithmetic overflow in memory budget calculation"),
            Self::InvalidGeometry => {
                write!(f, "invalid block geometry: parameters must be non-zero")
            }
            Self::InsufficientBudget {
                required_bytes,
                available_bytes,
            } => write!(
                f,
                "insufficient memory budget: required {} bytes, but only {} bytes available",
                required_bytes, available_bytes
            ),
            Self::PoolExhausted {
                requested_blocks,
                max_blocks,
            } => write!(
                f,
                "physical block pool exhausted: requested {} blocks, capacity is {}",
                requested_blocks, max_blocks
            ),
            Self::InsufficientCowHeadroom {
                requested_blocks,
                max_cow_blocks,
            } => write!(
                f,
                "insufficient COW transient headroom: requested {} blocks, limit is {}",
                requested_blocks, max_cow_blocks
            ),
        }
    }
}

impl std::error::Error for BudgetError {}
