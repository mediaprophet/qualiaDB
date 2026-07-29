use super::*;

impl PhysicsSimulationLibrary {
    /// Astrophysics — Newtonian N-body gravitation in 2D by direct force summation.
    ///
    /// `positions` and `velocities` are flat `[x0,y0,x1,y1,…]` (length `2·N`), `masses`
    /// length `N`. Accelerations `aᵢ = Σⱼ G·mⱼ·(rⱼ−rᵢ)/(|rⱼ−rᵢ|²+ε²)^{3/2}` are assembled
    /// here; the time integration is `integrate_dopri5`. Total energy and angular momentum
    /// are reported for conservation checks.
    pub fn run_nbody_gravitation(
        &self,
        masses: Vec<f64>,
        positions: Vec<f64>,
        velocities: Vec<f64>,
        g: f64,
        softening: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<NBodyResult, PhysicsError> {
        let n = masses.len();
        if n == 0 || positions.len() != 2 * n || velocities.len() != 2 * n {
            return Err(PhysicsError::InvalidConfiguration(
                "masses length N, positions/velocities length 2N required".to_string(),
            ));
        }
        let eps2 = softening * softening;
        let masses_e = masses.clone();
        // Layout: [pos(2N), vel(2N)].
        let mut state = Vec::with_capacity(4 * n);
        state.extend_from_slice(&positions);
        state.extend_from_slice(&velocities);

        let energy = |st: &[f64]| -> (f64, f64) {
            let mut ke = 0.0;
            let mut pe = 0.0;
            let mut angmom = 0.0;
            for i in 0..n {
                let (vx, vy) = (st[2 * n + 2 * i], st[2 * n + 2 * i + 1]);
                ke += 0.5 * masses[i] * (vx * vx + vy * vy);
                let (x, y) = (st[2 * i], st[2 * i + 1]);
                angmom += masses[i] * (x * vy - y * vx);
                for j in (i + 1)..n {
                    let dx = st[2 * j] - x;
                    let dy = st[2 * j + 1] - y;
                    let r = (dx * dx + dy * dy + eps2).sqrt();
                    pe -= g * masses[i] * masses[j] / r;
                }
            }
            (ke + pe, angmom)
        };
        let (energy_initial, angmom_initial) = energy(&state);

        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            // Positions' derivative = velocities.
            for i in 0..(2 * n) {
                dy[i] = y[2 * n + i];
            }
            // Velocities' derivative = accelerations (direct sum).
            for i in 0..n {
                let (xi, yi) = (y[2 * i], y[2 * i + 1]);
                let mut ax = 0.0;
                let mut ay = 0.0;
                for j in 0..n {
                    if j == i {
                        continue;
                    }
                    let dx = y[2 * j] - xi;
                    let dyj = y[2 * j + 1] - yi;
                    let r2 = dx * dx + dyj * dyj + eps2;
                    let inv_r3 = 1.0 / (r2 * r2.sqrt());
                    ax += g * masses_e[j] * dx * inv_r3;
                    ay += g * masses_e[j] * dyj * inv_r3;
                }
                dy[2 * n + 2 * i] = ax;
                dy[2 * n + 2 * i + 1] = ay;
            }
            Ok(())
        };
        let (final_state, snapshots, accepted, rejected) =
            self.integrate_ode_samples(state, total_time, num_samples, deriv)?;
        let (energy_final, angmom_final) = energy(&final_state);
        let position_snapshots: Vec<Vec<f64>> =
            snapshots.iter().map(|s| s[..2 * n].to_vec()).collect();
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|k| total_time * k as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        let energy_drift_rel = if energy_initial.abs() > f64::MIN_POSITIVE {
            (energy_final - energy_initial).abs() / energy_initial.abs()
        } else {
            (energy_final - energy_initial).abs()
        };
        Ok(NBodyResult {
            num_bodies: n,
            times,
            position_snapshots,
            final_positions: final_state[..2 * n].to_vec(),
            final_velocities: final_state[2 * n..].to_vec(),
            energy_initial,
            energy_final,
            energy_drift_rel,
            angular_momentum_initial: angmom_initial,
            angular_momentum_final: angmom_final,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}
