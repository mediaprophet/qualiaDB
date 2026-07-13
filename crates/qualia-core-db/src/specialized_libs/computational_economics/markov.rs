//! Discrete-time Markov chain core.
//!
//! Implements transition-matrix validation, stationary-distribution power
//! iteration, deterministic seeded simulation, and mean first-passage-time
//! solves. Part of the QualiaDB computational economics library (plan §5.3 /
//! P3).
//!
//! # Allocation class: `HotZeroHeap`
//!
//! Every public kernel operates on caller-owned slices and fixed-capacity
//! stack arrays (`[0.0f64; MAX_STATES]`). No `Vec`, `String`, or `Box` is
//! constructed on the hot path. The only heap traffic is whatever the caller
//! chose to allocate for the input/output buffers.
//!
//! # Assumptions
//!
//! `stationary_distribution_into` assumes the chain is **ergodic** (aperiodic
//! and irreducible). Power iteration converges to the unique stationary
//! distribution only under that assumption; otherwise the kernel returns
//! `MarkovError::NonConverged` once the iteration budget is exhausted.
//!
//! `mean_first_passage_time_into` likewise assumes the target state is
//! reachable from every other state (irreducibility); unreachable states
//! produce divergent hitting times and surface as `NonConverged`.

use super::error::{EconConvergence, EconStatus};

/// Maximum number of states supported by the stack-array kernels.
pub const MAX_STATES: usize = 32;

/// Tolerance used when checking that a transition-matrix row sums to 1.0.
const ROW_SUM_TOLERANCE: f64 = 1e-9;

/// Markov-chain kernel error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkovError {
    /// Bad dimensions, zero states, or `n > MAX_STATES`.
    InvalidInput,
    /// A transition matrix row does not sum to 1.0, or contains a negative /
    /// non-finite entry.
    InvalidTransitionMatrix,
    /// A state index was out of range for the supplied matrix.
    InvalidState,
    /// A caller-owned output buffer was too small for the request.
    BufferTooSmall,
    /// A non-finite value (NaN / infinity) appeared during iteration.
    NonFinite,
    /// The iterative solver did not converge within the iteration budget.
    NonConverged,
}

impl MarkovError {
    /// Map to the ABI-stable `EconStatus` used by the shared error vocabulary.
    pub fn to_status(self) -> EconStatus {
        match self {
            MarkovError::InvalidInput => EconStatus::InvalidInput,
            MarkovError::InvalidTransitionMatrix => EconStatus::InvalidInput,
            MarkovError::InvalidState => EconStatus::InvalidInput,
            MarkovError::BufferTooSmall => EconStatus::BufferTooSmall,
            MarkovError::NonFinite => EconStatus::NonFinite,
            MarkovError::NonConverged => EconStatus::MaxIterations,
        }
    }
}

