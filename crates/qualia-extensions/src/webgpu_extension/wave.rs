//! 2D wave equation `∂²u/∂t² = c²∇²u` by explicit leapfrog (periodic).
//!
//! `u_new = 2u − u_old + (cΔt)²∇²u`, CFL `cΔt·√(hx⁻²+hy⁻²) ≤ 1`.
//!
//! Validation: `u(x,y,0)=sin(x)sin(y)` with zero initial velocity is a standing
//! wave `u(t)=sin(x)sin(y)·cos(ωt)`, `ω=c√2`. After half a period the field
//! inverts (`u(T/2) ≈ −u(0)`); the test checks that the solution is strongly
//! anti-correlated with the initial state at `T/2` and never blows up.

use super::{dim, uniform, SolverReport, WebGpuJobParams};
use std::collections::HashMap;
use std::f64::consts::PI;

#[inline]
fn wrap(i: i64, n: i64) -> usize {
    (((i % n) + n) % n) as usize
}

pub fn solve(params: &WebGpuJobParams) -> SolverReport {
    let nx = dim(params.grid_size.0, 32);
    let ny = dim(params.grid_size.1, 32);
    let n = nx * ny;
    let (nxi, nyi) = (nx as i64, ny as i64);

    let hx = 2.0 * PI / nx as f64;
    let hy = 2.0 * PI / ny as f64;
    let c = uniform(params, "wave_speed", 1.0).max(1e-6) as f64;

    // CFL-stable step.
    let cfl_max = 1.0 / (c * (1.0 / (hx * hx) + 1.0 / (hy * hy)).sqrt());
    let mut dt = if params.dispatch_params.time_step > 0.0 {
        params.dispatch_params.time_step
    } else {
        0.5 * cfl_max
    };
    if dt > 0.95 * cfl_max {
        dt = 0.95 * cfl_max;
    }
    let steps = if params.dispatch_params.iterations > 0 {
        params.dispatch_params.iterations
    } else {
        44
    };

    let idx = |i: usize, j: usize| j * nx + i;

    // Seed a standing wave with (near) zero initial velocity.
    let omega = c * 2.0f64.sqrt(); // mode (1,1)
    let mut u = vec![0.0f64; n];
    let mut u_old = vec![0.0f64; n];
    if let Some(field) = params.input_data.get("displacement_field") {
        for k in 0..n.min(field.len()) {
            u[k] = field[k] as f64;
            u_old[k] = u[k];
        }
    } else {
        for j in 0..ny {
            for i in 0..nx {
                let base = (i as f64 * hx).sin() * (j as f64 * hy).sin();
                u[idx(i, j)] = base;
                // u(−Δt) = base·cos(ωΔt) ⇒ centred velocity ≈ 0.
                u_old[idx(i, j)] = base * (omega * dt).cos();
            }
        }
    }

    let inv_hx2 = 1.0 / (hx * hx);
    let inv_hy2 = 1.0 / (hy * hy);
    let coef = (c * dt) * (c * dt);
    let mut u_new = vec![0.0f64; n];

    let mut max_abs = 0.0f64;
    for _ in 0..steps {
        for j in 0..ny {
            for i in 0..nx {
                let cc = idx(i, j);
                let e = idx(wrap(i as i64 + 1, nxi), j);
                let w = idx(wrap(i as i64 - 1, nxi), j);
                let nn = idx(i, wrap(j as i64 + 1, nyi));
                let s = idx(i, wrap(j as i64 - 1, nyi));
                let lap = (u[e] + u[w] - 2.0 * u[cc]) * inv_hx2
                    + (u[nn] + u[s] - 2.0 * u[cc]) * inv_hy2;
                u_new[cc] = 2.0 * u[cc] - u_old[cc] + coef * lap;
                max_abs = max_abs.max(u_new[cc].abs());
            }
        }
        std::mem::swap(&mut u_old, &mut u);
        std::mem::swap(&mut u, &mut u_new);
    }

    let displacement_out: Vec<f32> = u.iter().map(|&x| x as f32).collect();
    let mut output = HashMap::new();
    output.insert("displacement_out".to_string(), displacement_out);

    let flops = steps as u64 * n as u64 * 12;
    SolverReport {
        output,
        iterations_used: steps,
        // Residual = peak amplitude growth above the initial unit amplitude
        // (≈0 for a stable standing wave; >0 would signal CFL instability).
        final_residual: (max_abs - 1.0).max(0.0) as f32,
        converged: max_abs < 1.2,
        flops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webgpu_extension::DispatchParams;

    fn analytic_ic(nx: usize, ny: usize) -> Vec<f64> {
        let hx = 2.0 * PI / nx as f64;
        let hy = 2.0 * PI / ny as f64;
        let mut v = vec![0.0f64; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                v[j * nx + i] = (i as f64 * hx).sin() * (j as f64 * hy).sin();
            }
        }
        v
    }

    fn correlation(a: &[f32], b: &[f64]) -> f64 {
        let dot: f64 = a.iter().zip(b).map(|(&x, &y)| x as f64 * y).sum();
        let na: f64 = a.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt();
        let nb: f64 = b.iter().map(|&y| y * y).sum::<f64>().sqrt();
        dot / (na * nb)
    }

    fn wave_params(steps: u32) -> WebGpuJobParams {
        let mut uniforms = HashMap::new();
        uniforms.insert("wave_speed".to_string(), 1.0);
        WebGpuJobParams {
            shader_name: "wave_equation_2d".to_string(),
            grid_size: (32, 32, 1),
            input_data: HashMap::new(),
            uniform_data: uniforms,
            dispatch_params: DispatchParams {
                iterations: steps,
                time_step: 0.05,
                convergence_threshold: 1e-3,
                max_execution_time_ms: 30_000,
            },
        }
    }

    #[test]
    fn standing_wave_inverts_after_half_period() {
        // ω = √2, T/2 = π/√2 ≈ 2.221 s. At Δt = 0.05 that is ~44 steps, where
        // cos(ωt) ≈ −1, so the field should be strongly anti-correlated with IC.
        let r = solve(&wave_params(44));
        let ic = analytic_ic(32, 32);
        let corr = correlation(&r.output["displacement_out"], &ic);
        assert!(
            corr < -0.8,
            "standing wave did not invert at T/2 (correlation {corr:.3})"
        );
    }

    #[test]
    fn leapfrog_is_stable_over_a_long_run() {
        let r = solve(&wave_params(500));
        assert!(r.converged, "wave solver went unstable: residual {}", r.final_residual);
        for &x in &r.output["displacement_out"] {
            assert!(x.is_finite());
        }
    }
}
