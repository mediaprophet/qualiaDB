//! Chemistry Modeling Library - Molecular Simulation and Chemical Analysis
//!
//! This module provides high-performance chemistry modeling operations leveraging Phase 2 enhancements:
//! - NVMe Computational Storage (CSD) for hardware-accelerated molecular computations
//! - Linear Algebra Library for quantum chemistry calculations
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy molecular data
//! - Statistical Computing Library for molecular dynamics analysis

use super::linear_algebra::LinearAlgebraLibrary;
use super::statistical_computing::StatisticalComputingLibrary;
use crate::csd_storage::CsdManager;
use crate::zns_storage::ZnsZoneManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Real molecular-dynamics engine (Lennard-Jones force field + velocity-Verlet
/// integrator) backing `run_molecular_dynamics`. Split into its own library
/// submodule (PROJECT RULE §11) so the genuine numerical core is reviewable on
/// its own and carries its own correctness tests.
pub mod molecular_dynamics;

/// Analytical Integral Engine for Quantum Chemistry
pub mod integrals;

/// Basis Set and Spatial Discretization Engine for Quantum Chemistry
pub mod basis_set;

/// Self-Consistent Field (SCF) Iterative Driver
pub mod scf;

/// Density Functional Theory (DFT) Integration
pub mod dft;

// Library-ized surface (PROJECT RULE §11): the former monolithic `mod.rs` body
// is split by cohesive concern into the sibling files below. Each submodule uses
// `use super::*;` for shared types; the full public surface is re-exported here
// so every `crate::specialized_libs::chemistry_modeling::<Item>` path resolves
// exactly as before.

/// `ChemistryModelingLibrary` manager struct and its methods.
mod library;
/// Core molecule/result value types (`Molecule`, `Atom`, `Bond`, trajectories …).
mod types;
/// Exact structural / mass properties (`standard_atomic_weight`, `StructuralProperties`).
mod structure;
/// Molecular-dynamics simulator (force fields, integrators, interactions).
mod simulation;
/// Quantum chemistry calculator surface.
mod quantum;
/// Reaction / kinetics / thermodynamics / phase analysis.
mod kinetics;
/// Property prediction (QSPR / descriptors / ML models).
mod properties;
/// Performance monitoring metrics.
mod metrics;
/// Chemistry error type.
mod errors;

pub use errors::*;
pub use kinetics::*;
pub use library::*;
pub use metrics::*;
pub use properties::*;
pub use quantum::*;
pub use simulation::*;
pub use structure::*;
pub use types::*;

#[cfg(test)]
mod tests;