impl From<MarkovError> for EconStatus {
    #[inline]
    fn from(err: MarkovError) -> Self {
        err.to_status()
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG — local SplitMix64 (reimplemented to avoid cross-module
// coupling with `domains::financial::economics::stochastic`).
// ---------------------------------------------------------------------------

/// SplitMix64 bit-mixing PRNG. Deterministic for a given seed; used by the
/// seeded simulation kernel so that identical seeds reproduce identical paths.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform draw on the open interval (0, 1) with 53 random mantissa bits.
    #[inline]
    fn unit_open(&mut self) -> f64 {
        let bits = self.next_u64() >> 11;
        ((bits as f64) + 0.5) * (1.0 / ((1u64 << 53) as f64))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate the dimension and buffer length for an `n`-state matrix.
fn check_dim(n: usize) -> Result<(), MarkovError> {
    if n == 0 || n > MAX_STATES {
        return Err(MarkovError::InvalidInput);
    }
    Ok(())
}

/// Row-major index into a flat `n x n` transition matrix.
#[inline]
fn idx(row: usize, col: usize, n: usize) -> usize {
    row * n + col
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate that `p` is a row-stochastic `n x n` transition matrix.
///
/// Each row must sum to `1.0` within `ROW_SUM_TOLERANCE`, and every entry must
/// be finite and non-negative. `p` must hold at least `n * n` elements.
pub fn validate_transition_matrix(p: &[f64], n: usize) -> Result<(), MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    for row in 0..n {
        let mut sum = 0.0f64;
        for col in 0..n {
            let entry = p[idx(row, col, n)];
            if !entry.is_finite() {
                return Err(MarkovError::InvalidTransitionMatrix);
            }
            if entry < 0.0 {
                return Err(MarkovError::InvalidTransitionMatrix);
            }
            sum += entry;
        }
        if (sum - 1.0).abs() > ROW_SUM_TOLERANCE {
            return Err(MarkovError::InvalidTransitionMatrix);
        }
    }
    Ok(())
}

/// Look up `P[from][to]` with bounds checking.
pub fn transition_probability(p: &[f64], n: usize, from: usize, to: usize) -> Result<f64, MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    if from >= n || to >= n {
        return Err(MarkovError::InvalidState);
    }
    Ok(p[idx(from, to, n)])
}

/// Expected holding time for `state`: `1 / (1 - P[state][state])`.
///
/// Returns `MarkovError::InvalidState` for an out-of-range state and
/// `MarkovError::NonFinite` when the self-loop probability is `1.0` (the
/// state is absorbing, so the holding time is infinite).
pub fn expected_holding_time(p: &[f64], n: usize, state: usize) -> Result<f64, MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    if state >= n {
        return Err(MarkovError::InvalidState);
    }
    let self_loop = p[idx(state, state, n)];
    if !self_loop.is_finite() {
        return Err(MarkovError::NonFinite);
    }
    let denom = 1.0 - self_loop;
    if denom <= 0.0 {
        return Err(MarkovError::NonFinite);
    }
    Ok(1.0 / denom)
}

/// Compute the stationary distribution of an ergodic Markov chain via power
/// iteration: `pi_{t+1} = pi_t * P`.
///
/// Writes the stationary distribution into `out[..n]`. The initial guess is
/// the uniform distribution. Convergence is declared when the infinity-norm
/// of `pi_{t+1} - pi_t` falls below `tolerance`. Returns an `EconConvergence`
/// report; `status` is `Converged` on success and `MaxIterations` (mapped from
/// `MarkovError::NonConverged`) when the budget is exhausted.
///
/// Assumes the chain is ergodic (aperiodic + irreducible). Non-ergodic chains
/// may fail to converge, in which case `NonConverged` is returned.
pub fn stationary_distribution_into(
    p: &[f64],
    n: usize,
    max_iterations: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    if out.len() < n {
        return Err(MarkovError::BufferTooSmall);
    }
    if max_iterations == 0 || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(MarkovError::InvalidInput);
    }
    validate_transition_matrix(p, n)?;

    // Stack scratch arrays — HotZeroHeap.
    let mut current = [0.0f64; MAX_STATES];
    let mut next = [0.0f64; MAX_STATES];

    let uniform = 1.0 / n as f64;
    for i in 0..n {
        current[i] = uniform;
    }

    let mut iter: u32 = 0;
    let mut residual = f64::INFINITY;
    let mut converged = false;

    while iter < max_iterations {
        // next[j] = sum_i current[i] * P[i][j]
        for j in 0..n {
            next[j] = 0.0;
        }
        for i in 0..n {
            let ci = current[i];
            if ci == 0.0 {
                continue;
            }
            let base = i * n;
            for j in 0..n {
                next[j] += ci * p[base + j];
            }
        }

        // Residual: infinity norm of (next - current).
        let mut diff = 0.0f64;
        for j in 0..n {
            if !next[j].is_finite() {
                return Err(MarkovError::NonFinite);
            }
            let d = (next[j] - current[j]).abs();
            if d > diff {
                diff = d;
            }
        }
        residual = diff;

        // Swap current and next.
        for j in 0..n {
            current[j] = next[j];
        }

        iter += 1;
        if residual <= tolerance {
            converged = true;
            break;
        }
    }

    if !converged {
        // Still copy the best estimate into the caller buffer.
        for j in 0..n {
            out[j] = current[j];
        }
        return Err(MarkovError::NonConverged);
    }

    for j in 0..n {
        out[j] = current[j];
    }
    Ok(EconConvergence::converged(iter, residual))
}

/// Deterministic seeded simulation of a Markov chain.
///
/// Starting from `initial_state`, draws `steps` successive states using a
/// local SplitMix64 RNG seeded with `seed` and inverse-CDF sampling per step.
/// Writes the state indices (including the initial state at index 0) into
/// `out[..=steps]` — i.e. `out` must hold at least `steps + 1` elements.
/// Returns the number of indices written (`steps + 1`).
pub fn simulate_chain_into(
    p: &[f64],
    n: usize,
    initial_state: usize,
    steps: usize,
    seed: u64,
    out: &mut [usize],
) -> Result<usize, MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    if initial_state >= n {
        return Err(MarkovError::InvalidState);
    }
    if out.len() < steps + 1 {
        return Err(MarkovError::BufferTooSmall);
    }
    validate_transition_matrix(p, n)?;

