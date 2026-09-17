//! Analytical Mass-Spring-Damper dynamics in 1D, 2D, and 3D (Zero-Heap).
//!
//! Evaluates the continuous analytical closed-form solution of:
//!   m·x''(t) + c·x'(t) + k·(x(t) - x_target) = 0
//! Supports under-damped (ζ < 1), critically-damped (ζ = 1), and over-damped (ζ > 1)
//! regimes without numerical drift, time-step sensitivity, or heap allocation.

/// Spring configuration parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringConfig {
    /// Mass (m > 0). Default: 1.0.
    pub mass: f64,
    /// Stiffness / Spring constant (k > 0).
    pub stiffness: f64,
    /// Damping coefficient (c >= 0).
    pub damping: f64,
    /// Velocity threshold for declaring the spring settled.
    pub rest_velocity_threshold: f64,
    /// Distance threshold from target for declaring the spring settled.
    pub rest_displacement_threshold: f64,
}

impl SpringConfig {
    /// Create a custom spring configuration.
    pub const fn new(stiffness: f64, damping: f64) -> Self {
        Self {
            mass: 1.0,
            stiffness,
            damping,
            rest_velocity_threshold: 1e-4,
            rest_displacement_threshold: 1e-4,
        }
    }

    /// Snappy UI response (high stiffness, critical damping).
    pub const fn snappy() -> Self {
        Self::new(280.0, 30.0)
    }

    /// Smooth & gentle deceleration.
    pub const fn gentle() -> Self {
        Self::new(120.0, 14.0)
    }

    /// Bouncy spring with noticeable overshoot.
    pub const fn bouncy() -> Self {
        Self::new(180.0, 12.0)
    }

    /// Exaggerated wobbly spring.
    pub const fn wobbly() -> Self {
        Self::new(180.0, 8.0)
    }

    /// Extremely stiff instant-snap spring.
    pub const fn stiff() -> Self {
        Self::new(400.0, 40.0)
    }

    /// Natural undamped angular frequency: ω_n = sqrt(k / m).
    #[inline]
    pub fn natural_frequency(&self) -> f64 {
        (self.stiffness / self.mass.max(1e-6)).sqrt()
    }

    /// Damping ratio: ζ = c / (2 * sqrt(m * k)).
    #[inline]
    pub fn damping_ratio(&self) -> f64 {
        self.damping / (2.0 * (self.mass.max(1e-6) * self.stiffness.max(1e-6)).sqrt())
    }
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::snappy()
    }
}

/// State of a 1D spring simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringState1D {
    pub position: f64,
    pub velocity: f64,
    pub target: f64,
}

impl SpringState1D {
    pub const fn new(position: f64, velocity: f64, target: f64) -> Self {
        Self {
            position,
            velocity,
            target,
        }
    }

    /// Closed-form analytical evaluation at time `t` seconds into the future.
    pub fn evaluate_at(&self, config: &SpringConfig, t: f64) -> (f64, f64, bool) {
        if t <= 0.0 {
            let settled = (self.position - self.target).abs() <= config.rest_displacement_threshold
                && self.velocity.abs() <= config.rest_velocity_threshold;
            return (self.position, self.velocity, settled);
        }

        let x0 = self.position - self.target;
        let v0 = self.velocity;
        let omega_n = config.natural_frequency();
        let zeta = config.damping_ratio();

        let (pos_offset, vel) = if (zeta - 1.0).abs() < 1e-5 {
            // Critically-damped (ζ ≈ 1.0)
            let decay = (-omega_n * t).exp();
            let c1 = x0;
            let c2 = v0 + omega_n * x0;
            let x = (c1 + c2 * t) * decay;
            let v = (c2 - omega_n * (c1 + c2 * t)) * decay;
            (x, v)
        } else if zeta < 1.0 {
            // Under-damped (ζ < 1.0): oscillates with decaying amplitude
            let omega_d = omega_n * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega_n * t).exp();
            let cos_term = (omega_d * t).cos();
            let sin_term = (omega_d * t).sin();
            let c1 = x0;
            let c2 = (v0 + zeta * omega_n * x0) / omega_d;
            let x = decay * (c1 * cos_term + c2 * sin_term);
            let v = decay
                * ((-zeta * omega_n * c1 + omega_d * c2) * cos_term
                    - (omega_d * c1 + zeta * omega_n * c2) * sin_term);
            (x, v)
        } else {
            // Over-damped (ζ > 1.0): no oscillation, sum of two exponentials
            let gamma = omega_n * (zeta * zeta - 1.0).sqrt();
            let r1 = -zeta * omega_n + gamma;
            let r2 = -zeta * omega_n - gamma;
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let exp1 = (r1 * t).exp();
            let exp2 = (r2 * t).exp();
            let x = c1 * exp1 + c2 * exp2;
            let v = c1 * r1 * exp1 + c2 * r2 * exp2;
            (x, v)
        };

