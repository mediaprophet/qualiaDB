//! Caller-buffered typed transfer operators for explicit, protected domains.
//!
//! Sparse maps are external typed resources, never FEMC or a new `.10d`
//! section. FEMC remains exclusively surface-to-tetrahedral correspondence.
//! All floating arithmetic is deterministic serial f64, except existing f32
//! FEMC/SKL/frame codecs. Callers record units and frames in TransferContract.
//! Errors preserve published outputs; scratch contents are disposable.
//! General signed/extrapolating, nonrigid frame, >4 skin influence and smooth
//! cross-face tangent reconstruction are deliberately unsupported.

mod sparse;
mod frames;
mod corner;
mod skin;
mod femc;
mod collapse;
mod provenance;
mod resource;

pub use sparse::*;
pub use frames::*;
pub use corner::*;

pub use femc::*;
pub use collapse::*;
pub use provenance::*;
pub use resource::*;

pub(crate) use super::geometry_workspace::production_budget::{BudgetError, ProductionBudget};
pub(crate) use super::production_contracts::{ContentId, ContractError, ReasonCode};

/// Exact equivalence class for every protected support restriction. Chart is
/// meaningful for corners; use zero only when chart restriction is inapplicable.
/// Additional protected labels must be hashed canonically into `protected`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProtectedDomain {
    pub material: u64,
    pub subpart: u64,
    pub provenance: u64,
    pub chart: u64,
    pub protected: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferIdentities {
    pub source: ContentId,
    pub target: ContentId,
    pub source_frame: ContentId,
    pub target_frame: ContentId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferError {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    Capacity { required: usize, available: usize },
    DomainCrossing { row: usize, col: usize },
    IdentityMismatch,
    Budget(BudgetError),
    Codec,
}
impl From<BudgetError> for TransferError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}
impl core::fmt::Display for TransferError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TransferError {}
impl TransferError {
    pub fn contract(self, field: &'static str) -> ContractError {
        let code = match self {
            Self::Unsupported("collapse_affine_reproduction") | Self::Unsupported("affine_reproduction") => {
                ReasonCode::UnsatisfiedConstraint
            }
            Self::Unsupported(_) => ReasonCode::UnsupportedCapability,
            Self::Capacity { .. } => ReasonCode::CapacityExhausted,
            Self::Budget(BudgetError::Cancelled) => ReasonCode::Cancelled,
            Self::Budget(_) => ReasonCode::CapacityExhausted,
            Self::DomainCrossing { .. } => ReasonCode::UnsatisfiedConstraint,
            Self::IdentityMismatch => ReasonCode::StaleIdentity,
            _ => ReasonCode::InvalidInput,
        };
        ContractError::new(code, field)
    }
}
impl From<TransferError> for ContractError {
    fn from(e: TransferError) -> Self {
        let field = match e {
            TransferError::InvalidInput(f) | TransferError::Unsupported(f) => f,
            TransferError::Capacity { .. } => "transfer_capacity",
            TransferError::DomainCrossing { .. } => "protected_domain",
            TransferError::IdentityMismatch => "transfer_identity",
            TransferError::Budget(BudgetError::Cancelled) => "transfer_cancelled",
            TransferError::Budget(_) => "transfer_budget",
            TransferError::Codec => "transfer_codec",
        };
        e.contract(field)
    }
}

pub(crate) fn capacity(required: usize, available: usize) -> Result<(), TransferError> {
    if required > available {
        Err(TransferError::Capacity { required, available })
    } else {
        Ok(())
    }
}
pub(crate) fn capacity_overflow() -> TransferError {
    TransferError::Capacity {
        required: usize::MAX,
        available: 0,
    }
}
pub(crate) fn charge(budget: &mut ProductionBudget<'_>, factors: &[usize]) -> Result<(), TransferError> {
    let work = factors
        .iter()
        .try_fold(1u64, |a, &b| a.checked_mul(b as u64))
        .ok_or(TransferError::InvalidInput("work_overflow"))?;
    budget.checkpoint(work)?;
    Ok(())
}
pub(crate) fn tolerance(value: f64) -> Result<(), TransferError> {
    if value.is_finite() && value >= 0.0 && value < 1.0 {
        Ok(())
    } else {
        Err(TransferError::InvalidInput("tolerance"))
    }
}

#[cfg(test)]
mod tests;
