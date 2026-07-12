//! Engineering Analysis Library - Structural, Mechanical, and Systems Engineering Analysis
//!
//! This module provides high-performance engineering analysis operations leveraging Phase 2 enhancements:
//! - Linear Algebra Library for matrix computations and finite element analysis
//! - Physics Simulation Library for structural dynamics and thermal analysis
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy engineering data
//! - Statistical Computing Library for reliability analysis and optimization

use super::linear_algebra::LinearAlgebraLibrary;
use super::physics_simulation::PhysicsSimulationLibrary;
use super::statistical_computing::StatisticalComputingLibrary;
use crate::zns_storage::ZnsZoneManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Standard normal random sample via the Box–Muller transform, using two uniform
/// draws from `rand::random()`. Returns a single N(0,1) value. Used by the Monte
/// Carlo reliability kernel — this is NOT a hot path (engineering analysis is a
/// planning/analysis module, not the evaluator loop), so `Vec`/`rand` are fine.
fn standard_normal_sample() -> f64 {
    // Draw two independent uniforms in (0, 1]; reject exact 0 to avoid log(0).
    let mut u1: f64 = rand::random();
    while u1 <= 0.0 {
        u1 = rand::random();
    }
    let u2: f64 = rand::random();
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

/// Approximate inverse of the standard normal CDF (Φ⁻¹) via the Acklam/Wichura
/// rational approximation. Given a failure probability `p` ∈ (0, 1), returns the
/// reliability index β = −Φ⁻¹(p). Clamps `p` away from 0/1 to keep the result
/// finite.
fn inverse_normal_cdf(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    // Acklam's algorithm.
    let a = [
        -3.969_683_028_665_376e+01,
        2.209_460_984_245_205e+02,
        -2.759_285_104_469_687e+02,
        1.383_577_518_672_69e+02,
        -3.066_479_806_617_929e+01,
        2.506_628_277_459_239e+00,
    ];
    let b = [
        -5.447_609_879_822_406e+01,
        1.615_858_368_580_409e+02,
        -1.556_989_798_598_866e+02,
        6.680_131_188_771_972e+01,
        -1.328_068_155_288_362e+01,
    ];
    let c = [
        -7.784_894_002_430_993e-03,
        -3.223_964_580_411_365e-01,
        -2.400_758_277_161_838e+00,
        -2.549_732_539_349_742e+00,
        4.374_664_141_464_968e+00,
        2.938_163_982_698_783e+00,
    ];
    let d = [
        7.784_695_709_041_462e-03,
        3.224_671_290_700_398e-01,
        2.445_134_137_232_851e+00,
        3.754_408_661_907_416e+00,
    ];

    let plow = 0.02425;
    let phigh = 1.0 - plow;
    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= phigh {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    }
}

/// Standard normal CDF Φ(x) via the Abramowitz & Stegun 7.1.26 approximation
/// (maximum absolute error < 7.5e-8). Used to compute failure probability
/// from the reliability index: P(fail) = Φ(−β).
fn normal_cdf(x: f64) -> f64 {
    // Φ(x) = ½ [1 + erf(x / √2)]
    let z = x / std::f64::consts::SQRT_2;
    let erf = if z >= 0.0 {
        // erf(z) for z ≥ 0 via A&S 7.1.26
        let t = 1.0 / (1.0 + 0.3275911 * z);
        let poly = t
            * (0.254829592
                + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        1.0 - poly * (-z * z).exp()
    } else {
        // erf(-z) = -erf(z)
        let az = -z;
        let t = 1.0 / (1.0 + 0.3275911 * az);
        let poly = t
            * (0.254829592
                + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        -(1.0 - poly * (-az * az).exp())
    };
    0.5 * (1.0 + erf)
}

/// Solve the undamped free-vibration generalized eigenproblem `K φ = ω² M φ` for a
/// symmetric stiffness matrix `stiffness` (row-major `n×n`) and a **lumped
/// (diagonal)** mass matrix given by its `n` positive diagonal entries `mass_diag`.
/// Returns `(ω, φ)` pairs sorted by ascending natural angular frequency ω (rad/s).
///
/// Method (mass pre/post-scaling to standard form): let `M^{-1/2}` be the diagonal
/// matrix of `1/√mᵢ`. Then `Ã = M^{-1/2} K M^{-1/2}` is symmetric and
/// `Ã ψ = ω² ψ` with `ψ = M^{1/2} φ`. The standard symmetric eigenproblem is solved
/// by the crate's Jacobi eigensolver
/// (`solvers::linear_algebra::eigen::symmetric_eigen`) — **no eigen algorithm is
/// re-derived here** — and the physical mode is recovered as `φ = M^{-1/2} ψ`.
/// Eigenvalues that come out marginally negative from round-off are clamped to 0
/// before the square root. Mode shapes are scaled to unit maximum component.
fn solve_modal_eigen(
    stiffness: &[f64],
    mass_diag: &[f64],
    n: usize,
) -> Result<Vec<(f64, Vec<f64>)>, EngineeringError> {
    if n == 0 {
        return Err(EngineeringError::InsufficientData(
            "system has zero degrees of freedom".to_string(),
        ));
    }
    if stiffness.len() != n * n {
        return Err(EngineeringError::ValidationError(format!(
            "stiffness must have n*n = {} entries, got {}",
            n * n,
            stiffness.len()
        )));
    }
    if mass_diag.len() != n {
        return Err(EngineeringError::ValidationError(format!(
            "mass diagonal must have n = {} entries, got {}",
            n,
            mass_diag.len()
        )));
    }
    if mass_diag.iter().any(|&m| !(m > 0.0)) {
        return Err(EngineeringError::ValidationError(
            "all lumped masses must be positive".to_string(),
        ));
    }

    // Ã = M^{-1/2} K M^{-1/2}.
    let inv_sqrt_m: Vec<f64> = mass_diag.iter().map(|&m| 1.0 / m.sqrt()).collect();
    let mut a = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            a[i * n + j] = stiffness[i * n + j] * inv_sqrt_m[i] * inv_sqrt_m[j];
        }
    }

    let mut eigvecs = vec![0.0_f64; n * n];
    crate::solvers::linear_algebra::eigen::symmetric_eigen(n, &mut a, &mut eigvecs).map_err(
        |e| EngineeringError::SolverError(format!("symmetric eigensolver failed: {:?}", e)),
    )?;

    // Diagonal of the transformed matrix now holds the eigenvalues λ = ω²; column
    // `j` of `eigvecs` is the corresponding ψ.
    let mut modes: Vec<(f64, Vec<f64>)> = Vec::with_capacity(n);
    for j in 0..n {
        let lambda = a[j * n + j];
        let omega = lambda.max(0.0).sqrt();
        // Physical mode φ = M^{-1/2} ψ (column j of eigvecs).
        let mut phi: Vec<f64> = (0..n).map(|i| eigvecs[i * n + j] * inv_sqrt_m[i]).collect();
        let max_abs = phi.iter().fold(0.0_f64, |m, &v| m.max(v.abs()));
        if max_abs > 0.0 {
            for v in phi.iter_mut() {
                *v /= max_abs;
            }
        }
        modes.push((omega, phi));
    }
    modes.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(core::cmp::Ordering::Equal));
    Ok(modes)
}

