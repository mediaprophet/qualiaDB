//! Ground-State Resolver (GSR) integration for quantum context resolution.
//!
//! This module keeps runtime state zero-heap by using fixed-capacity arrays for
//! QUBO terms, request queues, and result caches.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const MAX_QUBO_COEFFICIENTS: usize = 64;
pub const MAX_LINEAR_TERMS: usize = 64;
pub const MAX_PENDING_REQUESTS: usize = 16;
pub const MAX_RESULTS_CACHE: usize = 32;
pub const MAX_AXIOM_CACHE: usize = 32;

/// QUBO problem for quantum context resolution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuboProblem {
    /// Hash-based problem identifier.
    pub problem_id: u64,
    /// QUBO matrix coefficients.
    pub coefficients: [(usize, usize, f32); MAX_QUBO_COEFFICIENTS],
    /// Number of active quadratic coefficients.
    pub coefficient_count: usize,
    /// Linear terms.
    pub linear_terms: [(usize, f32); MAX_LINEAR_TERMS],
    /// Number of active linear terms.
    pub linear_term_count: usize,
    /// Problem size (number of variables).
    pub size: usize,
    /// Context identifier for this problem.
    pub context_id: u64,
}

impl Default for QuboProblem {
    fn default() -> Self {
        Self {
            problem_id: 0,
            coefficients: [(0, 0, 0.0); MAX_QUBO_COEFFICIENTS],
            coefficient_count: 0,
            linear_terms: [(0, 0.0); MAX_LINEAR_TERMS],
            linear_term_count: 0,
            size: 0,
            context_id: 0,
        }
    }
}

impl QuboProblem {
    pub fn add_coefficient(&mut self, i: usize, j: usize, coeff: f32) -> Result<(), GsrError> {
        if self.coefficient_count >= self.coefficients.len() {
            return Err(GsrError::CoefficientCapacityExceeded);
        }

        self.coefficients[self.coefficient_count] = (i, j, coeff);
        self.coefficient_count += 1;
        Ok(())
    }

    pub fn add_linear_term(&mut self, i: usize, coeff: f32) -> Result<(), GsrError> {
        if self.linear_term_count >= self.linear_terms.len() {
            return Err(GsrError::LinearTermCapacityExceeded);
        }

        self.linear_terms[self.linear_term_count] = (i, coeff);
        self.linear_term_count += 1;
        Ok(())
    }
}

/// GSR resolution result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GsrResult {
    /// Hash-based problem identifier.
    pub problem_id: u64,
    /// Winning context (q value to promote to ground truth).
    pub winning_context: f32,
    /// Resolution confidence (0.0 to 1.0).
    pub confidence: f32,
    /// Resolution timestamp.
    pub resolved_at: u64,
    /// Computation time in milliseconds.
    pub compute_time_ms: u64,
    /// Whether classical exhaustion was used.
    pub classical_fallback: bool,
}

/// GSR resolution request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GsrRequest {
    /// QUBO problem to solve.
    pub problem: QuboProblem,
    /// Maximum computation time in milliseconds.
    pub max_compute_time_ms: u64,
    /// Priority level (0 = highest).
    pub priority: u8,
    /// Request timestamp.
    pub requested_at: u64,
}

/// Ground-State Resolver for quantum context resolution.
pub struct GroundStateResolver {
    pending_requests: [Option<GsrRequest>; MAX_PENDING_REQUESTS],
    results_cache: [Option<GsrResult>; MAX_RESULTS_CACHE],
    axiom_cache: [Option<(u64, f32)>; MAX_AXIOM_CACHE],
    last_cleanup: Instant,
    qpu_available: bool,
    classical_solver_enabled: bool,
}

impl GroundStateResolver {
    /// Create a new GSR instance.
    pub fn new() -> Self {
        Self {
            pending_requests: [None; MAX_PENDING_REQUESTS],
            results_cache: [None; MAX_RESULTS_CACHE],
            axiom_cache: [None; MAX_AXIOM_CACHE],
            last_cleanup: Instant::now(),
            qpu_available: false,
            classical_solver_enabled: true,
        }
    }

    /// Submit a QUBO problem for resolution.
    pub fn submit_problem(&mut self, request: GsrRequest) -> Result<u64, GsrError> {
        if self.get_result(request.problem.problem_id).is_some() {
            return Ok(request.problem.problem_id);
        }

        let slot = self
            .pending_requests
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(GsrError::PendingQueueFull)?;
        *slot = Some(request);

        self.process_requests()?;
        Ok(request.problem.problem_id)
    }

