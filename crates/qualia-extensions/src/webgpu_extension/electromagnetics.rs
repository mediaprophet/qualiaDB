//! 1D electromagnetics by the **Yee FDTD** scheme (the canonical, validatable
//! Maxwell solver).
//!
//! `E_y` and `H_z` are staggered in space and leap-frogged in time:
//! ```text
//! Hz[i] += (Δt/μΔx)·(Ey[i+1] − Ey[i])
//! Ey[i] += (Δt/εΔx)·(Hz[i] − Hz[i-1]) − (Δt·σ/ε)·Ey[i]
//! ```
//! with the Courant number `c·Δt/Δx ≤ 1`. Fields are stored in the 3-vector
//! layout `E = (0, Ey, 0)`, `H = (0, 0, Hz)` (`electric_out` / `magnetic_out`).
//!
//! Validation: an impedance-matched pulse (`Hz = Ey/η`) is a pure right-moving
//! wave; the test checks it travels the analytic distance `c·Δt·N/Δx` and that
//! the lossless field energy stays bounded.

use super::{dim, uniform, SolverReport, WebGpuJobParams};
use std::collections::HashMap;

pub fn solve(params: &WebGpuJobParams) -> SolverReport {
    let n = dim(params.grid_size.0, 200);
    let dx = 1.0f64;
    let eps = uniform(params, "epsilon", 1.0).max(1e-6) as f64;
    let mu = uniform(params, "mu", 1.0).max(1e-6) as f64;
    let sigma = uniform(params, "sigma", 0.0).max(0.0) as f64;
    let c = 1.0 / (eps * mu).sqrt();

    // Courant-stable time step (Sc = 0.5 keeps a clean integer cell speed).
    let courant = 0.5;
    let dt = courant * dx / c;
    let steps = if params.dispatch_params.iterations > 0 {
        params.dispatch_params.iterations
    } else {
        100
    };

    let eta = (mu / eps).sqrt(); // wave impedance

    let mut ey = vec![0.0f64; n];
    let mut hz = vec![0.0f64; n];

    if let (Some(ein), Some(hin)) = (
        params.input_data.get("electric_field"),
        params.input_data.get("magnetic_field"),
    ) {
        // Caller fields are 3-vectors: read the y/z components.
        for i in 0..n.min(ein.len() / 3) {
            ey[i] = ein[3 * i + 1] as f64;
        }
        for i in 0..n.min(hin.len() / 3) {
            hz[i] = hin[3 * i + 2] as f64;
        }
    } else {
        // Impedance-matched Gaussian pulse → pure +x traveling wave.
        let x0 = n as f64 * 0.25;
        let width = n as f64 / 16.0;
        for i in 0..n {
            let arg = (i as f64 - x0) / width;
            let g = (-arg * arg).exp();
            ey[i] = g;
            hz[i] = g / eta;
        }
    }

    let energy = |ey: &[f64], hz: &[f64]| -> f64 {
        0.5 * (0..n)
            .map(|i| eps * ey[i] * ey[i] + mu * hz[i] * hz[i])
            .sum::<f64>()
    };
    let e0 = energy(&ey, &hz);

    let ch = dt / (mu * dx);
    let ce = dt / (eps * dx);
    let loss = dt * sigma / eps;

    // Maxwell's curl equations: μ∂Hz/∂t = −∂Ey/∂x, ε∂Ey/∂t = −∂Hz/∂x − σEy.
    for _ in 0..steps {
        for i in 0..n {
            let ip1 = if i + 1 < n { i + 1 } else { 0 };
            hz[i] -= ch * (ey[ip1] - ey[i]);
        }
        for i in 0..n {
            let im1 = if i == 0 { n - 1 } else { i - 1 };
            ey[i] -= ce * (hz[i] - hz[im1]) + loss * ey[i];
        }
    }

    let e1 = energy(&ey, &hz);

    let mut electric_out = Vec::with_capacity(n * 3);
    let mut magnetic_out = Vec::with_capacity(n * 3);
    for i in 0..n {
        electric_out.extend_from_slice(&[0.0, ey[i] as f32, 0.0]);
        magnetic_out.extend_from_slice(&[0.0, 0.0, hz[i] as f32]);
    }
    let mut output = HashMap::new();
    output.insert("electric_out".to_string(), electric_out);
    output.insert("magnetic_out".to_string(), magnetic_out);

    // Residual: relative energy change (≈0 in the lossless case).
    let residual = if e0 > 0.0 {
        ((e1 - e0) / e0).abs() as f32
    } else {
        0.0
    };
    let threshold = params.dispatch_params.convergence_threshold.max(0.1);
    let flops = steps as u64 * n as u64 * 8;

    SolverReport {
        output,
        iterations_used: steps,
        final_residual: residual,
        // For a lossy run (σ>0) energy *should* drop, so "converged" only
        // claims energy stability for the lossless case.
        converged: sigma > 0.0 || residual <= threshold,
        flops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webgpu_extension::DispatchParams;

    fn em_params(n: u32, steps: u32, sigma: f32) -> WebGpuJobParams {
        let mut uniforms = HashMap::new();
        uniforms.insert("epsilon".to_string(), 1.0);
        uniforms.insert("mu".to_string(), 1.0);
        uniforms.insert("sigma".to_string(), sigma);
        WebGpuJobParams {
            shader_name: "maxwell_fdtd_1d".to_string(),
            grid_size: (n, 1, 1),
            input_data: HashMap::new(),
            uniform_data: uniforms,
            dispatch_params: DispatchParams {
                iterations: steps,
                time_step: 0.0,
                convergence_threshold: 0.1,
                max_execution_time_ms: 30_000,
            },
        }
    }

    fn peak_index(field: &[f32]) -> usize {
        // field is 3-vector; the Ey component is index 3*i + 1.
        let n = field.len() / 3;
        let mut best = 0usize;
        let mut best_v = f32::MIN;
        for i in 0..n {
            let v = field[3 * i + 1];
            if v > best_v {
                best_v = v;
                best = i;
            }
        }
        best
    }

    #[test]
    fn pulse_propagates_at_the_speed_of_light() {
        // c = 1, Sc = 0.5 ⇒ pulse advances 0.5 cells/step. 100 steps ⇒ 50 cells.
        let n = 200u32;
        let steps = 100u32;
        let r = solve(&em_params(n, steps, 0.0));
        let peak = peak_index(&r.output["electric_out"]);
        let start = (n as f64 * 0.25) as usize; // 50
        let expected = start + 50; // 100
        assert!(
            (peak as i64 - expected as i64).abs() <= 3,
            "pulse peak at {peak}, expected ~{expected}"
        );
    }

    #[test]
    fn lossless_energy_is_bounded() {
        // The same-time Yee energy oscillates by a small amount (E and H live on
        // staggered time levels); for a well-resolved pulse it stays within a
        // few percent. The precise physics check is `propagates_at_c` above.
        let r = solve(&em_params(200, 100, 0.0));
        assert!(
            r.final_residual < 0.1,
            "lossless energy drifted by {}",
            r.final_residual
        );
        assert!(r.converged);
    }

    #[test]
    fn lossy_medium_dissipates_energy() {
        let r = solve(&em_params(200, 200, 0.05));
        // Conductivity must remove a large fraction of the energy.
        assert!(
            r.final_residual > 0.2,
            "lossy run did not dissipate energy (residual {})",
            r.final_residual
        );
    }
}
