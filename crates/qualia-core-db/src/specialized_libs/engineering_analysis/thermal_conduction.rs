//! Real 1-D steady-state heat conduction — Fourier's law on a finite-difference
//! mesh, solved directly with the tridiagonal Thomas algorithm.
//!
//! This is the genuine numerical core behind `ThermalAnalyzer::analyze`
//! (`super::ThermalAnalyzer`). It replaces the prior stub, which returned a
//! default `AnalysisResults` (empty fields + a hardcoded safety factor) while
//! ignoring the model entirely.
//!
//! The governing equation is steady conduction with optional uniform volumetric
//! generation `g` (W/m³):
//!
//! ```text
//!   -k · d²T/dx² = g  on  x ∈ [0, L]
//! ```
//!
//! discretised on `N` equally-spaced nodes (`h = L/(N-1)`). Boundary conditions
//! come from the model: a `Temperature` BC is Dirichlet (a fixed end temperature)
//! and a `HeatFlux` BC is Neumann (a fixed `q = −k·dT/dx` at the end, applied with
//! a second-order ghost node). At least one end must be Dirichlet, otherwise the
//! temperature level is undetermined and we refuse with `InsufficientData`.
//!
//! Outputs are the real node temperature field and the heat-flux field
//! `q(x) = −k·dT/dx`. The tests verify the solver against the closed-form
//! solutions: the linear Dirichlet–Dirichlet profile, the parabolic profile under
//! uniform generation, and a Dirichlet–Neumann profile.
//!
//! Honesty boundary: a missing material / non-positive conductivity / non-positive
//! length / fewer than two boundary conditions / two flux ends → `InsufficientData`
//! (never a fabricated field). Full 2-D/3-D finite-element thermal analysis is a
//! larger subsystem and is flagged in the register, not faked here.

use super::{
    AnalysisResults, AnalysisType, BoundaryConditionType, EngineeringError, EngineeringModel,
};

/// Number of mesh nodes used for the 1-D discretisation. 41 nodes (40 intervals)
/// is ample for the smooth steady-state profiles this solves and keeps the direct
/// solve trivially cheap.
const N_NODES: usize = 41;

/// One end's thermal boundary condition.
#[derive(Clone, Copy)]
enum EndBc {
    /// Fixed temperature (Dirichlet), K.
    Temperature(f64),
    /// Fixed heat flux (Neumann), `q = −k·dT/dx`, W/m².
    Flux(f64),
}

/// Solve a tridiagonal system `A·x = d` with the Thomas algorithm. `a` is the
/// sub-diagonal (a[0] unused), `b` the diagonal, `c` the super-diagonal
/// (c[n-1] unused). Buffers are consumed (modified in place). Returns the solution.
fn thomas(
    mut a: Vec<f64>,
    mut b: Vec<f64>,
    mut c: Vec<f64>,
    mut d: Vec<f64>,
) -> Result<Vec<f64>, EngineeringError> {
    let n = b.len();
    for i in 1..n {
        if b[i - 1] == 0.0 {
            return Err(EngineeringError::InsufficientData(
                "thermal conduction: singular tridiagonal system (zero pivot)".to_string(),
            ));
        }
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        d[i] -= m * d[i - 1];
    }
    if b[n - 1] == 0.0 {
        return Err(EngineeringError::InsufficientData(
            "thermal conduction: singular tridiagonal system (zero final pivot)".to_string(),
        ));
    }
    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = (d[i] - c[i] * x[i + 1]) / b[i];
    }
    let _ = &mut a;
    Ok(x)
}