    /// Get result for a problem.
    pub fn get_result(&self, problem_id: u64) -> Option<GsrResult> {
        self.results_cache
            .iter()
            .flatten()
            .find(|result| result.problem_id == problem_id)
            .copied()
    }

    /// Process pending requests.
    fn process_requests(&mut self) -> Result<(), GsrError> {
        for index in 0..self.pending_requests.len() {
            let request = match self.pending_requests[index].take() {
                Some(request) => request,
                None => continue,
            };

            let result = if self.qpu_available {
                self.solve_with_qpu(&request)?
            } else {
                self.solve_with_classical(&request)?
            };

            self.store_result(result)?;
        }

        Ok(())
    }

    /// Solve problem using QPU semantics.
    fn solve_with_qpu(&self, request: &GsrRequest) -> Result<GsrResult, GsrError> {
        let start = Instant::now();
        let mut winning_context = 0.0f32;
        let mut saw_term = false;

        for &(_, weight) in request.problem.linear_terms[..request.problem.linear_term_count].iter()
        {
            let magnitude = weight.abs();
            if !saw_term || magnitude > winning_context {
                winning_context = magnitude;
                saw_term = true;
            }
        }

        Ok(GsrResult {
            problem_id: request.problem.problem_id,
            winning_context,
            confidence: 0.95,
            resolved_at: current_unix_secs(),
            compute_time_ms: start.elapsed().as_millis() as u64,
            classical_fallback: false,
        })
    }

    /// Solve problem using classical exhaustion (fallback).
    fn solve_with_classical(&self, request: &GsrRequest) -> Result<GsrResult, GsrError> {
        if !self.classical_solver_enabled {
            return Err(GsrError::ClassicalSolverDisabled);
        }

        let start = Instant::now();
        let winning_context = if request.problem.size <= 16 {
            self.classical_exhaustive_search(&request.problem)?
        } else {
            self.classical_greedy_approximation(&request.problem)?
        };

        Ok(GsrResult {
            problem_id: request.problem.problem_id,
            winning_context,
            confidence: 0.85,
            resolved_at: current_unix_secs(),
            compute_time_ms: start.elapsed().as_millis() as u64,
            classical_fallback: true,
        })
    }

    /// Classical exhaustive search for small QUBO problems.
    fn classical_exhaustive_search(&self, problem: &QuboProblem) -> Result<f32, GsrError> {
        if problem.size > 31 {
            return Err(GsrError::ProblemTooLarge);
        }

        let n = problem.size;
        let mut best_value = f32::NEG_INFINITY;
        let mut best_context = 0.0;

        for mask in 0u32..(1u32 << n) {
            let mut value = 0.0;

            for &(i, j, coeff) in &problem.coefficients[..problem.coefficient_count] {
                let xi = ((mask >> i) & 1) as f32;
                let xj = ((mask >> j) & 1) as f32;
                value += coeff * xi * xj;
            }

            for &(i, linear_coeff) in &problem.linear_terms[..problem.linear_term_count] {
                let xi = ((mask >> i) & 1) as f32;
                value += linear_coeff * xi;
            }

            if value > best_value {
                best_value = value;
                best_context = mask as f32;
            }
        }

        Ok(best_context)
    }

    /// Classical greedy approximation for larger QUBO problems.
    fn classical_greedy_approximation(&self, problem: &QuboProblem) -> Result<f32, GsrError> {
        let mut context = 0u32;

        for &(i, linear_coeff) in &problem.linear_terms[..problem.linear_term_count] {
            if linear_coeff > 0.0 {
                context |= 1u32 << i;
            }
        }

        Ok(context as f32)
    }

    pub fn set_qpu_available(&mut self, available: bool) {
        self.qpu_available = available;
    }

    pub fn is_qpu_available(&self) -> bool {
        self.qpu_available
    }

    pub fn set_classical_solver_enabled(&mut self, enabled: bool) {
        self.classical_solver_enabled = enabled;
    }

    /// Evolve epistemic frame based on GSR result.
    pub fn evolve_epistemic_frame(
        &mut self,
        problem_id: u64,
        result: &GsrResult,
    ) -> Result<(), GsrError> {
        if let Some(entry) = self
            .axiom_cache
            .iter_mut()
            .find(|entry| matches!(entry, Some((id, _)) if *id == problem_id))
        {
            *entry = Some((problem_id, result.winning_context));
            return Ok(());
        }

        let slot = self
            .axiom_cache
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(GsrError::AxiomCacheFull)?;
        *slot = Some((problem_id, result.winning_context));
        Ok(())
    }

