use super::*;

impl PhysicsSimulationLibrary {
    /// MolecularDynamics — 2D Lennard-Jones particles. `positions`/`velocities` flat
    /// `[x0,y0,…]` (length `2·N`). Pair potential `U(r)=4ε[(σ/r)¹²−(σ/r)⁶]`, force
    /// magnitude `24ε(2(σ/r)¹²−(σ/r)⁶)/r` assembled here; integrated by `integrate_dopri5`.
    /// Total energy is conserved; kinetic temperature reported in reduced units (kB=1).
    pub fn run_molecular_dynamics(
        &self,
        positions: Vec<f64>,
        velocities: Vec<f64>,
        epsilon: f64,
        sigma: f64,
        mass: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<MolecularDynamicsResult, PhysicsError> {
        if positions.len() < 2 || positions.len() % 2 != 0 {
            return Err(PhysicsError::InvalidConfiguration(
                "positions must be a non-empty flat 2D array (length 2N)".to_string(),
            ));
        }
        let n = positions.len() / 2;
        if velocities.len() != 2 * n
            || !(epsilon > 0.0 && sigma > 0.0 && mass > 0.0 && total_time > 0.0)
        {
            return Err(PhysicsError::InvalidConfiguration(
                "velocities length 2N; epsilon, sigma, mass, total_time > 0".to_string(),
            ));
        }
        let mut state = Vec::with_capacity(4 * n);
        state.extend_from_slice(&positions);
        state.extend_from_slice(&velocities);

        let pair_potential = move |r: f64| -> f64 {
            let sr6 = (sigma / r).powi(6);
            4.0 * epsilon * (sr6 * sr6 - sr6)
        };
        let energy = |st: &[f64]| -> (f64, f64) {
            let mut ke = 0.0;
            for i in 0..n {
                let vx = st[2 * n + 2 * i];
                let vy = st[2 * n + 2 * i + 1];
                ke += 0.5 * mass * (vx * vx + vy * vy);
            }
            let mut pe = 0.0;
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = st[2 * j] - st[2 * i];
                    let dy = st[2 * j + 1] - st[2 * i + 1];
                    let r = (dx * dx + dy * dy).sqrt();
                    if r > 0.0 {
                        pe += pair_potential(r);
                    }
                }
            }
            (ke, pe)
        };
        let (ke0, pe0) = energy(&state);
        let energy_initial = ke0 + pe0;

        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            for i in 0..(2 * n) {
                dy[i] = y[2 * n + i];
            }
            for i in 0..n {
                let (xi, yi) = (y[2 * i], y[2 * i + 1]);
                let mut fx = 0.0;
                let mut fy = 0.0;
                for j in 0..n {
                    if j == i {
                        continue;
                    }
                    let dx = xi - y[2 * j];
                    let dyj = yi - y[2 * j + 1];
                    let r2 = dx * dx + dyj * dyj;
                    if r2 <= 0.0 {
                        continue;
                    }
                    let r = r2.sqrt();
                    let sr6 = (sigma / r).powi(6);
                    // Repulsive force magnitude/r along the separation vector.
                    let f_over_r = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r2;
                    fx += f_over_r * dx;
                    fy += f_over_r * dyj;
                }
                dy[2 * n + 2 * i] = fx / mass;
                dy[2 * n + 2 * i + 1] = fy / mass;
            }
            Ok(())
        };
        let (final_state, _snapshots, accepted, rejected) =
            self.integrate_ode_samples(state, total_time, num_samples, deriv)?;
        let (ke_final, pe_final) = energy(&final_state);
        let energy_final = ke_final + pe_final;
        let energy_drift_rel = if energy_initial.abs() > f64::MIN_POSITIVE {
            (energy_final - energy_initial).abs() / energy_initial.abs()
        } else {
            (energy_final - energy_initial).abs()
        };
        // 2D kinetic temperature (reduced units, kB = 1): 2·KE / dof, dof = 2N.
        let temperature = 2.0 * ke_final / (2 * n) as f64;
        Ok(MolecularDynamicsResult {
            num_particles: n,
            times: (0..=num_samples.max(1))
                .map(|k| total_time * k as f64 / num_samples.max(1) as f64)
                .collect(),
            final_positions: final_state[..2 * n].to_vec(),
            final_velocities: final_state[2 * n..].to_vec(),
            energy_initial,
            energy_final,
            energy_drift_rel,
            temperature,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}
