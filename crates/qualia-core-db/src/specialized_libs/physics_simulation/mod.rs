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

mod types;
mod engine;
mod time_integration;
mod discretization;
mod boundary_initial;
mod solvers;
mod solvers_eigen_opt;
mod distributed;
mod data_storage;
mod data_migration;
mod metrics;
mod domain_model;
mod results;
mod errors;
mod library_core;
mod cfd;
mod ode;
mod mechanics;
mod nbody;
mod fields;
mod molecular_dynamics;
mod quantum;
mod population;

pub use types::*;
pub use engine::*;
pub use time_integration::*;
pub use discretization::*;
pub use boundary_initial::*;
pub use solvers::*;
pub use solvers_eigen_opt::*;
pub use distributed::*;
pub use data_storage::*;
pub use data_migration::*;
pub use metrics::*;
pub use domain_model::*;
pub use results::*;
pub use errors::*;
pub use library_core::*;
pub use cfd::*;
pub use ode::*;
pub use mechanics::*;
pub use nbody::*;
pub use fields::*;
pub use molecular_dynamics::*;
pub use quantum::*;
pub use population::*;

#[cfg(test)]
mod tests;