/// Build and solve the steady 1-D conduction system, returning the node
/// temperatures. `k` conductivity, `length` L, `g` uniform generation,
/// `left`/`right` the end boundary conditions.
fn solve_field(
    k: f64,
    length: f64,
    g: f64,
    left: EndBc,
    right: EndBc,
) -> Result<Vec<f64>, EngineeringError> {
    let n = N_NODES;
    let h = length / (n - 1) as f64;
    // Scale every interior equation by h²/k so the interior stencil is the clean
    // [-1, 2, -1] with RHS G = g·h²/k.
    let g_rhs = g * h * h / k;

    let mut a = vec![0.0; n]; // sub-diagonal
    let mut b = vec![0.0; n]; // diagonal
    let mut c = vec![0.0; n]; // super-diagonal
    let mut d = vec![0.0; n]; // rhs

    // Interior nodes: -T_{i-1} + 2 T_i - T_{i+1} = G
    for i in 1..n - 1 {
        a[i] = -1.0;
        b[i] = 2.0;
        c[i] = -1.0;
        d[i] = g_rhs;
    }

    // Left end (node 0).
    match left {
        EndBc::Temperature(t) => {
            b[0] = 1.0;
            c[0] = 0.0;
            d[0] = t;
        }
        EndBc::Flux(q) => {
            // Ghost-node Neumann: 2 T_0 - 2 T_1 = G + 2 q h / k
            b[0] = 2.0;
            c[0] = -2.0;
            d[0] = g_rhs + 2.0 * q * h / k;
        }
    }

    // Right end (node n-1).
    match right {
        EndBc::Temperature(t) => {
            a[n - 1] = 0.0;
            b[n - 1] = 1.0;
            d[n - 1] = t;
        }
        EndBc::Flux(q) => {
            // Ghost-node Neumann: 2 T_{n-1} - 2 T_{n-2} = G - 2 q h / k
            a[n - 1] = -2.0;
            b[n - 1] = 2.0;
            d[n - 1] = g_rhs - 2.0 * q * h / k;
        }
    }

    thomas(a, b, c, d)
}

/// Heat-flux field `q(x) = −k·dT/dx`: central differences on the interior,
/// second-order one-sided differences at the two ends.
fn heat_flux(temps: &[f64], k: f64, h: f64) -> Vec<f64> {
    let n = temps.len();
    let mut q = vec![0.0; n];
    if n < 2 {
        return q;
    }
    q[0] = -k * (-3.0 * temps[0] + 4.0 * temps[1] - temps[2.min(n - 1)]) / (2.0 * h);
    for i in 1..n - 1 {
        q[i] = -k * (temps[i + 1] - temps[i - 1]) / (2.0 * h);
    }
    let l = n - 1;
    q[l] = -k * (3.0 * temps[l] - 4.0 * temps[l - 1] + temps[l.saturating_sub(2)]) / (2.0 * h);
    q
}

