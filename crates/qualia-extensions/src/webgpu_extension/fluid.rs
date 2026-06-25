//! Incompressible 2D Navier–Stokes (periodic) via Chorin's projection method.
//!
//! Each step: advect + diffuse to an intermediate velocity `u*`, solve the
//! pressure-Poisson equation `∇²p = (1/Δt)∇·u*` (Jacobi), then project
//! `u = u* − Δt∇p` so the field stays divergence-free.
//!
//! Validation: the **Taylor–Green vortex** is an exact solution of the
//! incompressible NS equations whose amplitude decays as `exp(-2νt)` (the
//! nonlinear advection is exactly balanced by the pressure gradient, leaving
//! pure viscous diffusion). The unit test reproduces that decay rate.

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

    let lx = 2.0 * PI;
    let ly = 2.0 * PI;
    let hx = lx / nx as f64;
    let hy = ly / ny as f64;

    let nu = uniform(params, "viscosity", 0.05).max(0.0) as f64;
    let mut dt = if params.dispatch_params.time_step > 0.0 {
        params.dispatch_params.time_step
    } else {
        0.01
    };
    // Explicit-diffusion stability: Δt ≤ 0.25·min(hx,hy)² / ν.
    if nu > 0.0 {
        let dt_max = 0.25 * hx.min(hy).powi(2) / nu;
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

    // Initial condition: caller-provided interleaved vec2 field, else the
    // analytic Taylor–Green vortex.
    let mut u = vec![0.0f64; n];
    let mut v = vec![0.0f64; n];
    if let Some(field) = params.input_data.get("velocity_field") {
        for k in 0..n.min(field.len() / 2) {
            u[k] = field[2 * k] as f64;
            v[k] = field[2 * k + 1] as f64;
        }
    } else {
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f64 * hx;
                let y = j as f64 * hy;
                u[idx(i, j)] = x.sin() * y.cos();
                v[idx(i, j)] = -x.cos() * y.sin();
            }
        }
    }

    let mut ustar = vec![0.0f64; n];
    let mut vstar = vec![0.0f64; n];
    let mut p = vec![0.0f64; n];
    let mut p_next = vec![0.0f64; n];
    let mut rhs = vec![0.0f64; n];

    let inv_hx2 = 1.0 / (hx * hx);
    let inv_hy2 = 1.0 / (hy * hy);
    let denom = 2.0 * (inv_hx2 + inv_hy2);
    let jacobi_iters = 60usize;

    for _ in 0..steps {
        // 1) Advection (central) + viscous diffusion → intermediate velocity.
        for j in 0..ny {
            for i in 0..nx {
                let c = idx(i, j);
                let e = idx(wrap(i as i64 + 1, nxi), j);
                let w = idx(wrap(i as i64 - 1, nxi), j);
                let nn = idx(i, wrap(j as i64 + 1, nyi));
                let s = idx(i, wrap(j as i64 - 1, nyi));

                let dudx = (u[e] - u[w]) / (2.0 * hx);
                let dudy = (u[nn] - u[s]) / (2.0 * hy);
                let dvdx = (v[e] - v[w]) / (2.0 * hx);
                let dvdy = (v[nn] - v[s]) / (2.0 * hy);

                let lap_u = (u[e] + u[w] - 2.0 * u[c]) * inv_hx2
                    + (u[nn] + u[s] - 2.0 * u[c]) * inv_hy2;
                let lap_v = (v[e] + v[w] - 2.0 * v[c]) * inv_hx2
                    + (v[nn] + v[s] - 2.0 * v[c]) * inv_hy2;

                let adv_u = u[c] * dudx + v[c] * dudy;
                let adv_v = u[c] * dvdx + v[c] * dvdy;

                ustar[c] = u[c] + dt * (-adv_u + nu * lap_u);
                vstar[c] = v[c] + dt * (-adv_v + nu * lap_v);
            }
        }

        // 2) Pressure-Poisson RHS = (1/Δt)·∇·u*.
        for j in 0..ny {
            for i in 0..nx {
                let e = idx(wrap(i as i64 + 1, nxi), j);
                let w = idx(wrap(i as i64 - 1, nxi), j);
                let nn = idx(i, wrap(j as i64 + 1, nyi));
                let s = idx(i, wrap(j as i64 - 1, nyi));
                let div = (ustar[e] - ustar[w]) / (2.0 * hx)
                    + (vstar[nn] - vstar[s]) / (2.0 * hy);
                rhs[idx(i, j)] = div / dt;
            }
        }

        // Jacobi solve of ∇²p = rhs (periodic).
        for it in 0..jacobi_iters {
            for j in 0..ny {
                for i in 0..nx {
                    let c = idx(i, j);
                    let e = idx(wrap(i as i64 + 1, nxi), j);
                    let w = idx(wrap(i as i64 - 1, nxi), j);
                    let nn = idx(i, wrap(j as i64 + 1, nyi));
                    let s = idx(i, wrap(j as i64 - 1, nyi));
                    p_next[c] = ((p[e] + p[w]) * inv_hx2 + (p[nn] + p[s]) * inv_hy2 - rhs[c])
                        / denom;
                }
            }
            std::mem::swap(&mut p, &mut p_next);
            // Pin the periodic null space (constant offset) on the last sweep.
            if it == jacobi_iters - 1 {
                let mean: f64 = p.iter().sum::<f64>() / n as f64;
                for pv in p.iter_mut() {
                    *pv -= mean;
                }
            }
        }

        // 3) Project: u = u* − Δt·∇p.
        for j in 0..ny {
            for i in 0..nx {
                let c = idx(i, j);
                let e = idx(wrap(i as i64 + 1, nxi), j);
                let w = idx(wrap(i as i64 - 1, nxi), j);
                let nn = idx(i, wrap(j as i64 + 1, nyi));
                let s = idx(i, wrap(j as i64 - 1, nyi));
                u[c] = ustar[c] - dt * (p[e] - p[w]) / (2.0 * hx);
                v[c] = vstar[c] - dt * (p[nn] - p[s]) / (2.0 * hy);
            }
        }
    }

    // Residual = max incompressibility error |∇·u| after projection.
    let mut max_div = 0.0f64;
    for j in 0..ny {
        for i in 0..nx {
            let e = idx(wrap(i as i64 + 1, nxi), j);
            let w = idx(wrap(i as i64 - 1, nxi), j);
            let nn = idx(i, wrap(j as i64 + 1, nyi));
            let s = idx(i, wrap(j as i64 - 1, nyi));
            let div =
                (u[e] - u[w]) / (2.0 * hx) + (v[nn] - v[s]) / (2.0 * hy);
            max_div = max_div.max(div.abs());
        }
    }

    let mut velocity_out = Vec::with_capacity(n * 2);
    for k in 0..n {
        velocity_out.push(u[k] as f32);
        velocity_out.push(v[k] as f32);
    }
    let mut output = HashMap::new();
    output.insert("velocity_out".to_string(), velocity_out);

    let threshold = params.dispatch_params.convergence_threshold.max(1e-2);
    // Per-step cost: ~30 FLOPs/cell (advect+diffuse) + jacobi sweeps + project.
    let flops = steps as u64 * n as u64 * (30 + jacobi_iters as u64 * 8 + 8);

    SolverReport {
        output,
        iterations_used: steps,
        final_residual: max_div as f32,
        converged: (max_div as f32) <= threshold,
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

    fn tg_params(nu: f32, dt: f64, steps: u32) -> WebGpuJobParams {
        let mut uniforms = HashMap::new();
        uniforms.insert("viscosity".to_string(), nu);
        WebGpuJobParams {
            shader_name: "navier_stokes_2d".to_string(),
            grid_size: (32, 32, 1),
            input_data: HashMap::new(),
            uniform_data: uniforms,
            dispatch_params: DispatchParams {
                iterations: steps,
                time_step: dt,
                convergence_threshold: 0.1,
                max_execution_time_ms: 30_000,
            },
        }
    }

    #[test]
    fn taylor_green_vortex_decays_at_analytic_rate() {
        // ν = 0.5, Δt = 0.01, 50 steps ⇒ t = 0.5; analytic amplitude ratio is
        // exp(-2·ν·t) = exp(-0.5) ≈ 0.6065. Initial TG RMS = sqrt(1/2) ≈ 0.7071.
        let nu = 0.5;
        let dt = 0.01;
        let steps = 50;
        let r = solve(&tg_params(nu, dt, steps));

        let final_rms = rms(&r.output["velocity_out"]);
        let t = dt * steps as f64;
        // RMS over the interleaved (u,v) array = sqrt((mean u² + mean v²)/2)
        // = sqrt((1/4 + 1/4)/... ) = 0.5 for the unit-amplitude TG vortex.
        let expected = 0.5 * (-2.0 * nu as f64 * t).exp();

        let rel_err = (final_rms - expected).abs() / expected;
        assert!(
            rel_err < 0.06,
            "TG decay off: got rms {final_rms:.5}, expected {expected:.5} (rel {rel_err:.3})"
        );
    }

    #[test]
    fn projection_keeps_the_field_divergence_free() {
        let r = solve(&tg_params(0.5, 0.01, 50));
        assert!(
            r.final_residual < 0.1,
            "incompressibility residual too large: {}",
            r.final_residual
        );
        assert!(r.converged);
    }

    #[test]
    fn inviscid_vortex_barely_decays() {
        // With ν = 0 the Taylor–Green amplitude should be (nearly) conserved.
        let r = solve(&tg_params(0.0, 0.005, 40));
        let final_rms = rms(&r.output["velocity_out"]);
        let init = 0.5; // interleaved-array RMS of the unit-amplitude TG vortex
        assert!(
            (final_rms - init).abs() / init < 0.05,
            "inviscid amplitude drifted: {final_rms:.5} vs {init:.5}"
        );
    }
}
