//! Gravitational N-body simulation by **velocity-Verlet** (symplectic), 2D.
//!
//! Pairwise softened gravity `a_i = G·Σ_{j≠i} m_j (r_j−r_i)/(|r_j−r_i|²+ε²)^{3/2}`.
//!
//! Validation: a symplectic integrator conserves total energy
//! `E = ½Σm|v|² − ½ΣΣ G m_i m_j /√(|r_i−r_j|²+ε²)`. The test runs a small
//! cluster and asserts the energy drift stays well under 1%.

use super::{dim, uniform, SolverReport, WebGpuJobParams};
use std::collections::HashMap;
use std::f64::consts::PI;

struct System {
    px: Vec<f64>,
    py: Vec<f64>,
    vx: Vec<f64>,
    vy: Vec<f64>,
    m: Vec<f64>,
    g: f64,
    soft2: f64,
}

impl System {
    fn accel(&self, ax: &mut [f64], ay: &mut [f64]) {
        let n = self.px.len();
        for a in ax.iter_mut() {
            *a = 0.0;
        }
        for a in ay.iter_mut() {
            *a = 0.0;
        }
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.px[j] - self.px[i];
                let dy = self.py[j] - self.py[i];
                let r2 = dx * dx + dy * dy + self.soft2;
                let inv_r3 = 1.0 / (r2 * r2.sqrt());
                let fx = self.g * dx * inv_r3;
                let fy = self.g * dy * inv_r3;
                ax[i] += self.m[j] * fx;
                ay[i] += self.m[j] * fy;
                ax[j] -= self.m[i] * fx;
                ay[j] -= self.m[i] * fy;
            }
        }
    }

    fn energy(&self) -> f64 {
        let n = self.px.len();
        let mut ke = 0.0;
        for i in 0..n {
            ke += 0.5 * self.m[i] * (self.vx[i] * self.vx[i] + self.vy[i] * self.vy[i]);
        }
        let mut pe = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.px[j] - self.px[i];
                let dy = self.py[j] - self.py[i];
                let r = (dx * dx + dy * dy + self.soft2).sqrt();
                pe -= self.g * self.m[i] * self.m[j] / r;
            }
        }
        ke + pe
    }
}

