//! 2D heat diffusion `∂u/∂t = α∇²u` by explicit finite differences (periodic).
//!
//! Validation: `u(x,y,0) = sin(x)·sin(y)` is an eigenfunction of the Laplacian
//! (`∇²u = −2u`), so it decays as `u(t) = u(0)·exp(-2αt)`. The test reproduces
//! that decay rate.

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
    let alpha = uniform(params, "diffusivity", 0.5).max(0.0) as f64;

    let mut dt = if params.dispatch_params.time_step > 0.0 {
        params.dispatch_params.time_step
    } else {
        0.01
    };
    if alpha > 0.0 {
        let dt_max = 0.25 * hx.min(hy).powi(2) / alpha;
        if dt > dt_max {
            dt = dt_max;
        }
    }
    let steps = if params.dispatch_params.iterations > 0 {
        params.dispatch_params.iterations
    } else {
        50
    };

    let idx = |i: usize, j: usize| j * nx + i;
    let mut u = vec![0.0f64; n];
    if let Some(field) = params.input_data.get("temperature_field") {
        for k in 0..n.min(field.len()) {
            u[k] = field[k] as f64;
        }
    } else {
        for j in 0..ny {
            for i in 0..nx {
                u[idx(i, j)] = (i as f64 * hx).sin() * (j as f64 * hy).sin();
            }
        }
    }

    let inv_hx2 = 1.0 / (hx * hx);
    let inv_hy2 = 1.0 / (hy * hy);
    let mut next = vec![0.0f64; n];

    for _ in 0..steps {
        for j in 0..ny {
            for i in 0..nx {
                let c = idx(i, j);
                let e = idx(wrap(i as i64 + 1, nxi), j);
                let w = idx(wrap(i as i64 - 1, nxi), j);
                let nn = idx(i, wrap(j as i64 + 1, nyi));
                let s = idx(i, wrap(j as i64 - 1, nyi));
                let lap = (u[e] + u[w] - 2.0 * u[c]) * inv_hx2
                    + (u[nn] + u[s] - 2.0 * u[c]) * inv_hy2;
                next[c] = u[c] + dt * alpha * lap;
            }
        }
        std::mem::swap(&mut u, &mut next);
    }

    let temperature_out: Vec<f32> = u.iter().map(|&x| x as f32).collect();
    let mut output = HashMap::new();
    output.insert("temperature_out".to_string(), temperature_out);

    let flops = steps as u64 * n as u64 * 10;
    SolverReport {
        output,
        iterations_used: steps,
        final_residual: 0.0,
        converged: true,
        flops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webgpu_extension::DispatchParams;

    fn rms(field: &[f32]) -> f64 {
        let s: f64 = field.iter().map(|&x| (x as f64) * (x as f64)).sum();
        (s / field.len() as f64).sqrt()
    }

    #[test]
    fn eigenmode_decays_at_analytic_rate() {
        // sin(x)sin(y) decays as exp(-2αt). α=0.5, Δt=0.01, 50 steps ⇒ t=0.5,
        // ratio exp(-0.5)≈0.6065. Initial RMS of sin·sin = sqrt(1/4) = 0.5.
        let alpha = 0.5;
        let dt = 0.01;
        let steps = 50;
        let mut uniforms = HashMap::new();
        uniforms.insert("diffusivity".to_string(), alpha);
        let p = WebGpuJobParams {
            shader_name: "heat_diffusion_2d".to_string(),
            grid_size: (32, 32, 1),
            input_data: HashMap::new(),
            uniform_data: uniforms,
            dispatch_params: DispatchParams {
                iterations: steps,
                time_step: dt,
                convergence_threshold: 1e-3,
                max_execution_time_ms: 30_000,
            },
        };
        let r = solve(&p);
        let final_rms = rms(&r.output["temperature_out"]);
        let expected = 0.5 * (-2.0 * alpha as f64 * dt * steps as f64).exp();
        let rel = (final_rms - expected).abs() / expected;
        assert!(
            rel < 0.05,
            "heat decay off: got {final_rms:.5}, expected {expected:.5} (rel {rel:.3})"
        );
    }
}
