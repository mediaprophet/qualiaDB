//! Deterministic Fixed-Timestep Simulation Accumulator (64 Hz).
//!
//! $1/64 = 0.015625\text{ s}$ ($2^{-6}$) is an exact binary floating-point representation,
//! eliminating accumulator truncation drift across long simulation runs and preventing
//! display refresh rate beating.

pub const FIXED_HZ: f64 = 64.0;
pub const FIXED_DELTA_SECONDS: f64 = 1.0 / FIXED_HZ; // Exact: 0.015625
pub const FIXED_DELTA_NANOS: u64 = 15_625_000;
pub const DEFAULT_MAX_SUBSTEPS: usize = 8; // Max 125ms per frame to prevent spiral-of-death

/// 64 Hz Fixed Timestep Accumulator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedTimestepAccumulator {
    /// Fixed delta time per step in seconds ($0.015625$)
    pub fixed_delta: f64,
    /// Accumulated real time in seconds awaiting consumption
    pub accumulator: f64,
    /// Maximum substeps allowed in a single frame update
    pub max_substeps: usize,
    /// Total fixed steps executed since initialization
    pub total_steps: u64,
    /// Elapsed simulation time in seconds ($total\_steps \times fixed\_delta$)
    pub elapsed_sim_time: f64,
}

impl Default for FixedTimestepAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedTimestepAccumulator {
    pub const fn new() -> Self {
        Self {
            fixed_delta: FIXED_DELTA_SECONDS,
            accumulator: 0.0,
            max_substeps: DEFAULT_MAX_SUBSTEPS,
            total_steps: 0,
            elapsed_sim_time: 0.0,
        }
    }

    /// Feeds raw frame delta time into the accumulator.
    ///
    /// Returns the number of fixed 64 Hz simulation steps that must be executed this frame.
    pub fn advance(&mut self, raw_delta_seconds: f64) -> usize {
        // Clamp incoming delta time to prevent spiral of death on backgrounding / freezes
        let max_accum = self.fixed_delta * (self.max_substeps as f64);
        let clamped_delta = raw_delta_seconds.min(max_accum);
        self.accumulator += clamped_delta;

        let mut steps = 0;
        while self.accumulator >= self.fixed_delta && steps < self.max_substeps {
            self.accumulator -= self.fixed_delta;
            self.total_steps += 1;
            self.elapsed_sim_time += self.fixed_delta;
            steps += 1;
        }

        // Drop remaining excess if still over budget
        if self.accumulator > max_accum {
            self.accumulator = 0.0;
        }

        steps
    }

    /// Returns the fractional blend factor $\alpha \in [0, 1)$ between the previous
    /// and current simulation states for renderer interpolation.
    #[inline]
    pub fn interpolation_alpha(&self) -> f32 {
        (self.accumulator / self.fixed_delta).clamp(0.0, 1.0) as f32
    }

    /// Resets accumulator state.
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.total_steps = 0;
        self.elapsed_sim_time = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_binary_representation() {
        assert_eq!(FIXED_DELTA_SECONDS, 0.015625);
        // Ensure no float precision loss
        let sum = (0..64).fold(0.0f64, |acc, _| acc + FIXED_DELTA_SECONDS);
        assert!((sum - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_accumulator_steps() {
        let mut sim = FixedTimestepAccumulator::new();

        // 16ms frame (approx 60 Hz)
        let steps = sim.advance(0.016);
        assert_eq!(steps, 1);
        assert!(sim.accumulator > 0.0);

        // Another 16ms frame: accum was ~0.000375 + 0.016 = 0.016375 -> 1 step
        let steps2 = sim.advance(0.016);
        assert_eq!(steps2, 1);

        // 32ms frame (30 Hz): should yield 2 steps
        let steps3 = sim.advance(0.032);
        assert_eq!(steps3, 2);
    }

    #[test]
    fn test_spiral_of_death_clamp() {
        let mut sim = FixedTimestepAccumulator::new();
        // Massive freeze of 10 seconds
        let steps = sim.advance(10.0);
        assert_eq!(steps, DEFAULT_MAX_SUBSTEPS);
        assert!(sim.accumulator < FIXED_DELTA_SECONDS);
    }

    #[test]
    fn test_alpha_interpolation() {
        let mut sim = FixedTimestepAccumulator::new();
        sim.advance(0.015625 * 0.5);
        let alpha = sim.interpolation_alpha();
        assert!((alpha - 0.5).abs() < 1e-4);
    }
}