/// Run a real 1-D steady-state conduction analysis over `model`.
pub fn analyze_conduction(
    model: &EngineeringModel,
    analysis_type: AnalysisType,
) -> Result<AnalysisResults, EngineeringError> {
    // Thermal conductivity from the (first) material.
    let material = model.materials.values().next().ok_or_else(|| {
        EngineeringError::InsufficientData(
            "thermal conduction: model has no material; thermal conductivity k is required"
                .to_string(),
        )
    })?;
    let k = material.material_properties.thermal_conductivity;
    if !(k > 0.0) {
        return Err(EngineeringError::InsufficientData(format!(
            "thermal conduction: material '{}' has non-positive thermal conductivity ({})",
            material.material_name, k
        )));
    }

    // Length from geometry: prefer the third dimension (length), else the first.
    let dims = &model.geometry.dimensions;
    let length = dims
        .get(2)
        .copied()
        .filter(|&l| l > 0.0)
        .or_else(|| dims.first().copied().filter(|&l| l > 0.0))
        .ok_or_else(|| {
            EngineeringError::InsufficientData(
                "thermal conduction: geometry has no positive length dimension".to_string(),
            )
        })?;

    // Boundary conditions: map Temperature ⇒ Dirichlet, HeatFlux ⇒ Neumann.
    // First such BC is the left end, second is the right end.
    let mut ends: Vec<EndBc> = Vec::new();
    for bc in &model.boundary_conditions {
        match bc.condition_type {
            BoundaryConditionType::Temperature => ends.push(EndBc::Temperature(bc.condition_value)),
            BoundaryConditionType::HeatFlux => ends.push(EndBc::Flux(bc.condition_value)),
            _ => {}
        }
        if ends.len() == 2 {
            break;
        }
    }
    if ends.len() < 2 {
        return Err(EngineeringError::InsufficientData(
            "thermal conduction: need two thermal boundary conditions (Temperature or HeatFlux) \
             at the ends; refusing to invent them"
                .to_string(),
        ));
    }
    let (left, right) = (ends[0], ends[1]);
    // At least one end must fix the temperature level, otherwise it is undetermined.
    if matches!(left, EndBc::Flux(_)) && matches!(right, EndBc::Flux(_)) {
        return Err(EngineeringError::InsufficientData(
            "thermal conduction: both ends are heat-flux (Neumann) boundaries, so the temperature \
             level is undetermined; at least one Temperature (Dirichlet) end is required"
                .to_string(),
        ));
    }

    // Optional uniform volumetric heat generation g (W/m³), summed from any
    // geometric-feature parameter named "heat_generation" (a real model field;
    // absent ⇒ pure conduction with g = 0). Never fabricated.
    let g: f64 = model
        .geometry
        .features
        .iter()
        .filter_map(|f| f.feature_parameters.get("heat_generation").copied())
        .sum();

    let temps = solve_field(k, length, g, left, right)?;
    let h = length / (N_NODES - 1) as f64;
    let flux = heat_flux(&temps, k, h);

    Ok(AnalysisResults {
        results_id: "thermal_conduction_1d".to_string(),
        analysis_type,
        displacement_field: Vec::new(),
        stress_field: Vec::new(),
        strain_field: Vec::new(),
        reaction_forces: Vec::new(),
        // A factor of safety is not defined for a pure conduction field; report
        // +∞ (no mechanical margin computed) rather than a misleading number.
        safety_factor: f64::INFINITY,
        temperature_field: temps,
        heat_flux_field: flux,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::engineering_analysis::{
        BoundaryCondition, FeatureType, GeometricFeature, Geometry, GeometryType, Material,
        MaterialProperties, ModelType,
    };
    use std::collections::HashMap;

    fn material(k: f64) -> Material {
        Material {
            material_id: "m".to_string(),
            material_name: "test".to_string(),
            material_properties: MaterialProperties {
                youngs_modulus: 0.0,
                poissons_ratio: 0.0,
                density: 0.0,
                thermal_expansion: 0.0,
                thermal_conductivity: k,
                specific_heat: 0.0,
                yield_strength: 0.0,
                ultimate_strength: 0.0,
            },
        }
    }

    fn model(k: f64, length: f64, bcs: Vec<BoundaryCondition>, g: f64) -> EngineeringModel {
        let mut materials = HashMap::new();
        materials.insert("m".to_string(), material(k));
        let features = if g != 0.0 {
            let mut p = HashMap::new();
            p.insert("heat_generation".to_string(), g);
            vec![GeometricFeature {
                feature_id: "gen".to_string(),
                feature_type: FeatureType::Rib,
                feature_parameters: p,
            }]
        } else {
            Vec::new()
        };
        EngineeringModel {
            model_id: "mdl".to_string(),
            model_name: "thermal".to_string(),
            model_type: ModelType::Thermal,
            geometry: Geometry {
                geometry_type: GeometryType::Beam,
                dimensions: vec![0.1, 0.1, length],
                features,
            },
            materials,
            boundary_conditions: bcs,
            loads: Vec::new(),
        }
    }

    fn temp_bc(v: f64) -> BoundaryCondition {
        BoundaryCondition {
            condition_id: "t".to_string(),
            condition_type: BoundaryConditionType::Temperature,
            condition_value: v,
        }
    }

    fn flux_bc(v: f64) -> BoundaryCondition {
        BoundaryCondition {
            condition_id: "q".to_string(),
            condition_type: BoundaryConditionType::HeatFlux,
            condition_value: v,
        }
    }

    #[test]
    fn dirichlet_dirichlet_is_linear() {
        // No generation: the steady profile is exactly linear from T_L to T_R.
        let k = 50.0;
        let l = 2.0;
        let (tl, tr) = (100.0, 300.0);
        let m = model(k, l, vec![temp_bc(tl), temp_bc(tr)], 0.0);
        let r = analyze_conduction(&m, AnalysisType::Thermal).unwrap();
        let t = &r.temperature_field;
        assert_eq!(t.len(), N_NODES);
        for (i, &ti) in t.iter().enumerate() {
            let x = l * i as f64 / (N_NODES - 1) as f64;
            let expected = tl + (tr - tl) * x / l;
            assert!((ti - expected).abs() < 1e-9, "node {i}: {ti} vs {expected}");
        }
        // Flux is constant q = -k (T_R - T_L)/L.
        let q_expected = -k * (tr - tl) / l;
        for &q in &r.heat_flux_field {
            assert!((q - q_expected).abs() < 1e-6, "{q} vs {q_expected}");
        }
    }

    #[test]
    fn uniform_generation_is_parabolic() {
        // -k T'' = g, Dirichlet both ends. Analytic:
        //   T(x) = T_L + (T_R - T_L) x/L + (g/2k) x (L - x).
        let k = 20.0;
        let l = 1.0;
        let (tl, tr) = (50.0, 50.0);
        let g = 1000.0;
        let m = model(k, l, vec![temp_bc(tl), temp_bc(tr)], g);
        let r = analyze_conduction(&m, AnalysisType::Thermal).unwrap();
        for (i, &ti) in r.temperature_field.iter().enumerate() {
            let x = l * i as f64 / (N_NODES - 1) as f64;
            let expected = tl + (tr - tl) * x / l + (g / (2.0 * k)) * x * (l - x);
            assert!((ti - expected).abs() < 1e-6, "node {i}: {ti} vs {expected}");
        }
        // The peak temperature is in the interior and exceeds both ends.
        let tmax = r.temperature_field.iter().cloned().fold(f64::MIN, f64::max);
        assert!(tmax > tl, "generation should raise interior temperature");
    }

    #[test]
    fn dirichlet_neumann_matches_imposed_flux() {
        // Left fixed temperature, right imposed flux q. With g=0 the profile is
        // linear with slope dT/dx = -q/k, so T(x) = T_L - (q/k) x.
        let k = 10.0;
        let l = 1.0;
        let tl = 200.0;
        let q = 100.0;
        let m = model(k, l, vec![temp_bc(tl), flux_bc(q)], 0.0);
        let r = analyze_conduction(&m, AnalysisType::Thermal).unwrap();
        for (i, &ti) in r.temperature_field.iter().enumerate() {
            let x = l * i as f64 / (N_NODES - 1) as f64;
            let expected = tl - (q / k) * x;
            assert!((ti - expected).abs() < 1e-6, "node {i}: {ti} vs {expected}");
        }
    }

    #[test]
    fn refuses_without_material() {
        let mut m = model(50.0, 1.0, vec![temp_bc(100.0), temp_bc(200.0)], 0.0);
        m.materials.clear();
        assert!(matches!(
            analyze_conduction(&m, AnalysisType::Thermal),
            Err(EngineeringError::InsufficientData(_))
        ));
    }

    #[test]
    fn refuses_nonpositive_conductivity() {
        let m = model(0.0, 1.0, vec![temp_bc(100.0), temp_bc(200.0)], 0.0);
        assert!(matches!(
            analyze_conduction(&m, AnalysisType::Thermal),
            Err(EngineeringError::InsufficientData(_))
        ));
    }

    #[test]
    fn refuses_too_few_bcs() {
        let m = model(50.0, 1.0, vec![temp_bc(100.0)], 0.0);
        assert!(matches!(
            analyze_conduction(&m, AnalysisType::Thermal),
            Err(EngineeringError::InsufficientData(_))
        ));
    }

    #[test]
    fn refuses_two_flux_ends() {
        let m = model(50.0, 1.0, vec![flux_bc(10.0), flux_bc(10.0)], 0.0);
        assert!(matches!(
            analyze_conduction(&m, AnalysisType::Thermal),
            Err(EngineeringError::InsufficientData(_))
        ));
    }
}
