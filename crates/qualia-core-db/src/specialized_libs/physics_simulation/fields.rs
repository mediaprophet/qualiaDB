use super::*;

impl PhysicsSimulationLibrary {
    /// HeatTransfer — 1D heat/diffusion equation `u_t = α·u_xx` on a grid with insulated
    /// (Neumann) ends, so total heat is conserved and the profile relaxes toward its mean.
    /// The spatial Laplacian is assembled here; time integration is `integrate_dopri5`.
    pub fn run_heat_diffusion_1d(
        &self,
        initial: Vec<f64>,
        alpha: f64,
        dx: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<HeatDiffusionResult, PhysicsError> {
        let n = initial.len();
        if n < 3 || !(alpha > 0.0 && dx > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require grid length >= 3, alpha > 0, dx > 0, total_time > 0".to_string(),
            ));
        }
        let initial_mean = initial.iter().sum::<f64>() / n as f64;
        let inv_dx2 = 1.0 / (dx * dx);
        let deriv = move |_t: f64, u: &[f64], du: &mut [f64]| -> Result<(), OdeError> {
            // Conservative flux form: du_i = α·(F_{i+1/2} − F_{i-1/2})/dx² with
            // F_{i+1/2} = u_{i+1} − u_i, and zero flux at both insulated ends. Summing over
            // i telescopes to α·(F_{n-1/2} − F_{-1/2})/dx² = 0, so total heat is conserved.
            for i in 0..n {
                let flux_left = if i == 0 { 0.0 } else { u[i] - u[i - 1] };
                let flux_right = if i == n - 1 { 0.0 } else { u[i + 1] - u[i] };
                du[i] = alpha * (flux_right - flux_left) * inv_dx2;
            }
            Ok(())
        };
        let (final_temp, snapshots, accepted, rejected) =
            self.integrate_ode_samples(initial, total_time, num_samples, deriv)?;
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|k| total_time * k as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        let final_mean = final_temp.iter().sum::<f64>() / n as f64;
        let max_deviation_from_mean = final_temp
            .iter()
            .map(|&v| (v - final_mean).abs())
            .fold(0.0, f64::max);
        Ok(HeatDiffusionResult {
            times,
            snapshots,
            final_temperature: final_temp,
            initial_mean,
            final_mean,
            max_deviation_from_mean,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
    /// CEM — 1D scalar wave equation `u_tt = c²·u_xx` (a plane-wave field component) on a
    /// grid with fixed (Dirichlet) ends. Posed as the first-order system `u_t = v`,
    /// `v_t = c²·u_xx` and integrated by `integrate_dopri5`. Total wave energy is reported.
    pub fn run_wave_equation_1d(
        &self,
        initial_displacement: Vec<f64>,
        initial_velocity: Vec<f64>,
        c: f64,
        dx: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<WaveResult, PhysicsError> {
        let n = initial_displacement.len();
        if n < 3 || initial_velocity.len() != n || !(c > 0.0 && dx > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require matching grids length >= 3, c > 0, dx > 0, total_time > 0".to_string(),
            ));
        }
        let c2 = c * c;
        let inv_dx2 = 1.0 / (dx * dx);
        let mut state = Vec::with_capacity(2 * n);
        state.extend_from_slice(&initial_displacement);
        state.extend_from_slice(&initial_velocity);
        // Ends are pinned to zero.
        state[0] = 0.0;
        state[n - 1] = 0.0;
        state[n] = 0.0;
        state[2 * n - 1] = 0.0;

        let energy = |st: &[f64]| -> f64 {
            let mut e = 0.0;
            for i in 0..n {
                let v = st[n + i];
                e += 0.5 * v * v * dx;
            }
            for i in 0..n - 1 {
                let grad = (st[i + 1] - st[i]) / dx;
                e += 0.5 * c2 * grad * grad * dx;
            }
            e
        };
        let energy_initial = energy(&state);

        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            for i in 0..n {
                if i == 0 || i == n - 1 {
                    dy[i] = 0.0; // pinned displacement
                    dy[n + i] = 0.0; // pinned velocity
                } else {
                    dy[i] = y[n + i]; // u_t = v
                    dy[n + i] = c2 * (y[i + 1] - 2.0 * y[i] + y[i - 1]) * inv_dx2; // v_t = c² u_xx
                }
            }
            Ok(())
        };
        let (final_state, snapshots, accepted, rejected) =
            self.integrate_ode_samples(state, total_time, num_samples, deriv)?;
        let energy_final = energy(&final_state);
        let disp_snapshots: Vec<Vec<f64>> =
            snapshots.iter().map(|s| s[..n].to_vec()).collect();
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|k| total_time * k as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        Ok(WaveResult {
            times,
            snapshots: disp_snapshots,
            final_displacement: final_state[..n].to_vec(),
            energy_initial,
            energy_final,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
    /// MultiPhysics — coupled 1D advection–diffusion `u_t + c·u_x = α·u_xx` on a periodic
    /// grid: a prescribed flow (fluid transport) coupled to diffusion (thermal spreading).
    /// First-order upwind advection + central diffusion assembled here; integrated by
    /// `integrate_dopri5`. The periodic scheme conserves `Σ u_i·dx`; the pure-diffusion
    /// limit (`c = 0`) relaxes toward the mean.
    pub fn run_advection_diffusion_1d(
        &self,
        initial: Vec<f64>,
        advection_velocity: f64,
        diffusion_coeff: f64,
        dx: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<AdvectionDiffusionResult, PhysicsError> {
        let n = initial.len();
        if n < 3 || !(dx > 0.0 && total_time > 0.0) || diffusion_coeff < 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "require grid length >= 3, dx > 0, total_time > 0, diffusion_coeff >= 0"
                    .to_string(),
            ));
        }
        let c = advection_velocity;
        let alpha = diffusion_coeff;
        let inv_dx = 1.0 / dx;
        let inv_dx2 = 1.0 / (dx * dx);
        let initial_total = initial.iter().sum::<f64>() * dx;
        let deriv = move |_t: f64, u: &[f64], du: &mut [f64]| -> Result<(), OdeError> {
            for i in 0..n {
                let ip1 = (i + 1) % n;
                let im1 = (i + n - 1) % n;
                // First-order upwind advection (stable for either sign of c).
                let adv = if c >= 0.0 {
                    -c * (u[i] - u[im1]) * inv_dx
                } else {
                    -c * (u[ip1] - u[i]) * inv_dx
                };
                let diff = alpha * (u[ip1] - 2.0 * u[i] + u[im1]) * inv_dx2;
                du[i] = adv + diff;
            }
            Ok(())
        };
        let (final_field, snapshots, accepted, rejected) =
            self.integrate_ode_samples(initial, total_time, num_samples, deriv)?;
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|k| total_time * k as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        let final_total = final_field.iter().sum::<f64>() * dx;
        Ok(AdvectionDiffusionResult {
            times,
            snapshots,
            final_field,
            advection_velocity: c,
            diffusion_coeff: alpha,
            initial_total,
            final_total,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}
