//! Physics Simulation Library - High-Performance Physics Computing
//!
//! This module provides high-performance physics simulation operations leveraging Phase 2 enhancements:
//! - NVMe Computational Storage (CSD) for hardware-accelerated physics computations
//! - Zero-Infrastructure Acoustic & BLE Mesh for distributed physics simulations
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy physics data
//! - Ambient Sub-Threshold Orchestration for mobile physics optimization

use super::linear_algebra::AccessPattern;
use crate::acoustic_ble_mesh::{MeshNetworkManager, MessagePriority, NetworkStatus};
// Real, tested numeric solvers reused for the physics simulations below. No numerical
// algorithm is re-derived inline here — every integration/eigen step delegates to these.
use crate::solvers::calculus::ode_adaptive::{integrate_dopri5, AdaptiveOdeConfig, OdeError};
use crate::solvers::calculus::ode_advanced::{integrate_symplectic, SymplecticMethod};
use crate::solvers::linear_algebra::eigen::symmetric_eigen;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod boundary_initial;
mod cfd;
mod data_migration;
mod data_storage;
mod discretization;
mod distributed;
mod domain_model;
mod emf;
mod engine;
mod errors;
mod fields;
mod library_core;
mod mechanics;
mod metrics;
mod molecular_dynamics;
mod nbody;
mod ode;
mod population;
mod quantum;
mod results;
mod solvers;
mod solvers_eigen_opt;
mod time_integration;
mod types;

pub use boundary_initial::*;
pub use data_migration::*;
pub use data_storage::*;
pub use discretization::*;
pub use distributed::*;
pub use domain_model::*;
pub use engine::*;
pub use errors::*;
pub use metrics::*;
pub use results::*;
pub use solvers::*;
pub use solvers_eigen_opt::*;
pub use time_integration::*;
pub use types::*;
// The kernel submodules (library_core/cfd/ode/mechanics/nbody/fields/
// molecular_dynamics/quantum/population) contribute `impl PhysicsSimulationLibrary`
// blocks and tree-internal helpers only — no free items to re-export.
pub use emf::EmfSource;

#[cfg(test)]
mod tests;
