//! Convergence Checking Utilities
//!
//! This module provides zero-heap utilities for checking convergence
//! in numerical solvers. All operations use fixed-size arrays and
//! caller-supplied buffers to respect the zero-heap mandate.

/// Default maximum iterations for convergence checking
pub const DEFAULT_MAX_ITERATIONS: usize = 1000;

/// Default tolerance for convergence checking
pub const DEFAULT_TOLERANCE: f64 = 1e-6;

/// Convergence criteria for iterative solvers
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConvergenceCriteria {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Absolute tolerance
    pub tolerance: f64,
    /// Relative tolerance
    pub relative_tolerance: f64,
    /// Minimum residual change
    pub min_residual_change: f64,
}

impl Default for ConvergenceCriteria {
    fn default() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
            tolerance: DEFAULT_TOLERANCE,
            relative_tolerance: 1e-8,
            min_residual_change: 1e-10,
        }
    }
}

impl ConvergenceCriteria {
    /// Creates new convergence criteria
    pub fn new(max_iterations: usize, tolerance: f64) -> Self {
        Self {
            max_iterations,
            tolerance,
            ..Default::default()
        }
    }

    /// Sets relative tolerance
    pub fn with_relative_tolerance(mut self, rel_tol: f64) -> Self {
        self.relative_tolerance = rel_tol;
        self
    }

    /// Sets minimum residual change
    pub fn with_min_residual_change(mut self, min_change: f64) -> Self {
        self.min_residual_change = min_change;
        self
    }
}

/// Convergence status
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceStatus {
    /// Converged successfully
    Converged,
    /// Did not converge within max iterations
    NotConverged,
    /// Diverged (residual increased)
    Diverged,
    /// Stalled (no progress)
    Stalled,
}

/// Convergence checker for iterative solvers
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConvergenceChecker {
    criteria: ConvergenceCriteria,
    iteration: usize,
    previous_residual: f64,
    residual_history: [f64; 8], // Track last 8 residuals for stagnation detection
    history_index: usize,
}

impl ConvergenceChecker {
    /// Creates a new convergence checker
    pub fn new(criteria: ConvergenceCriteria) -> Self {
        Self {
            criteria,
            iteration: 0,
            previous_residual: f64::INFINITY,
            residual_history: [0.0; 8],
            history_index: 0,
        }
    }

    /// Creates with default criteria
    pub fn with_defaults() -> Self {
        Self::new(ConvergenceCriteria::default())
    }

    /// Checks convergence based on current residual
    pub fn check(&mut self, residual: f64) -> ConvergenceStatus {
        self.iteration += 1;

        // Check max iterations
        if self.iteration >= self.criteria.max_iterations {
            return ConvergenceStatus::NotConverged;
        }

        // Check absolute tolerance
        if residual < self.criteria.tolerance {
            return ConvergenceStatus::Converged;
        }

        // Check relative tolerance
        if self.previous_residual != f64::INFINITY {
            let relative_change = (residual - self.previous_residual).abs() / self.previous_residual.abs();
            if relative_change < self.criteria.relative_tolerance {
                return ConvergenceStatus::Converged;
            }

            // Check for divergence
            if residual > self.previous_residual * 2.0 {
                return ConvergenceStatus::Diverged;
            }

            // Check for stagnation (no significant change)
            if (residual - self.previous_residual).abs() < self.criteria.min_residual_change {
                return ConvergenceStatus::Stalled;
            }
        }

        // Update history for stagnation detection
        self.residual_history[self.history_index] = residual;
        self.history_index = (self.history_index + 1) % 8;

        // Check for stagnation over history
        if self.iteration > 8 {
            let min_residual = self.residual_history.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_residual = self.residual_history.iter().cloned().fold(0.0, f64::max);
            if (max_residual - min_residual) < self.criteria.min_residual_change {
                return ConvergenceStatus::Stalled;
            }
        }

        self.previous_residual = residual;
        ConvergenceStatus::NotConverged
    }

    /// Returns current iteration count
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// Resets the checker
    pub fn reset(&mut self) {
        self.iteration = 0;
        self.previous_residual = f64::INFINITY;
        self.residual_history = [0.0; 8];
        self.history_index = 0;
    }

    /// Returns the convergence criteria
    pub fn criteria(&self) -> &ConvergenceCriteria {
        &self.criteria
    }
}

impl Default for ConvergenceChecker {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convergence_criteria_default() {
        let criteria = ConvergenceCriteria::default();
        assert_eq!(criteria.max_iterations, DEFAULT_MAX_ITERATIONS);
        assert_eq!(criteria.tolerance, DEFAULT_TOLERANCE);
    }

    #[test]
    fn test_convergence_checker_converged() {
        let mut checker = ConvergenceChecker::with_defaults();
        let status = checker.check(1e-7); // Below tolerance
        assert_eq!(status, ConvergenceStatus::Converged);
    }

    #[test]
    fn test_convergence_checker_not_converged() {
        let mut checker = ConvergenceChecker::with_defaults();
        let status = checker.check(1.0); // Above tolerance
        assert_eq!(status, ConvergenceStatus::NotConverged);
    }

    #[test]
    fn test_convergence_checker_diverged() {
        let mut checker = ConvergenceChecker::with_defaults();
        checker.check(1.0);
        let status = checker.check(3.0); // Increased significantly
        assert_eq!(status, ConvergenceStatus::Diverged);
    }

    #[test]
    fn test_convergence_checker_reset() {
        let mut checker = ConvergenceChecker::with_defaults();
        checker.check(1.0);
        checker.check(0.5);
        assert_eq!(checker.iteration(), 2);
        
        checker.reset();
        assert_eq!(checker.iteration(), 0);
    }
}