use super::*;

impl PhysicsSimulationLibrary {
    /// Integrate a first-order vector ODE `dy/dt = f(t, y)` from t=0 to `t_final`,
    /// delegating each sub-interval to the tested adaptive `integrate_dopri5`. Returns
    /// `(times, snapshots, accepted_steps, rejected_steps)` where `snapshots[k]` is the
    /// full state at `times[k]` (index 0 is the initial state).
    pub(super) fn integrate_ode_samples<F>(
        &self,
        mut state: Vec<f64>,
        t_final: f64,
        num_samples: usize,
        deriv: F,
    ) -> Result<(Vec<f64>, Vec<Vec<f64>>, u32, u32), PhysicsError>
    where
        F: Fn(f64, &[f64], &mut [f64]) -> Result<(), OdeError>,
    {
        let dim = state.len();
        if dim == 0 {
            return Err(PhysicsError::InvalidConfiguration(
                "empty initial state".to_string(),
            ));
        }
        if !(t_final > 0.0) || !t_final.is_finite() {
            return Err(PhysicsError::InvalidConfiguration(
                "t_final must be a positive finite number".to_string(),
            ));
        }
        let num_samples = num_samples.max(1);
        let mut workspace = vec![0.0f64; dim * 8];
        let mut times: Vec<f64> = Vec::with_capacity(num_samples + 1);
        let mut snapshots: Vec<Vec<f64>> = Vec::with_capacity(num_samples + 1);
        times.push(0.0);
        snapshots.push(state.clone());

        let mut accepted = 0u32;
        let mut rejected = 0u32;
        let dt_sample = t_final / num_samples as f64;
        let mut t0 = 0.0f64;
        for i in 0..num_samples {
            let t1 = if i + 1 == num_samples {
                t_final
            } else {
                (i + 1) as f64 * dt_sample
            };
            let mut cfg = AdaptiveOdeConfig::default();
            let span = t1 - t0;
            cfg.maximum_step = span.max(cfg.minimum_step);
            cfg.initial_step = (span / 100.0).clamp(cfg.minimum_step, cfg.maximum_step);
            let res = integrate_dopri5(&deriv, &mut state, t0, t1, cfg, &mut workspace)
                .map_err(|e| PhysicsError::SolverError(format!("dopri5 integration: {:?}", e)))?;
            accepted += res.accepted_steps;
            rejected += res.rejected_steps;
            times.push(t1);
            snapshots.push(state.clone());
            t0 = t1;
        }
        Ok((state, snapshots, accepted, rejected))
    }
}