    let mut rng = SplitMix64::new(seed);
    let mut state = initial_state;
    out[0] = state;

    for step in 0..steps {
        let u = rng.unit_open();
        let base = state * n;
        let mut acc = 0.0f64;
        let mut next_state = n - 1; // fallback for floating-point tail
        for col in 0..n {
            acc += p[base + col];
            if u < acc {
                next_state = col;
                break;
            }
        }
        state = next_state;
        out[step + 1] = state;
    }
    Ok(steps + 1)
}

/// Iterative solve for the mean first-passage time to `target`.
///
/// For an ergodic chain, `m_i` is the expected number of steps to first reach
/// `target` starting from state `i`. `m_target = 0`. For `i != target`:
///
/// ```text
/// m_i = 1 + sum_{j != target} P[i][j] * m_j
/// ```
///
/// Solved by fixed-point iteration. Writes results into `out[..n]` with
/// `out[target] = 0`. Returns an `EconConvergence` report.
pub fn mean_first_passage_time_into(
    p: &[f64],
    n: usize,
    target: usize,
    max_iterations: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, MarkovError> {
    check_dim(n)?;
    if p.len() < n * n {
        return Err(MarkovError::InvalidInput);
    }
    if target >= n {
        return Err(MarkovError::InvalidState);
    }
    if out.len() < n {
        return Err(MarkovError::BufferTooSmall);
    }
    if max_iterations == 0 || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(MarkovError::InvalidInput);
    }
    validate_transition_matrix(p, n)?;

    let mut current = [0.0f64; MAX_STATES];
    let mut next = [0.0f64; MAX_STATES];

    // Initial guess: zero everywhere (target stays zero).
    for i in 0..n {
        current[i] = 0.0;
    }

    let mut iter: u32 = 0;
    let mut residual = f64::INFINITY;
    let mut converged = false;

    while iter < max_iterations {
        let mut diff = 0.0f64;
        for i in 0..n {
            if i == target {
                next[i] = 0.0;
                continue;
            }
            let base = i * n;
            let mut s = 1.0f64;
            for j in 0..n {
                if j == target {
                    continue;
                }
                s += p[base + j] * current[j];
            }
            if !s.is_finite() {
                return Err(MarkovError::NonFinite);
            }
            next[i] = s;
            let d = (s - current[i]).abs();
            if d > diff {
                diff = d;
            }
        }
        residual = diff;

        for i in 0..n {
            current[i] = next[i];
        }

        iter += 1;
        if residual <= tolerance {
            converged = true;
            break;
        }
    }

    for i in 0..n {
        out[i] = current[i];
    }

    if !converged {
        return Err(MarkovError::NonConverged);
    }
    Ok(EconConvergence::converged(iter, residual))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 2-state symmetric chain: stationary distribution is [0.5, 0.5].
    #[test]
    fn symmetric_two_state_stationary() {
        // [[0.5, 0.5], [0.5, 0.5]]
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0.0f64; MAX_STATES];
        let conv = stationary_distribution_into(&p, 2, 10_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!((out[0] - 0.5).abs() < 1e-9, "out[0]={}", out[0]);
        assert!((out[1] - 0.5).abs() < 1e-9, "out[1]={}", out[1]);
    }

    /// 2-state asymmetric chain [[0.9,0.1],[0.5,0.5]] -> stationary [5/6, 1/6].
    #[test]
    fn asymmetric_two_state_stationary() {
        let p = [0.9, 0.1, 0.5, 0.5];
        let mut out = [0.0f64; MAX_STATES];
        let conv = stationary_distribution_into(&p, 2, 100_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!((out[0] - 5.0 / 6.0).abs() < 1e-6, "out[0]={}", out[0]);
        assert!((out[1] - 1.0 / 6.0).abs() < 1e-6, "out[1]={}", out[1]);
    }

    /// Validation rejects a row that does not sum to 1.
    #[test]
    fn validation_rejects_row_not_summing_to_one() {
        let p = [0.9, 0.2, 0.5, 0.5]; // row 0 sums to 1.1
        let err = validate_transition_matrix(&p, 2).unwrap_err();
        assert_eq!(err, MarkovError::InvalidTransitionMatrix);
    }

    /// Validation rejects a negative entry.
    #[test]
    fn validation_rejects_negative_entry() {
        let p = [1.2, -0.2, 0.5, 0.5];
        let err = validate_transition_matrix(&p, 2).unwrap_err();
        assert_eq!(err, MarkovError::InvalidTransitionMatrix);
    }

    /// Validation rejects a NaN entry.
    #[test]
    fn validation_rejects_nan_entry() {
        let p = [f64::NAN, 1.0, 0.5, 0.5];
        let err = validate_transition_matrix(&p, 2).unwrap_err();
        assert_eq!(err, MarkovError::InvalidTransitionMatrix);
    }

    /// Validation rejects an infinity entry.
    #[test]
    fn validation_rejects_infinity_entry() {
        let p = [f64::INFINITY, 0.0, 0.5, 0.5];
        let err = validate_transition_matrix(&p, 2).unwrap_err();
        assert_eq!(err, MarkovError::InvalidTransitionMatrix);
    }

    /// Validation accepts a well-formed matrix.
    #[test]
    fn validation_accepts_valid_matrix() {
        let p = [0.9, 0.1, 0.5, 0.5];
        assert!(validate_transition_matrix(&p, 2).is_ok());
    }

    /// Validation rejects zero states and over-capacity states.
    #[test]
    fn validation_rejects_bad_dimensions() {
        let p = [0.0; 4];
        assert_eq!(
            validate_transition_matrix(&p, 0).unwrap_err(),
            MarkovError::InvalidInput
        );
        assert_eq!(
            validate_transition_matrix(&p, MAX_STATES + 1).unwrap_err(),
            MarkovError::InvalidInput
        );
    }

    /// Validation rejects a buffer too small for the matrix.
    #[test]
    fn validation_rejects_undersized_slice() {
        let p = [0.9, 0.1]; // only 2 elements for a 2x2
        assert_eq!(
            validate_transition_matrix(&p, 2).unwrap_err(),
            MarkovError::InvalidInput
        );
    }

    /// Same seed reproduces the same path.
    #[test]
    fn simulation_is_reproducible() {
        let p = [0.9, 0.1, 0.5, 0.5];
        let steps = 200;
        let mut a = [0usize; 256];
        let mut b = [0usize; 256];
        let na = simulate_chain_into(&p, 2, 0, steps, 42, &mut a).unwrap();
        let nb = simulate_chain_into(&p, 2, 0, steps, 42, &mut b).unwrap();
        assert_eq!(na, nb);
        assert_eq!(na, steps + 1);
        for k in 0..na {
            assert_eq!(a[k], b[k], "path divergence at {}", k);
        }
    }

    /// Different seeds produce (almost certainly) different paths.
    #[test]
    fn different_seeds_diverge() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let steps = 100;
        let mut a = [0usize; 256];
        let mut b = [0usize; 256];
        simulate_chain_into(&p, 2, 0, steps, 1, &mut a).unwrap();
        simulate_chain_into(&p, 2, 0, steps, 2, &mut b).unwrap();
        // At least one step should differ.
        let mut any_diff = false;
        for k in 0..=steps {
            if a[k] != b[k] {
                any_diff = true;
                break;
            }
        }
        assert!(any_diff, "two different seeds produced identical paths");
    }

    /// Simulation respects transition probabilities statistically.
    #[test]
    fn simulation_respects_transition_probabilities() {
        // [[0.9, 0.1], [0.5, 0.5]]
        let p = [0.9, 0.1, 0.5, 0.5];
        let steps = 200_000;
        let mut path = vec![0usize; steps + 1];
        simulate_chain_into(&p, 2, 0, steps, 12345, &mut path).unwrap();

        // Count transitions out of state 0.
        let mut from0_total = 0usize;
        let mut from0_to1 = 0usize;
        let mut from1_total = 0usize;
        let mut from1_to0 = 0usize;
        for k in 0..steps {
            let s = path[k];
            let ns = path[k + 1];
            if s == 0 {
                from0_total += 1;
                if ns == 1 {
                    from0_to1 += 1;
                }
            } else {
                from1_total += 1;
                if ns == 0 {
                    from1_to0 += 1;
                }
            }
        }
        let p01 = from0_to1 as f64 / from0_total as f64;
        let p10 = from1_to0 as f64 / from1_total as f64;
        assert!((p01 - 0.1).abs() < 0.01, "empirical p01={}", p01);
        assert!((p10 - 0.5).abs() < 0.01, "empirical p10={}", p10);
    }

    /// Holding time for a state with p=0.9 self-loop is 10.
    #[test]
    fn holding_time_self_loop() {
        let p = [0.9, 0.1, 0.5, 0.5];
        let h = expected_holding_time(&p, 2, 0).unwrap();
        assert!((h - 10.0).abs() < 1e-9, "holding time={}", h);
    }

    /// Holding time for an absorbing state (self-loop = 1.0) errors.
    #[test]
    fn holding_time_absorbing_errors() {
        let p = [1.0, 0.0, 0.5, 0.5];
        let err = expected_holding_time(&p, 2, 0).unwrap_err();
        assert_eq!(err, MarkovError::NonFinite);
    }

    /// Mean first-passage time on a small chain.
    ///
    /// Chain: 0 -> 1 -> 2 (deterministic), plus self-loops to make it ergodic.
    /// Use [[0.5, 0.5, 0], [0, 0.5, 0.5], [0, 0, 1.0]] is not ergodic (2
    /// absorbing). Instead use a 3-state chain where target=2.
    #[test]
    fn mean_first_passage_time_small_chain() {
        // [[0.5, 0.5, 0.0],
        //  [0.0, 0.5, 0.5],
        //  [0.1, 0.1, 0.8]]
        let p = [0.5, 0.5, 0.0, 0.0, 0.5, 0.5, 0.1, 0.1, 0.8];
        let mut out = [0.0f64; MAX_STATES];
        let conv = mean_first_passage_time_into(&p, 3, 2, 100_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(out[2].abs() < 1e-9, "target mfp must be 0, got {}", out[2]);
        // m_0 and m_1 must be positive and finite.
        assert!(out[0] > 0.0 && out[0].is_finite(), "m_0={}", out[0]);
        assert!(out[1] > 0.0 && out[1].is_finite(), "m_1={}", out[1]);
        // m_1 < m_0 since state 1 is closer to target 2.
        assert!(out[1] < out[0], "m_1={} should be < m_0={}", out[1], out[0]);
    }

    /// Mean first-passage time on a 2-state chain with a closed-form check.
    ///
    /// [[0.9, 0.1], [0.5, 0.5]], target = 1.
    /// m_1 = 0. m_0 = 1 + 0.9 * m_0 => m_0 = 1 / 0.1 = 10.
    #[test]
    fn mean_first_passage_time_two_state_closed_form() {
        let p = [0.9, 0.1, 0.5, 0.5];
        let mut out = [0.0f64; MAX_STATES];
        let conv = mean_first_passage_time_into(&p, 2, 1, 100_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(out[1].abs() < 1e-9);
        assert!((out[0] - 10.0).abs() < 1e-6, "m_0={}", out[0]);
    }

    /// Buffer-too-small error for stationary distribution.
    #[test]
    fn stationary_buffer_too_small() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0.0f64; 1]; // need 2
        let err = stationary_distribution_into(&p, 2, 100, 1e-9, &mut out).unwrap_err();
        assert_eq!(err, MarkovError::BufferTooSmall);
    }

    /// Buffer-too-small error for simulation.
    #[test]
    fn simulation_buffer_too_small() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0usize; 5]; // need steps+1 = 11
        let err = simulate_chain_into(&p, 2, 0, 10, 1, &mut out).unwrap_err();
        assert_eq!(err, MarkovError::BufferTooSmall);
    }

    /// Buffer-too-small error for mean first-passage time.
    #[test]
    fn mfp_buffer_too_small() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0.0f64; 1];
        let err = mean_first_passage_time_into(&p, 2, 0, 100, 1e-9, &mut out).unwrap_err();
        assert_eq!(err, MarkovError::BufferTooSmall);
    }

    /// Invalid-state error for transition_probability.
    #[test]
    fn transition_probability_invalid_state() {
        let p = [0.5, 0.5, 0.5, 0.5];
        assert_eq!(
            transition_probability(&p, 2, 2, 0).unwrap_err(),
            MarkovError::InvalidState
        );
        assert_eq!(
            transition_probability(&p, 2, 0, 2).unwrap_err(),
            MarkovError::InvalidState
        );
    }

    /// Invalid-state error for expected_holding_time.
    #[test]
    fn holding_time_invalid_state() {
        let p = [0.5, 0.5, 0.5, 0.5];
        assert_eq!(
            expected_holding_time(&p, 2, 5).unwrap_err(),
            MarkovError::InvalidState
        );
    }

    /// Invalid-state error for simulation initial state.
    #[test]
    fn simulation_invalid_initial_state() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0usize; 16];
        assert_eq!(
            simulate_chain_into(&p, 2, 5, 10, 1, &mut out).unwrap_err(),
            MarkovError::InvalidState
        );
    }

    /// Invalid-state error for mean first-passage time target.
    #[test]
    fn mfp_invalid_target() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0.0f64; MAX_STATES];
        assert_eq!(
            mean_first_passage_time_into(&p, 2, 5, 100, 1e-9, &mut out).unwrap_err(),
            MarkovError::InvalidState
        );
    }

    /// transition_probability returns the correct entry.
    #[test]
    fn transition_probability_lookup() {
        let p = [0.9, 0.1, 0.5, 0.5];
        assert!((transition_probability(&p, 2, 0, 1).unwrap() - 0.1).abs() < 1e-12);
        assert!((transition_probability(&p, 2, 1, 0).unwrap() - 0.5).abs() < 1e-12);
    }

    /// Stationary distribution of a 3-state chain sums to 1.
    #[test]
    fn stationary_distribution_normalizes() {
        // Ergodic 3-state chain.
        let p = [0.2, 0.6, 0.2, 0.3, 0.4, 0.3, 0.1, 0.2, 0.7];
        let mut out = [0.0f64; MAX_STATES];
        let conv = stationary_distribution_into(&p, 3, 100_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        let sum = out[0] + out[1] + out[2];
        assert!((sum - 1.0).abs() < 1e-9, "sum={}", sum);
        // Verify pi * P = pi.
        for j in 0..3 {
            let mut pip = 0.0;
            for i in 0..3 {
                pip += out[i] * p[i * 3 + j];
            }
            assert!((pip - out[j]).abs() < 1e-6, "pi*P[{}] != pi[{}]", j, j);
        }
    }

    /// Non-convergence surfaces when the iteration budget is too small.
    #[test]
    fn stationary_non_converged_on_tiny_budget() {
        // Near-identity but asymmetric so uniform start is not stationary.
        let p = [0.999, 0.001, 0.1, 0.9];
        let mut out = [0.0f64; MAX_STATES];
        let err = stationary_distribution_into(&p, 2, 1, 1e-15, &mut out).unwrap_err();
        assert_eq!(err, MarkovError::NonConverged);
    }

    /// Mean first-passage time non-convergence on a tiny budget.
    #[test]
    fn mfp_non_converged_on_tiny_budget() {
        let p = [0.9, 0.1, 0.5, 0.5];
        let mut out = [0.0f64; MAX_STATES];
        let err = mean_first_passage_time_into(&p, 2, 1, 1, 1e-15, &mut out).unwrap_err();
        assert_eq!(err, MarkovError::NonConverged);
    }

    /// Zero-step simulation writes only the initial state.
    #[test]
    fn simulation_zero_steps_writes_initial() {
        let p = [0.5, 0.5, 0.5, 0.5];
        let mut out = [0usize; 4];
        let n = simulate_chain_into(&p, 2, 1, 0, 7, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 1);
    }

    /// Error-to-status mapping covers every variant.
    #[test]
    fn error_to_status_mapping() {
        assert_eq!(MarkovError::InvalidInput.to_status(), EconStatus::InvalidInput);
        assert_eq!(
            MarkovError::InvalidTransitionMatrix.to_status(),
            EconStatus::InvalidInput
        );
        assert_eq!(MarkovError::InvalidState.to_status(), EconStatus::InvalidInput);
        assert_eq!(MarkovError::BufferTooSmall.to_status(), EconStatus::BufferTooSmall);
        assert_eq!(MarkovError::NonFinite.to_status(), EconStatus::NonFinite);
        assert_eq!(MarkovError::NonConverged.to_status(), EconStatus::MaxIterations);
    }

    /// `MAX_STATES` is 32.
    #[test]
    fn max_states_is_32() {
        assert_eq!(MAX_STATES, 32);
    }

    /// A 4-state ring chain has a uniform stationary distribution.
    #[test]
    fn ring_chain_uniform_stationary() {
        // 0->1->2->3->0, deterministic ring (periodic). Add small self-loops
        // to make it aperiodic / ergodic.
        let p = [
            0.1, 0.9, 0.0, 0.0, //
            0.0, 0.1, 0.9, 0.0, //
            0.0, 0.0, 0.1, 0.9, //
            0.9, 0.0, 0.0, 0.1, //
        ];
        let mut out = [0.0f64; MAX_STATES];
        let conv = stationary_distribution_into(&p, 4, 100_000, 1e-12, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        for i in 0..4 {
            assert!((out[i] - 0.25).abs() < 1e-6, "out[{}]={}", i, out[i]);
        }
    }
}