pub fn solve(params: &WebGpuJobParams) -> SolverReport {
    let n = dim(params.grid_size.0, 16).max(2);
    let g = uniform(params, "gravity", 1.0) as f64;
    let soft = uniform(params, "softening", 0.5).max(1e-3) as f64;
    let dt = if params.dispatch_params.time_step > 0.0 {
        params.dispatch_params.time_step
    } else {
        0.001
    };
    let steps = if params.dispatch_params.iterations > 0 {
        params.dispatch_params.iterations
    } else {
        500
    };

    // Deterministic initial condition: a ring of equal masses with a small
    // tangential velocity (caller may override via input fields).
    let mut sys = System {
        px: vec![0.0; n],
        py: vec![0.0; n],
        vx: vec![0.0; n],
        vy: vec![0.0; n],
        m: vec![1.0; n],
        g,
        soft2: soft * soft,
    };
    if let (Some(pos), Some(vel)) = (
        params.input_data.get("positions"),
        params.input_data.get("velocities"),
    ) {
        for i in 0..n.min(pos.len() / 2) {
            sys.px[i] = pos[2 * i] as f64;
            sys.py[i] = pos[2 * i + 1] as f64;
        }
        for i in 0..n.min(vel.len() / 2) {
            sys.vx[i] = vel[2 * i] as f64;
            sys.vy[i] = vel[2 * i + 1] as f64;
        }
    } else {
        let r = 1.0;
        let vt = 0.2;
        for i in 0..n {
            let theta = 2.0 * PI * i as f64 / n as f64;
            sys.px[i] = r * theta.cos();
            sys.py[i] = r * theta.sin();
            sys.vx[i] = -vt * theta.sin();
            sys.vy[i] = vt * theta.cos();
        }
    }

    let e0 = sys.energy();

    let mut ax = vec![0.0f64; n];
    let mut ay = vec![0.0f64; n];
    let mut ax_new = vec![0.0f64; n];
    let mut ay_new = vec![0.0f64; n];
    sys.accel(&mut ax, &mut ay);

    for _ in 0..steps {
        // r += v·Δt + ½a·Δt²
        for i in 0..n {
            sys.px[i] += sys.vx[i] * dt + 0.5 * ax[i] * dt * dt;
            sys.py[i] += sys.vy[i] * dt + 0.5 * ay[i] * dt * dt;
        }
        sys.accel(&mut ax_new, &mut ay_new);
        // v += ½(a_old + a_new)·Δt
        for i in 0..n {
            sys.vx[i] += 0.5 * (ax[i] + ax_new[i]) * dt;
            sys.vy[i] += 0.5 * (ay[i] + ay_new[i]) * dt;
        }
        std::mem::swap(&mut ax, &mut ax_new);
        std::mem::swap(&mut ay, &mut ay_new);
    }

    let e1 = sys.energy();
    let residual = if e0.abs() > 0.0 {
        ((e1 - e0) / e0).abs() as f32
    } else {
        (e1 - e0).abs() as f32
    };

    let mut positions_out = Vec::with_capacity(n * 2);
    let mut velocities_out = Vec::with_capacity(n * 2);
    for i in 0..n {
        positions_out.push(sys.px[i] as f32);
        positions_out.push(sys.py[i] as f32);
        velocities_out.push(sys.vx[i] as f32);
        velocities_out.push(sys.vy[i] as f32);
    }
    let mut output = HashMap::new();
    output.insert("positions_out".to_string(), positions_out);
    output.insert("velocities_out".to_string(), velocities_out);

    let threshold = params.dispatch_params.convergence_threshold.max(0.01);
    let flops = steps as u64 * (n as u64) * (n as u64) * 15;
    SolverReport {
        output,
        iterations_used: steps,
        final_residual: residual,
        converged: residual <= threshold,
        flops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webgpu_extension::DispatchParams;

    #[test]
    fn symplectic_integrator_conserves_energy() {
        let p = WebGpuJobParams {
            shader_name: "nbody_verlet".to_string(),
            grid_size: (16, 1, 1),
            input_data: HashMap::new(),
            uniform_data: HashMap::new(),
            dispatch_params: DispatchParams {
                iterations: 1000,
                time_step: 0.002,
                convergence_threshold: 0.01,
                max_execution_time_ms: 30_000,
            },
        };
        let r = solve(&p);
        assert!(
            r.final_residual < 0.01,
            "energy drift too large: {}",
            r.final_residual
        );
        assert!(r.converged);
    }

    #[test]
    fn two_body_orbit_stays_bounded() {
        // Two equal masses given a circular-ish velocity should stay near their
        // initial separation (no runaway), with conserved energy.
        let mut pos = HashMap::new();
        pos.insert("positions".to_string(), vec![-1.0f32, 0.0, 1.0, 0.0]);
        pos.insert("velocities".to_string(), vec![0.0f32, -0.3, 0.0, 0.3]);
        let p = WebGpuJobParams {
            shader_name: "nbody_verlet".to_string(),
            grid_size: (2, 1, 1),
            input_data: pos,
            uniform_data: HashMap::new(),
            dispatch_params: DispatchParams {
                iterations: 2000,
                time_step: 0.001,
                convergence_threshold: 0.02,
                max_execution_time_ms: 30_000,
            },
        };
        let r = solve(&p);
        assert!(
            r.final_residual < 0.02,
            "orbit energy drift {}",
            r.final_residual
        );
        let pos = &r.output["positions_out"];
        let sep = (((pos[0] - pos[2]).powi(2) + (pos[1] - pos[3]).powi(2)) as f64).sqrt();
        assert!(sep > 0.2 && sep < 5.0, "orbit separation ran away: {sep}");
    }
}
