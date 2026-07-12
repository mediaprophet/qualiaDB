use super::*;

impl PhysicsSimulationLibrary {
    /// Biophysics — logistic population dynamics `dN/dt = r·N·(1 − N/K)`, integrated by
    /// `integrate_dopri5`. Matches the analytic logistic curve
    /// `N(t) = K / (1 + ((K−N₀)/N₀)·e^{−r·t})`.
    pub fn run_logistic_growth(
        &self,
        n0: f64,
        growth_rate: f64,
        carrying_capacity: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<PopulationDynamicsResult, PhysicsError> {
        if !(n0 >= 0.0 && carrying_capacity > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require n0 >= 0, carrying_capacity > 0, total_time > 0".to_string(),
            ));
        }
        let r = growth_rate;
        let k = carrying_capacity;
        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            dy[0] = r * y[0] * (1.0 - y[0] / k);
            Ok(())
        };
        let (_final, snapshots, accepted, rejected) =
            self.integrate_ode_samples(vec![n0], total_time, num_samples, deriv)?;
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|kk| total_time * kk as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        let population: Vec<f64> = snapshots.iter().map(|s| s[0]).collect();
        Ok(PopulationDynamicsResult {
            times,
            population,
            carrying_capacity: k,
            growth_rate: r,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}