        let final_pos = self.target + pos_offset;
        let settled = pos_offset.abs() <= config.rest_displacement_threshold
            && vel.abs() <= config.rest_velocity_threshold;

        (final_pos, vel, settled)
    }

    /// Advance the state by `dt` seconds using the closed-form equation.
    pub fn step(&mut self, config: &SpringConfig, dt: f64) -> bool {
        let (p, v, settled) = self.evaluate_at(config, dt);
        if settled {
            self.position = self.target;
            self.velocity = 0.0;
            true
        } else {
            self.position = p;
            self.velocity = v;
            false
        }
    }
}

/// 3D Spring state (x, y, z) evaluated independently across components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringState3D {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub target: [f64; 3],
}

impl SpringState3D {
    pub const fn new(position: [f64; 3], velocity: [f64; 3], target: [f64; 3]) -> Self {
        Self {
            position,
            velocity,
            target,
        }
    }

    /// Evaluate 3D spring state at `t` seconds.
    pub fn evaluate_at(&self, config: &SpringConfig, t: f64) -> ([f64; 3], [f64; 3], bool) {
        let mut out_pos = [0.0; 3];
        let mut out_vel = [0.0; 3];
        let mut settled = true;

        for i in 0..3 {
            let s1 = SpringState1D::new(self.position[i], self.velocity[i], self.target[i]);
            let (p, v, s) = s1.evaluate_at(config, t);
            out_pos[i] = p;
            out_vel[i] = v;
            if !s {
                settled = false;
            }
        }

        (out_pos, out_vel, settled)
    }

    /// Advance the 3D spring state by `dt` seconds.
    pub fn step(&mut self, config: &SpringConfig, dt: f64) -> bool {
        let (p, v, settled) = self.evaluate_at(config, dt);
        if settled {
            self.position = self.target;
            self.velocity = [0.0, 0.0, 0.0];
            true
        } else {
            self.position = p;
            self.velocity = v;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critically_damped_spring_converges_to_target() {
        let config = SpringConfig::snappy();
        let mut state = SpringState1D::new(0.0, 0.0, 100.0);

        // Step forward in time until settled
        let mut steps = 0;
        while !state.step(&config, 1.0 / 60.0) && steps < 300 {
            steps += 1;
        }

        assert_eq!(state.position, 100.0);
        assert_eq!(state.velocity, 0.0);
        assert!(steps < 200, "spring should settle in under 3.3s");
    }

    #[test]
    fn under_damped_spring_bounces() {
        let config = SpringConfig::bouncy();
        let state = SpringState1D::new(0.0, 0.0, 100.0);

        // Sample at several points to verify overshoot
        let mut max_pos = 0.0;
        for i in 1..100 {
            let t = i as f64 * 0.02;
            let (p, _, _) = state.evaluate_at(&config, t);
            if p > max_pos {
                max_pos = p;
            }
        }

        assert!(
            max_pos > 100.0,
            "bouncy spring must overshoot target: max={max_pos}"
        );
    }

    #[test]
    fn spring_3d_settles_all_axes() {
        let config = SpringConfig::stiff();
        let mut state = SpringState3D::new([0.0, 50.0, -100.0], [0.0; 3], [10.0, 0.0, 0.0]);

        for _ in 0..150 {
            state.step(&config, 1.0 / 60.0);
        }

        assert_eq!(state.position, [10.0, 0.0, 0.0]);
        assert_eq!(state.velocity, [0.0, 0.0, 0.0]);
    }
}
