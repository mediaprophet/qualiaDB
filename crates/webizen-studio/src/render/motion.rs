//! Critically damped spring physics for calm, non-distracting UI motion
//!
//! This module provides zero-heap spring physics for smooth UI transitions
//! that respect user accessibility preferences (prefers-reduced-motion).

/// Global tick driven by requestAnimationFrame or native window loop
#[derive(Debug, Clone, Copy)]
pub struct Timeline {
    pub current_time: f64,
    pub delta_t: f64,
    pub reduced_motion: bool,
}

/// A zero-heap, critically damped spring for calm, non-distracting UI motion
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f64,
    pub velocity: f64,
    pub target: f64,
    stiffness: f64,
    damping: f64,
}

impl Spring {
    /// Create a new spring with initial value
    pub fn new(initial: f64) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            // Critically damped defaults: stiffness = k, damping = 2 * sqrt(k)
            stiffness: 100.0,
            damping: 20.0,
        }
    }

    /// Set the target value for the spring
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    /// Advance the spring physics by one step
    /// If reduced_motion is true, snaps instantly to target
    pub fn step(&mut self, timeline: &Timeline) -> f64 {
        if timeline.reduced_motion {
            self.value = self.target;
            self.velocity = 0.0;
            return self.value;
        }

        // Implicit Euler integration for stability
        let displacement = self.target - self.value;
        let spring_force = displacement * self.stiffness;
        let damping_force = -self.velocity * self.damping;
        let acceleration = spring_force + damping_force;

        self.velocity += acceleration * timeline.delta_t;
        self.value += self.velocity * timeline.delta_t;

        self.value
    }

    /// Check if the spring has effectively reached its target
    pub fn is_at_target(&self, tolerance: f64) -> bool {
        (self.value - self.target).abs() < tolerance && self.velocity.abs() < tolerance
    }
}
