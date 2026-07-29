use super::*;

/// Estimate a period from a sampled oscillatory signal by timing successive upward
/// crossings of its mean. Pure post-hoc analysis of the integrated result — not an
/// integrator. Returns 0.0 if fewer than two crossings were captured.
fn estimate_period_from_crossings(times: &[f64], values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let mut crossings: Vec<f64> = Vec::new();
    for i in 1..values.len() {
        let a = values[i - 1] - mean;
        let b = values[i] - mean;
        if a <= 0.0 && b > 0.0 {
            let denom = b - a;
            let frac = if denom.abs() > f64::MIN_POSITIVE {
                -a / denom
            } else {
                0.0
            };
            crossings.push(times[i - 1] + frac * (times[i] - times[i - 1]));
        }
    }
    if crossings.len() >= 2 {
        let total: f64 = crossings.windows(2).map(|w| w[1] - w[0]).sum();
        total / (crossings.len() - 1) as f64
    } else {
        0.0
    }
}

impl PhysicsSimulationLibrary {
    /// ParticlePhysics — 2D projectile / ballistic motion with optional quadratic drag.
    ///
    /// State `[x, y, vx, vy]`; `dvx = -k·|v|·vx`, `dvy = -g - k·|v|·vy` where `k = drag`
    /// (drag per unit mass). Integrated by `integrate_dopri5`. With `drag = 0` the range
    /// recovers the analytic `v0²·sin(2θ)/g`.
    pub fn run_projectile_motion(
        &self,
        v0: f64,
        angle_rad: f64,
        g: f64,
        drag: f64,
        num_samples: usize,
        max_time: f64,
    ) -> Result<ProjectileResult, PhysicsError> {
        if !(v0.is_finite() && angle_rad.is_finite() && g > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require finite v0, angle and g > 0".to_string(),
            ));
        }
        let state = vec![0.0, 0.0, v0 * angle_rad.cos(), v0 * angle_rad.sin()];
        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            let (vx, vy) = (y[2], y[3]);
            let speed = (vx * vx + vy * vy).sqrt();
            dy[0] = vx;
            dy[1] = vy;
            dy[2] = -drag * speed * vx;
            dy[3] = -g - drag * speed * vy;
            Ok(())
        };
        let (_final_state, snapshots, accepted, rejected) =
            self.integrate_ode_samples(state, max_time, num_samples, deriv)?;

        // Sample times are uniform across [0, max_time].
        let n = snapshots.len();
        let mut trajectory: Vec<[f64; 5]> = Vec::with_capacity(n);
        for (k, s) in snapshots.iter().enumerate() {
            let t = max_time * k as f64 / (n - 1).max(1) as f64;
            trajectory.push([t, s[0], s[1], s[2], s[3]]);
        }
        let max_height = trajectory.iter().map(|r| r[2]).fold(f64::MIN, f64::max);

        // Landing: first downward crossing of y = 0 after launch (skip the launch point).
        let mut landed = false;
        let mut range = trajectory.last().map(|r| r[1]).unwrap_or(0.0);
        let mut time_of_flight = max_time;
        for w in trajectory.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a[0] > 0.0 && a[2] >= 0.0 && b[2] < 0.0 {
                let frac = a[2] / (a[2] - b[2]); // linear interp to y=0
                range = a[1] + frac * (b[1] - a[1]);
                time_of_flight = a[0] + frac * (b[0] - a[0]);
                landed = true;
                break;
            }
        }
        Ok(ProjectileResult {
            trajectory,
            range,
            max_height,
            time_of_flight,
            landed,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
    /// StructuralDynamics — 1D spring–mass harmonic oscillator, integrated by the
    /// symplectic `integrate_symplectic` (Störmer–Verlet). Hamiltonian
    /// `H = p²/(2m) + ½k·q²`, so `dq/dt = p/m`, `dp/dt = -k·q`. Reports both the analytic
    /// period `2π√(m/k)` and the one measured from the integrated trajectory, plus the
    /// bounded energy drift that is the hallmark of a symplectic integrator.
    pub fn run_harmonic_oscillator(
        &self,
        mass: f64,
        k_spring: f64,
        x0: f64,
        v0: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<OscillatorResult, PhysicsError> {
        if !(mass > 0.0 && k_spring > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require mass > 0, k_spring > 0, total_time > 0".to_string(),
            ));
        }
        let num_samples = num_samples.max(1);
        let omega = (k_spring / mass).sqrt();
        let analytic_period = 2.0 * std::f64::consts::PI / omega;
        // Micro-step fine enough to resolve the period (~400 steps/period).
        let h_target = analytic_period / 400.0;

        let force = move |q: f64| -k_spring * q; // dp/dt
        let kinetic_velocity = move |p: f64| p / mass; // dq/dt
        let hamiltonian = move |q: f64, p: f64| 0.5 * p * p / mass + 0.5 * k_spring * q * q;

        let mut q = x0;
        let mut p = mass * v0;
        let energy_initial = hamiltonian(q, p);
        let mut times: Vec<f64> = Vec::with_capacity(num_samples + 1);
        let mut positions: Vec<f64> = Vec::with_capacity(num_samples + 1);
        let mut velocities: Vec<f64> = Vec::with_capacity(num_samples + 1);
        times.push(0.0);
        positions.push(q);
        velocities.push(p / mass);

        let dt_sample = total_time / num_samples as f64;
        let mut max_drift = 0.0f64;
        for i in 0..num_samples {
            let steps = (dt_sample / h_target).ceil().max(1.0) as u64;
            let h = dt_sample / steps as f64;
            let res = integrate_symplectic(
                q,
                p,
                h,
                steps,
                &force,
                &kinetic_velocity,
                &hamiltonian,
                SymplecticMethod::Yoshida4,
            );
            q = res.q;
            p = res.p;
            if res.max_energy_drift > max_drift {
                max_drift = res.max_energy_drift;
            }
            times.push((i + 1) as f64 * dt_sample);
            positions.push(q);
            velocities.push(p / mass);
        }
        let energy_final = hamiltonian(q, p);
        let measured_period = estimate_period_from_crossings(&times, &positions);
        Ok(OscillatorResult {
            times,
            positions,
            velocities,
            analytic_period,
            measured_period,
            energy_initial,
            energy_final,
            max_energy_drift: max_drift,
        })
    }
    /// Nonlinear rigid-body dynamics — a simple gravity pendulum (point mass on a rigid
    /// rod). State `[θ, ω]`; `dθ/dt = ω`, `dω/dt = -(g/L)·sin θ`. Integrated by
    /// `integrate_dopri5`. Energy `E = ½L²ω² + gL(1−cos θ)` (unit mass) is conserved; the
    /// small-angle period is `2π√(L/g)`.
    pub fn run_pendulum(
        &self,
        length: f64,
        g: f64,
        theta0: f64,
        omega0: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<PendulumResult, PhysicsError> {
        if !(length > 0.0 && g > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require length > 0, g > 0, total_time > 0".to_string(),
            ));
        }
        let l = length;
        let energy =
            move |theta: f64, omega: f64| 0.5 * l * l * omega * omega + g * l * (1.0 - theta.cos());
        let energy_initial = energy(theta0, omega0);
        let state = vec![theta0, omega0];
        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            dy[0] = y[1];
            dy[1] = -(g / l) * y[0].sin();
            Ok(())
        };
        let (final_state, snapshots, accepted, rejected) =
            self.integrate_ode_samples(state, total_time, num_samples, deriv)?;
        let n = snapshots.len();
        let times: Vec<f64> = (0..n)
            .map(|k| total_time * k as f64 / (n - 1).max(1) as f64)
            .collect();
        let angles: Vec<f64> = snapshots.iter().map(|s| s[0]).collect();
        let angular_velocities: Vec<f64> = snapshots.iter().map(|s| s[1]).collect();
        let energy_final = energy(final_state[0], final_state[1]);
        let measured_period = estimate_period_from_crossings(&times, &angles);
        Ok(PendulumResult {
            times,
            angles,
            angular_velocities,
            small_angle_period: 2.0 * std::f64::consts::PI * (l / g).sqrt(),
            measured_period,
            energy_initial,
            energy_final,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}
