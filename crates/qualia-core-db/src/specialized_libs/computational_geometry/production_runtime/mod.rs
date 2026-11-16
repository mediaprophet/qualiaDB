//! Bounded production adapters over the existing CDAG meshlet ABI.
//! Replacement coverage is an external typed dependency, not a new CDAG field.
//! This v1 runtime admits trees/forests, a strict subset of DAG hierarchies.

mod hierarchy;
mod selection;
mod residency;
mod draw;
mod activation;

pub use activation::*;
pub use draw::*;
pub use hierarchy::*;
pub use residency::*;
pub use selection::*;

use super::geometry_workspace::production_budget::BudgetError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidHierarchy,
    InvalidBounds,
    InvalidView,
    InvalidCut,
    Capacity,
    MissingFallback,
    StaleIdentity,
    DeviceLost,
    Unsupported,
    Budget(BudgetError),
}

impl From<BudgetError> for RuntimeError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidHierarchy => f.write_str("invalid replacement hierarchy"),
            Self::InvalidBounds => f.write_str("invalid cluster bounds"),
            Self::InvalidView => f.write_str("invalid view or projection"),
            Self::InvalidCut => f.write_str("cut is not a contiguous coverage partition"),
            Self::Capacity => f.write_str("caller buffer or dispatch capacity exceeded"),
            Self::MissingFallback => f.write_str("required fallback is not resident or complete"),
            Self::StaleIdentity => f.write_str("stale or mixed-version identity"),
            Self::DeviceLost => f.write_str("device lost around activation"),
            Self::Unsupported => f.write_str("unsupported runtime request"),
            Self::Budget(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests;
