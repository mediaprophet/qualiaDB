//! Capacity and work admission for caller-buffered cold production passes.
//!
//! This ledger does not intercept allocations. A qualified caller enumerates
//! every live input, scratch and output buffer, uses only those buffers during
//! execution, and charges work before each bounded unit. Legacy allocating
//! algorithms are not made bounded by wrapping them in this ledger. Typed slice
//! capacities are exact; arena padding uses a conservative upper bound. Count
//! each backing allocation once, not both its storage and borrowed subslices.

use super::{Cancellation, ARENA_ALIGN, DEFAULT_COLD_GEOMETRY_BUDGET};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferClass { Input, Scratch, Output }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    ArithmeticOverflow,
    ByteLimitExceeded { required: usize, limit: usize },
    WorkLimitExceeded { requested: u64, remaining: u64 },
    Cancelled,
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArithmeticOverflow => f.write_str("production capacity arithmetic overflow"),
            Self::ByteLimitExceeded { required, limit } => write!(f, "production bytes {required} exceed limit {limit}"),
            Self::WorkLimitExceeded { requested, remaining } => write!(f, "production work {requested} exceeds remaining {remaining}"),
            Self::Cancelled => f.write_str("production pass cancelled"),
        }
    }
}
impl std::error::Error for BudgetError {}

/// Simultaneously live capacities, including caller output capacity rather
/// than merely the final populated prefix. Fields cannot be silently changed
/// after admission. Reused scratch is counted once at its maximum live size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapacityPlan {
    input_bytes: usize,
    scratch_bytes: usize,
    output_bytes: usize,
}

impl CapacityPlan {
    pub fn input_bytes(&self) -> usize { self.input_bytes }
    pub fn scratch_bytes(&self) -> usize { self.scratch_bytes }
    pub fn output_bytes(&self) -> usize { self.output_bytes }

    pub fn total_bytes(&self) -> Result<usize, BudgetError> {
        self.input_bytes.checked_add(self.scratch_bytes)
            .and_then(|n| n.checked_add(self.output_bytes))
            .ok_or(BudgetError::ArithmeticOverflow)
    }

    /// Failed additions leave the original plan intact.
    pub fn add_bytes(&mut self, class: BufferClass, bytes: usize) -> Result<(), BudgetError> {
        let mut next = *self;
        let slot = match class {
            BufferClass::Input => &mut next.input_bytes,
            BufferClass::Scratch => &mut next.scratch_bytes,
            BufferClass::Output => &mut next.output_bytes,
        };
        *slot = slot.checked_add(bytes).ok_or(BudgetError::ArithmeticOverflow)?;
        next.total_bytes()?;
        *self = next;
        Ok(())
    }

    pub fn add_slice<T>(&mut self, class: BufferClass, capacity: usize) -> Result<(), BudgetError> {
        let bytes = capacity.checked_mul(std::mem::size_of::<T>())
            .ok_or(BudgetError::ArithmeticOverflow)?;
        self.add_bytes(class, bytes)
    }

    /// Accounts for the arena's physical alignment, even for an unaligned byte
    /// buffer. Use once per allocation. Zero-sized allocations need no padding.
    pub fn add_arena_slice<T>(&mut self, capacity: usize) -> Result<(), BudgetError> {
        let bytes = capacity.checked_mul(std::mem::size_of::<T>())
            .ok_or(BudgetError::ArithmeticOverflow)?;
        let padded = if bytes == 0 { 0 } else {
            bytes.checked_add(std::mem::align_of::<T>().max(ARENA_ALIGN) - 1)
                .ok_or(BudgetError::ArithmeticOverflow)?
        };
        self.add_bytes(BufferClass::Scratch, padded)
    }
}

/// Zero-allocation admission/work counter sharing the existing cancellation
/// token. Do not clone the token: Cancellation::clone snapshots its state.
pub struct ProductionBudget<'a> {
    plan: CapacityPlan,
    byte_limit: usize,
    work_limit: u64,
    work_used: u64,
    cancel: &'a Cancellation,
}

impl<'a> ProductionBudget<'a> {
    pub fn admit(plan: CapacityPlan, byte_limit: usize, work_limit: u64, cancel: &'a Cancellation) -> Result<Self, BudgetError> {
        if cancel.is_cancelled() { return Err(BudgetError::Cancelled); }
        let required = plan.total_bytes()?;
        if required > byte_limit {
            return Err(BudgetError::ByteLimitExceeded { required, limit: byte_limit });
        }
        Ok(Self { plan, byte_limit, work_limit, work_used: 0, cancel })
    }

    pub fn admit_default(plan: CapacityPlan, work_limit: u64, cancel: &'a Cancellation) -> Result<Self, BudgetError> {
        Self::admit(plan, DEFAULT_COLD_GEOMETRY_BUDGET, work_limit, cancel)
    }

    /// Check cancellation and reserve work before doing it. Rejection does not
    /// consume work. Call with zero to check cancellation at publication.
    /// A checked atomic mesh/transfer/event commit contains no checkpoints;
    /// observe cancellation again only after its complete committed prefix.
    pub fn checkpoint(&mut self, work_items: u64) -> Result<(), BudgetError> {
        if self.cancel.is_cancelled() { return Err(BudgetError::Cancelled); }
        let remaining = self.work_remaining();
        if work_items > remaining {
            return Err(BudgetError::WorkLimitExceeded { requested: work_items, remaining });
        }
        self.work_used += work_items; // bounded by work_limit <= u64::MAX
        Ok(())
    }

    pub fn work_used(&self) -> u64 { self.work_used }
    pub fn work_remaining(&self) -> u64 { self.work_limit - self.work_used }
    pub fn byte_limit(&self) -> usize { self.byte_limit }
    pub fn capacity_plan(&self) -> CapacityPlan { self.plan }
}

#[cfg(test)]
#[path = "production_budget_tests.rs"]
mod tests;
