// ─── Survival-engineering kernels: stress / drag / fatigue ───────────────────────
//
// The rapid-deployment / mobile-infrastructure scope (camper trailers, pop-up
// habitations, harsh-environment survival). The big FEA scaffolding above is the type
// machinery; these are the actual continuum-mechanics / fluid-dynamics / fatigue
// COMPUTATIONS. All zero-heap (fixed-size tensors / caller slices).

/// The reduced state of a 3×3 Cauchy stress tensor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StressState {
    /// von Mises equivalent stress (yield criterion).
    pub von_mises: f64,
    /// Principal stresses σ1 ≥ σ2 ≥ σ3 (the tensor's eigenvalues).
    pub principal: [f64; 3],
    /// Maximum shear stress = (σ1 − σ3) / 2 (Tresca).
    pub max_shear: f64,
    /// Hydrostatic (mean) stress = trace / 3.
    pub hydrostatic: f64,
}

/// Principal stresses of a symmetric 3×3 stress tensor by the closed-form
/// (Smith 1961) eigenvalue solution, returned σ1 ≥ σ2 ≥ σ3.
/// Principal stresses = eigenvalues of the symmetric Cauchy stress tensor, sorted
/// descending. The closed-form symmetric-3×3 eigensolver lives once in the engine
/// (`solvers::linear_algebra::eigen`); this marshals the tensor and calls it.
fn principal_stresses(t: &[[f64; 3]; 3]) -> [f64; 3] {
    let a = [
        t[0][0], t[0][1], t[0][2], t[1][0], t[1][1], t[1][2], t[2][0], t[2][1], t[2][2],
    ];
    crate::solvers::linear_algebra::eigen::symmetric_eigen_3x3(&a)
}

/// Analyse a 3×3 Cauchy stress tensor (e.g. chassis shear on an off-road camper):
/// von Mises equivalent stress, principal stresses, maximum shear, hydrostatic stress.
pub fn cauchy_stress_analysis(tensor: &[[f64; 3]; 3]) -> StressState {
    let (sxx, syy, szz) = (tensor[0][0], tensor[1][1], tensor[2][2]);
    let (txy, tyz, tzx) = (tensor[0][1], tensor[1][2], tensor[2][0]);
    let von_mises = (0.5 * ((sxx - syy).powi(2) + (syy - szz).powi(2) + (szz - sxx).powi(2))
        + 3.0 * (txy * txy + tyz * tyz + tzx * tzx))
        .sqrt();
    let principal = principal_stresses(tensor);
    StressState {
        von_mises,
        principal,
        max_shear: (principal[0] - principal[2]) / 2.0,
        hydrostatic: (sxx + syy + szz) / 3.0,
    }
}

/// Aerodynamic drag / wind-load force (N): `F = ½·ρ·v²·C_d·A` — wind-load on a
/// rapid-deployment structure or drag on a moving camper.
pub fn drag_force(
    air_density_kg_m3: f64,
    velocity_m_s: f64,
    drag_coefficient: f64,
    area_m2: f64,
) -> f64 {
    0.5 * air_density_kg_m3 * velocity_m_s * velocity_m_s * drag_coefficient * area_m2
}

/// Reynolds number `Re = ρ·v·L / μ` — laminar/turbulent regime for the wind-load model.
pub fn reynolds_number(
    density: f64,
    velocity: f64,
    char_length_m: f64,
    dynamic_viscosity: f64,
) -> f64 {
    if dynamic_viscosity == 0.0 {
        return f64::INFINITY;
    }
    density * velocity * char_length_m / dynamic_viscosity
}

/// Cycles-to-failure under a constant stress amplitude via Basquin's law
/// `σ_a = σ_f'·(2N)^b`  ⇒  `N = ½·(σ_a/σ_f')^(1/b)` (`b` is the negative fatigue
/// strength exponent). Below the endurance behaviour this is huge (effectively
/// infinite life). Feeds the probabilistic failure-prediction model.
pub fn fatigue_cycles_basquin(
    stress_amplitude: f64,
    fatigue_strength_coeff: f64,
    fatigue_strength_exponent: f64,
) -> f64 {
    if stress_amplitude <= 0.0 || fatigue_strength_coeff <= 0.0 || fatigue_strength_exponent == 0.0
    {
        return f64::INFINITY;
    }
    0.5 * (stress_amplitude / fatigue_strength_coeff).powf(1.0 / fatigue_strength_exponent)
}

/// Palmgren–Miner cumulative fatigue damage `D = Σ nᵢ/Nᵢ` over load blocks
/// `(applied_cycles, allowable_cycles)`. Failure is predicted when `D ≥ 1`. Zero-heap.
pub fn miner_cumulative_damage(blocks: &[(f64, f64)]) -> f64 {
    let mut d = 0.0;
    for &(applied, allowable) in blocks {
        if allowable > 0.0 {
            d += applied / allowable;
        }
    }
    d
}

#[cfg(test)]
mod survival_engineering_tests {
    use super::*;

    #[test]
    fn uniaxial_stress_state() {
        // Pure uniaxial tension of 100 MPa along x.
        let t = [[100.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let s = cauchy_stress_analysis(&t);
        assert!((s.von_mises - 100.0).abs() < 1e-6);
        assert!((s.principal[0] - 100.0).abs() < 1e-6 && s.principal[2].abs() < 1e-6);
        assert!((s.max_shear - 50.0).abs() < 1e-6);
        assert!((s.hydrostatic - 100.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn pure_shear_stress_state() {
        // Pure shear τ = 50 → principal {50, 0, −50}, von Mises = √3·50 ≈ 86.6.
        let t = [[0.0, 50.0, 0.0], [50.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let s = cauchy_stress_analysis(&t);
        assert!(
            (s.von_mises - 3f64.sqrt() * 50.0).abs() < 1e-6,
            "vm {}",
            s.von_mises
        );
        assert!(
            (s.principal[0] - 50.0).abs() < 1e-6,
            "σ1 {}",
            s.principal[0]
        );
        assert!(
            (s.principal[2] + 50.0).abs() < 1e-6,
            "σ3 {}",
            s.principal[2]
        );
        assert!((s.max_shear - 50.0).abs() < 1e-6);
    }

    #[test]
    fn drag_and_reynolds() {
        // 10 m/s wind on 2 m² flat-ish panel (Cd≈1) in sea-level air (ρ=1.225).
        assert!((drag_force(1.225, 10.0, 1.0, 2.0) - 122.5).abs() < 1e-6);
        // Re for 1 m chord at 10 m/s in air (μ≈1.8e-5) → ~6.8e5 (turbulent).
        let re = reynolds_number(1.225, 10.0, 1.0, 1.8e-5);
        assert!(re > 6.0e5 && re < 7.0e5, "Re {re}");
    }

    #[test]
    fn fatigue_life_and_cumulative_damage() {
        // Lower stress amplitude ⇒ more cycles to failure (Basquin, b<0).
        let n_low = fatigue_cycles_basquin(100.0, 900.0, -0.085);
        let n_high = fatigue_cycles_basquin(300.0, 900.0, -0.085);
        assert!(n_low > n_high, "lower stress should give longer life");
        // Miner: two blocks each at half their allowable → D = 1.0 (failure threshold).
        let d = miner_cumulative_damage(&[(500.0, 1000.0), (250.0, 500.0)]);
        assert!((d - 1.0).abs() < 1e-9, "D {d}");
        assert!(
            miner_cumulative_damage(&[(100.0, 1000.0)]) < 1.0,
            "safe block < 1"
        );
    }
}
