//! P20 — GPU-less atomic activation, rollback, and state-migration accounting.
//!
//! This is an adapter over [`super::production_runtime`], not a GPU driver
//! and not P22 device-loss qualification. It records a typed five-phase
//! ledger (`decode`, `upload`, `compaction`, `draw`, `activation`) with
//! independent `work_units`, `bytes`, `status`, and [`TimingSource`]. There
//! is no blended score. CPU/synthetic durations are labelled `CpuMonotonic`
//! or `NotMeasured` and cannot be labelled `GpuTimestamp`.
//!
//! GPU execution is `unsupported` unless the caller supplies a live device
//! token. This module never constructs `wgpu::Device`. Even with a token it
//! does not dispatch GPU work; a present device without a driver execution is
//! `inconclusive`, never `passed`. Token presence is not a GPU timestamp.
//!
//! # Fault injection scope
//!
//! Injection is a GPU-less state machine. It can force:
//! - `device_lost` around [`QualifiedSet`] activation
//! - missing page / incomplete replacement (`PageState`, `activate_replacement`)
//! - draw-command capacity overflow (`generate_indexed_draws` zeros the buffer)
//! - cancelled inflight residency transfer (`PageState::Cancelled`)
//! - mixed-generation activation (`StaleIdentity`)
//! - incomplete assets / invalidated evidence around activation
//!
//! Injection cannot construct or lose a real device, emit GPU timestamps,
//! undo consumed simulation ([`ROLLBACK_DOES_NOT_UNDO_SIMULATION`]), or
//! change `decoded_error_ledger` physical records (those stay `unsupported`).
//! Energy and momentum deltas are `Unsupported`, never a silent `0.0`.
//!
//! Parent `computational_geometry/mod.rs` must declare `pub mod production_lifecycle`.

mod faults;
mod phases;

pub use faults::*;
pub use phases::*;

use super::geometry_workspace::production_budget::BudgetError;
use super::production_runtime::RuntimeError;

pub use super::production_contracts::TimingSource;
pub use super::production_runtime::{QualifiedSet, ROLLBACK_DOES_NOT_UNDO_SIMULATION};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidInput(&'static str),
    Capacity { required: usize, available: usize },
    DishonestTiming,
    Budget(BudgetError),
    Runtime(RuntimeError),
}

impl From<BudgetError> for LifecycleError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}

impl From<RuntimeError> for LifecycleError {
    fn from(value: RuntimeError) -> Self {
        match value {
            RuntimeError::Budget(error) => Self::Budget(error),
            other => Self::Runtime(other),
        }
    }
}

impl core::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "lifecycle invalid input: {msg}"),
            Self::Capacity { required, available } => {
                write!(f, "lifecycle capacity {required} > {available}")
            }
            Self::DishonestTiming => {
                f.write_str("CPU/synthetic time cannot be labelled GpuTimestamp")
            }
            Self::Budget(error) => write!(f, "{error}"),
            Self::Runtime(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for LifecycleError {}