/// Real 1-D steady-state heat-conduction solver (Fourier's law, finite-difference
/// + tridiagonal Thomas algorithm) backing `perform_thermal_analysis`. Split into
/// its own library submodule (PROJECT RULE §11); carries its own correctness tests
/// against the analytic conduction solutions.
pub mod thermal_conduction;

/// Real 2-D incompressible Navier–Stokes finite-volume solver (Chorin projection
/// method on a staggered Cartesian grid). Backs `perform_fluid_analysis`. Split
/// into its own library submodule (PROJECT RULE §11); carries its own correctness
/// tests (lid-driven cavity, channel flow, pressure outlet).
pub mod cfd;

// ── Library-ized submodules (PROJECT RULE §11: mechanical code-motion split of a
// ~6.7k-line mod.rs into single-purpose siblings; no logic/signature change). Each
// submodule uses `use super::*` for shared types and helper fns; `mod.rs` re-exports
// the full public surface via `pub use <name>::*` so every existing external path
// (`crate::specialized_libs::engineering_analysis::<Item>`) resolves exactly as before.
mod buckling;
mod dynamics;
mod errors;
mod fluid;
mod library;
mod mechanical;
mod model;
mod reliability;
mod structural;
mod survival;
mod thermal;
mod vibration;

pub use buckling::*;
pub use dynamics::*;
pub use errors::*;
pub use fluid::*;
pub use library::*;
pub use mechanical::*;
pub use model::*;
pub use reliability::*;
pub use structural::*;
pub use survival::*;
pub use thermal::*;
pub use vibration::*;

#[cfg(test)]
mod tests;