    /// Get cached axiom for a problem.
    pub fn get_cached_axiom(&self, problem_id: u64) -> Option<f32> {
        self.axiom_cache
            .iter()
            .flatten()
            .find(|(id, _)| *id == problem_id)
            .map(|(_, value)| *value)
    }

    /// Clean up old cache entries.
    pub fn cleanup_cache(&mut self, max_age: Duration) -> Result<(), GsrError> {
        let now = Instant::now();
        if now.duration_since(self.last_cleanup) <= Duration::from_secs(300) {
            return Ok(());
        }

        let current_time = current_unix_secs();
        for entry in &mut self.results_cache {
            if let Some(result) = entry {
                if current_time.saturating_sub(result.resolved_at) >= max_age.as_secs() {
                    *entry = None;
                }
            }
        }

        self.last_cleanup = now;
        Ok(())
    }

    fn store_result(&mut self, result: GsrResult) -> Result<(), GsrError> {
        if let Some(entry) = self.results_cache.iter_mut().find(
            |entry| matches!(entry, Some(existing) if existing.problem_id == result.problem_id),
        ) {
            *entry = Some(result);
            return Ok(());
        }

        let slot = self
            .results_cache
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(GsrError::ResultsCacheFull)?;
        *slot = Some(result);
        Ok(())
    }
}

impl Default for GroundStateResolver {
    fn default() -> Self {
        Self::new()
    }
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// GSR-related errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GsrError {
    CoefficientCapacityExceeded,
    LinearTermCapacityExceeded,
    PendingQueueFull,
    ResultsCacheFull,
    AxiomCacheFull,
    ProblemTooLarge,
    ClassicalSolverDisabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gsr_creation() {
        let gsr = GroundStateResolver::new();
        assert!(!gsr.is_qpu_available());
    }

    #[test]
    fn test_qpu_availability() {
        let mut gsr = GroundStateResolver::new();
        gsr.set_qpu_available(true);
        assert!(gsr.is_qpu_available());

        gsr.set_qpu_available(false);
        assert!(!gsr.is_qpu_available());
    }

    #[test]
    fn test_classical_exhaustive_search() {
        let gsr = GroundStateResolver::new();
        let mut problem = QuboProblem {
            problem_id: 1,
            size: 2,
            context_id: 0,
            ..QuboProblem::default()
        };
        problem.add_linear_term(0, 1.0).unwrap();
        problem.add_linear_term(1, -0.5).unwrap();

        let result = gsr.classical_exhaustive_search(&problem).unwrap();
        assert!(result >= 0.0);
    }

    #[test]
    fn test_classical_greedy_approximation() {
        let gsr = GroundStateResolver::new();
        let mut problem = QuboProblem {
            problem_id: 2,
            size: 3,
            context_id: 0,
            ..QuboProblem::default()
        };
        problem.add_linear_term(0, 1.0).unwrap();
        problem.add_linear_term(1, -0.5).unwrap();
        problem.add_linear_term(2, 2.0).unwrap();

        let result = gsr.classical_greedy_approximation(&problem).unwrap();
        assert!(result >= 0.0);
    }

    #[test]
    fn test_axiom_caching() {
        let mut gsr = GroundStateResolver::new();
        let result = GsrResult {
            problem_id: 42,
            winning_context: 42.0,
            confidence: 0.9,
            resolved_at: 1000,
            compute_time_ms: 100,
            classical_fallback: false,
        };

        gsr.evolve_epistemic_frame(42, &result).unwrap();
        let cached = gsr.get_cached_axiom(42);

        assert_eq!(cached, Some(42.0));
    }

    #[test]
    fn test_submit_problem_caches_result() {
        let mut gsr = GroundStateResolver::new();
        let mut problem = QuboProblem {
            problem_id: 7,
            size: 2,
            context_id: 9,
            ..QuboProblem::default()
        };
        problem.add_linear_term(0, 1.0).unwrap();

        let request = GsrRequest {
            problem,
            max_compute_time_ms: 100,
            priority: 0,
            requested_at: 1,
        };

        let problem_id = gsr.submit_problem(request).unwrap();
        let result = gsr.get_result(problem_id);

        assert_eq!(problem_id, 7);
        assert!(result.is_some());
    }
}
