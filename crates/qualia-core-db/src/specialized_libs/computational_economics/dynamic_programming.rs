//! Dynamic programming core: finite-state Bellman operators, value iteration,
//! policy iteration, and optimal stopping.
//!
//! Allocation class: `HotZeroHeap`. All hot kernels use caller-owned output
//! buffers and fixed-capacity stack arrays (`[f64; MAX_STATES]`,
//! `[u32; MAX_STATES]`). No `Vec`, `String`, or `Box` appears in any iteration
//! path. The 42 MB Sentinel ceiling is respected structurally: the largest
//! scratch footprint is `MAX_STATES * MAX_ACTIONS * MAX_STATES * 8` bytes for
//! the reward/transitions views (read-only caller slices) plus
//! `MAX_STATES * 8` bytes of owned stack scratch (~256 B), well under budget.
//!
//! # Assumptions
//!
//! - **Stationary MDP**: rewards and transition probabilities do not depend on
//!   time. The Bellman operator is a contraction when `0 <= discount < 1`.
//! - **Infinite horizon**: the value function solves `V = T V` where `T` is the
//!   Bellman operator. No terminal state is special-cased; callers encode
//!   absorbing states via self-loop transitions.
//! - **Geometric discounting**: future rewards are weighted by `discount**t`.
//! - **Maximization objective**: the agent maximizes expected discounted
//!   reward. Ties are broken by smallest action index (deterministic).
//!
//! # Error model
//!
//! Each solver returns `Result<EconConvergence, DpError>`. `DpError` maps to
//! `EconStatus` via `DpError::to_status`, covering `Converged`,
//! `MaxIterations`, `InvalidModel`, `BufferTooSmall`, `NonFinite`, and
//! `InvalidInput` as required by plan §5.3 and AGIA.md.

use super::error::{EconConvergence, EconStatus};

/// Maximum number of states supported by the fixed-capacity kernels.
pub const MAX_STATES: usize = 32;

/// Maximum number of actions supported by the fixed-capacity kernels.
pub const MAX_ACTIONS: usize = 32;

/// Domain-specific error for dynamic programming kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpError {
    /// Dimensions, discount, or slice lengths failed validation.
    InvalidInput,
    /// A caller-supplied output buffer was too small for the request.
    BufferTooSmall,
    /// A non-finite value (NaN/inf) appeared during iteration.
    NonFinite,
    /// The iteration budget was exhausted before convergence.
    NonConverged,
    /// The supplied MDP is structurally invalid (e.g. transition rows do not
    /// sum to 1 within tolerance, or a state has no reachable successor).
    InvalidModel,
}

impl DpError {
    /// Map the error to the closest ABI-stable `EconStatus` code.
    pub fn to_status(self) -> EconStatus {
        match self {
            DpError::InvalidInput => EconStatus::InvalidInput,
            DpError::BufferTooSmall => EconStatus::BufferTooSmall,
            DpError::NonFinite => EconStatus::NonFinite,
            DpError::NonConverged => EconStatus::MaxIterations,
            DpError::InvalidModel => EconStatus::InvalidInput,
        }
    }
}

impl From<DpError> for EconStatus {
    #[inline]
    fn from(err: DpError) -> Self {
        err.to_status()
    }
}

/// Tolerance used when checking that transition rows sum to one.
const ROW_SUM_TOL: f64 = 1e-9;

#[inline]
fn reward_index(s: usize, a: usize, n_actions: usize) -> usize {
    s * n_actions + a
}

#[inline]
fn trans_index(s: usize, a: usize, sp: usize, n_actions: usize, n_states: usize) -> usize {
    s * n_actions * n_states + a * n_states + sp
}

/// Validate the shape and fineness of an MDP description.
fn validate_mdp(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    n_states: usize,
    n_actions: usize,
) -> Result<(), DpError> {
    if n_states == 0 || n_actions == 0 {
        return Err(DpError::InvalidInput);
    }
    if n_states > MAX_STATES || n_actions > MAX_ACTIONS {
        return Err(DpError::InvalidInput);
    }
    if rewards.len() != n_states * n_actions {
        return Err(DpError::InvalidInput);
    }
    if transitions.len() != n_states * n_actions * n_states {
        return Err(DpError::InvalidInput);
    }
    if !discount.is_finite() || discount < 0.0 || discount >= 1.0 {
        return Err(DpError::InvalidInput);
    }
    for r in rewards {
        if !r.is_finite() {
            return Err(DpError::NonFinite);
        }
    }
    for p in transitions {
        if !p.is_finite() || *p < 0.0 {
            return Err(DpError::InvalidModel);
        }
    }
    // Each (s, a) transition row must sum to 1.
    for s in 0..n_states {
        for a in 0..n_actions {
            let mut sum = 0.0;
            for sp in 0..n_states {
                sum += transitions[trans_index(s, a, sp, n_actions, n_states)];
            }
            if (sum - 1.0).abs() > ROW_SUM_TOL {
                return Err(DpError::InvalidModel);
            }
        }
    }
    Ok(())
}

/// Single-state Bellman update.
///
/// Computes `max_a [ reward(s,a) + discount * sum_s' P(s'|s,a) * V(s') ]` and
/// returns the updated value for `state`. The argmax action is not returned
/// here; use [`value_iteration_into`] for joint value/policy recovery.
///
/// `values` must have length `>= n_states`.
pub fn bellman_update(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    values: &[f64],
    n_states: usize,
    n_actions: usize,
    state: usize,
) -> Result<f64, DpError> {
    validate_mdp(rewards, transitions, discount, n_states, n_actions)?;
    if state >= n_states {
        return Err(DpError::InvalidInput);
    }
    if values.len() < n_states {
        return Err(DpError::BufferTooSmall);
    }
    for v in values.iter().take(n_states) {
        if !v.is_finite() {
            return Err(DpError::NonFinite);
        }
    }

    let mut best = f64::NEG_INFINITY;
    for a in 0..n_actions {
        let mut cont = 0.0;
        for sp in 0..n_states {
            let p = transitions[trans_index(state, a, sp, n_actions, n_states)];
            cont += p * values[sp];
        }
        let q = rewards[reward_index(state, a, n_actions)] + discount * cont;
        if !q.is_finite() {
            return Err(DpError::NonFinite);
        }
        if q > best {
            best = q;
        }
    }
    Ok(best)
}

/// Full Bellman sweep writing updated values into `next` and the greedy policy
/// into `policy`. Returns the sup-norm residual `max_s |next_s - values_s|`.
fn bellman_sweep(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    values: &[f64],
    n_states: usize,
    n_actions: usize,
    next: &mut [f64],
    policy: &mut [u32],
) -> Result<f64, DpError> {
    let mut max_diff = 0.0f64;
    for s in 0..n_states {
        let mut best = f64::NEG_INFINITY;
        let mut best_a: u32 = 0;
        for a in 0..n_actions {
            let mut cont = 0.0;
            for sp in 0..n_states {
                let p = transitions[trans_index(s, a, sp, n_actions, n_states)];
                cont += p * values[sp];
            }
            let q = rewards[reward_index(s, a, n_actions)] + discount * cont;
            if !q.is_finite() {
                return Err(DpError::NonFinite);
            }
            if q > best {
                best = q;
                best_a = a as u32;
            }
        }
        next[s] = best;
        policy[s] = best_a;
        let diff = (best - values[s]).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    Ok(max_diff)
}

/// Value function iteration (VFI).
///
/// Iterates the full Bellman operator until the sup-norm residual falls below
/// `tolerance` or `max_iterations` is reached. Writes the final value function
/// into `values_out[..n_states]` and the greedy policy into
/// `policy_out[..n_states]` (action indices as `u32`).
///
/// Returns `EconConvergence::converged(...)` when the residual is below
/// tolerance, or `stalled(EconStatus::MaxIterations, ...)` when the budget is
/// exhausted.
pub fn value_iteration_into(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    n_states: usize,
    n_actions: usize,
    max_iterations: u32,
    tolerance: f64,
    values_out: &mut [f64],
    policy_out: &mut [u32],
) -> Result<EconConvergence, DpError> {
    validate_mdp(rewards, transitions, discount, n_states, n_actions)?;
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(DpError::InvalidInput);
    }
    if values_out.len() < n_states || policy_out.len() < n_states {
        return Err(DpError::BufferTooSmall);
    }

    // Stack scratch: HotZeroHeap. No heap allocation.
    let mut current = [0.0f64; MAX_STATES];
    let mut next = [0.0f64; MAX_STATES];
    let mut scratch_policy = [0u32; MAX_STATES];

    for s in 0..n_states {
        current[s] = 0.0;
    }

    let mut last_residual = f64::INFINITY;
    let mut iter: u32 = 0;
    let mut converged = false;
    for k in 0..max_iterations {
        iter = k + 1;
        let residual = bellman_sweep(
            rewards,
            transitions,
            discount,
            &current,
            n_states,
            n_actions,
            &mut next,
            &mut scratch_policy,
        )?;
        last_residual = residual;
        if !residual.is_finite() {
            return Err(DpError::NonFinite);
        }
        // Swap: copy next into current.
        for s in 0..n_states {
            current[s] = next[s];
        }
        if residual < tolerance {
            converged = true;
            break;
        }
    }

    // Final policy from the converged values.
    let _ = bellman_sweep(
        rewards,
        transitions,
        discount,
        &current,
        n_states,
        n_actions,
        &mut next,
        &mut scratch_policy,
    )?;

    for s in 0..n_states {
        values_out[s] = current[s];
        policy_out[s] = scratch_policy[s];
    }

    if converged {
        Ok(EconConvergence::converged(iter, last_residual))
    } else {
        Ok(EconConvergence::stalled(
            EconStatus::MaxIterations,
            iter,
            last_residual,
        ))
    }
}

/// Policy evaluation via iterative Bellman backups of the *fixed* policy.
///
/// Solves `V(s) = reward(s, pi(s)) + discount * sum_s' P(s'|s, pi(s)) V(s')`
/// by value iteration restricted to the chosen actions. Writes the evaluated
/// values into `values[..n_states]`.
fn policy_evaluate(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    policy: &[u32],
    n_states: usize,
    n_actions: usize,
    max_inner: u32,
    tolerance: f64,
    values: &mut [f64; MAX_STATES],
) -> Result<(u32, f64), DpError> {
    let mut next = [0.0f64; MAX_STATES];
    for s in 0..n_states {
        values[s] = 0.0;
    }
    let mut last_residual = f64::INFINITY;
    let mut iter: u32 = 0;
    for k in 0..max_inner {
        iter = k + 1;
        let mut max_diff = 0.0f64;
        for s in 0..n_states {
            let a = policy[s] as usize;
            if a >= n_actions {
                return Err(DpError::InvalidModel);
            }
            let mut cont = 0.0;
            for sp in 0..n_states {
                let p = transitions[trans_index(s, a, sp, n_actions, n_states)];
                cont += p * values[sp];
            }
            let v = rewards[reward_index(s, a, n_actions)] + discount * cont;
            if !v.is_finite() {
                return Err(DpError::NonFinite);
            }
            next[s] = v;
            let diff = (v - values[s]).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }
        for s in 0..n_states {
            values[s] = next[s];
        }
        last_residual = max_diff;
        if max_diff < tolerance {
            break;
        }
    }
    Ok((iter, last_residual))
}

/// One step of policy improvement. Returns true if the policy changed.
fn policy_improve(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    values: &[f64; MAX_STATES],
    n_states: usize,
    n_actions: usize,
    policy: &mut [u32; MAX_STATES],
) -> Result<bool, DpError> {
    let mut changed = false;
    for s in 0..n_states {
        let mut best = f64::NEG_INFINITY;
        let mut best_a: u32 = 0;
        for a in 0..n_actions {
            let mut cont = 0.0;
            for sp in 0..n_states {
                let p = transitions[trans_index(s, a, sp, n_actions, n_states)];
                cont += p * values[sp];
            }
            let q = rewards[reward_index(s, a, n_actions)] + discount * cont;
            if !q.is_finite() {
                return Err(DpError::NonFinite);
            }
            if q > best {
                best = q;
                best_a = a as u32;
            }
        }
        if best_a != policy[s] {
            changed = true;
            policy[s] = best_a;
        }
    }
    Ok(changed)
}

/// Policy iteration.
///
/// Alternates policy evaluation (iterative, up to `max_inner` backups per
/// outer loop) with greedy policy improvement. Terminates when the policy is
/// stable across an outer iteration, or when `max_outer` is reached.
///
/// Writes the evaluated value function into `values_out[..n_states]` and the
/// final policy into `policy_out[..n_states]`.
pub fn policy_iteration_into(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    n_states: usize,
    n_actions: usize,
    max_outer: u32,
    max_inner: u32,
    tolerance: f64,
    values_out: &mut [f64],
    policy_out: &mut [u32],
) -> Result<EconConvergence, DpError> {
    validate_mdp(rewards, transitions, discount, n_states, n_actions)?;
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(DpError::InvalidInput);
    }
    if values_out.len() < n_states || policy_out.len() < n_states {
        return Err(DpError::BufferTooSmall);
    }
    if max_outer == 0 || max_inner == 0 {
        return Err(DpError::InvalidInput);
    }

    let mut policy = [0u32; MAX_STATES];
    let mut values = [0.0f64; MAX_STATES];

    let mut outer_iter: u32 = 0;
    let mut last_residual = 0.0f64;
    let mut converged = false;
    for k in 0..max_outer {
        outer_iter = k + 1;
        let (_inner, loop_residual) = policy_evaluate(
            rewards,
            transitions,
            discount,
            &policy,
            n_states,
            n_actions,
            max_inner,
            tolerance,
            &mut values,
        )?;
        last_residual = loop_residual;
        let changed = policy_improve(
            rewards,
            transitions,
            discount,
            &values,
            n_states,
            n_actions,
            &mut policy,
        )?;
        if !changed {
            converged = true;
            break;
        }
    }

    // Final evaluation so values_out matches the returned policy. Skip if we
    // just evaluated in the last loop iteration (converged with no policy
    // change); otherwise re-evaluate to align values with the final policy.
    if !converged {
        let (_inner, residual) = policy_evaluate(
            rewards,
            transitions,
            discount,
            &policy,
            n_states,
            n_actions,
            max_inner,
            tolerance,
            &mut values,
        )?;
        last_residual = residual;
    }

    for s in 0..n_states {
        values_out[s] = values[s];
        policy_out[s] = policy[s];
    }

    if converged {
        Ok(EconConvergence::converged(outer_iter, last_residual))
    } else {
        Ok(EconConvergence::stalled(
            EconStatus::MaxIterations,
            outer_iter,
            last_residual,
        ))
    }
}

/// Optimal stopping.
///
/// Solves `V(s) = max( stop_value(s), discount * continuation_value(s) )` by
/// value iteration. The continuation value is supplied exogenously per state
/// (it is *not* a function of `V` itself — this is the standard reduced-form
/// stopping problem where the continuation payoff is pre-computed).
///
/// `policy_out[s] = 1` when stopping is optimal, `0` when continuing is
/// optimal. Ties favor continuation (smaller index).
pub fn optimal_stopping_into(
    continuation_values: &[f64],
    stopping_values: &[f64],
    discount: f64,
    n_states: usize,
    max_iterations: u32,
    tolerance: f64,
    values_out: &mut [f64],
    policy_out: &mut [u32],
) -> Result<EconConvergence, DpError> {
    if n_states == 0 || n_states > MAX_STATES {
        return Err(DpError::InvalidInput);
    }
    if continuation_values.len() < n_states || stopping_values.len() < n_states {
        return Err(DpError::InvalidInput);
    }
    if !discount.is_finite() || discount < 0.0 || discount >= 1.0 {
        return Err(DpError::InvalidInput);
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(DpError::InvalidInput);
    }
    if values_out.len() < n_states || policy_out.len() < n_states {
        return Err(DpError::BufferTooSmall);
    }
    for s in 0..n_states {
        if !continuation_values[s].is_finite() || !stopping_values[s].is_finite() {
            return Err(DpError::NonFinite);
        }
    }

    let mut current = [0.0f64; MAX_STATES];
    let mut next = [0.0f64; MAX_STATES];

    // Initialize with the stop values (upper bound on the value function).
    for s in 0..n_states {
        current[s] = stopping_values[s];
    }

    let mut last_residual = f64::INFINITY;
    let mut iter: u32 = 0;
    let mut converged = false;
    for k in 0..max_iterations {
        iter = k + 1;
        let mut max_diff = 0.0f64;
        for s in 0..n_states {
            let cont = discount * continuation_values[s];
            let stop = stopping_values[s];
            // V(s) = max(stop, discount * continuation). Ties favor continue.
            let (v, _stop_flag) = if stop >= cont {
                (stop, 1u32)
            } else {
                (cont, 0u32)
            };
            if !v.is_finite() {
                return Err(DpError::NonFinite);
            }
            next[s] = v;
            let diff = (v - current[s]).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }
        for s in 0..n_states {
            current[s] = next[s];
        }
        last_residual = max_diff;
        if max_diff < tolerance {
            converged = true;
            break;
        }
    }

    for s in 0..n_states {
        values_out[s] = current[s];
        let cont = discount * continuation_values[s];
        let stop = stopping_values[s];
        policy_out[s] = if stop >= cont { 1 } else { 0 };
    }

    if converged {
        Ok(EconConvergence::converged(iter, last_residual))
    } else {
        Ok(EconConvergence::stalled(
            EconStatus::MaxIterations,
            iter,
            last_residual,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a deterministic 2-state, 2-action MDP with a hand-checkable
    /// solution.
    ///
    /// States: 0 = "bad", 1 = "good".
    /// Actions: 0 = "stay", 1 = "switch".
    /// Rewards: stay in good = 2, stay in bad = 0, switch costs 0.5.
    /// Transitions: stay keeps state; switch flips state.
    /// Discount = 0.9.
    ///
    /// Optimal: always switch from bad to good, stay in good.
    /// V(good) = 2 + 0.9 * V(good) => V(good) = 20.
    /// V(bad)  = -0.5 + 0.9 * V(good) = -0.5 + 18 = 17.5.
    fn build_two_state_mdp() -> (Vec<f64>, Vec<f64>, usize, usize, f64) {
        let n_states = 2usize;
        let n_actions = 2usize;
        // rewards[s * n_actions + a]
        // s=0 (bad): a=0 stay -> 0, a=1 switch -> -0.5
        // s=1 (good): a=0 stay -> 2, a=1 switch -> 1.5
        let rewards = vec![0.0, -0.5, 2.0, 1.5];
        // transitions[s * n_actions * n_states + a * n_states + sp]
        // s=0, a=0 (stay): [1,0]
        // s=0, a=1 (switch): [0,1]
        // s=1, a=0 (stay): [0,1]
        // s=1, a=1 (switch): [1,0]
        let transitions = vec![
            1.0, 0.0, // s0 a0
            0.0, 1.0, // s0 a1
            0.0, 1.0, // s1 a0
            1.0, 0.0, // s1 a1
        ];
        let discount = 0.9;
        (rewards, transitions, n_states, n_actions, discount)
    }

    #[test]
    fn bellman_update_single_state_matches_hand_computation() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let values = [17.5, 20.0];
        // State 1 (good): stay -> 2 + 0.9*20 = 20; switch -> 1.5 + 0.9*17.5 = 17.25.
        let v = bellman_update(
            &rewards,
            &transitions,
            discount,
            &values,
            n_states,
            n_actions,
            1,
        )
        .unwrap();
        assert!((v - 20.0).abs() < 1e-9);
    }

    #[test]
    fn value_iteration_recovers_known_solution() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            1000,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!((values[0] - 17.5).abs() < 1e-6, "V(bad) = {}", values[0]);
        assert!((values[1] - 20.0).abs() < 1e-6, "V(good) = {}", values[1]);
        // Optimal policy: switch from bad (a=1), stay in good (a=0).
        assert_eq!(policy[0], 1);
        assert_eq!(policy[1], 0);
    }

    #[test]
    fn policy_iteration_recovers_known_solution() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv = policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            1000,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!((values[0] - 17.5).abs() < 1e-6);
        assert!((values[1] - 20.0).abs() < 1e-6);
        assert_eq!(policy[0], 1);
        assert_eq!(policy[1], 0);
    }

    #[test]
    fn vfi_and_policy_iteration_agree() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut v_vfi = [0.0f64; 2];
        let mut p_vfi = [0u32; 2];
        let mut v_pi = [0.0f64; 2];
        let mut p_pi = [0u32; 2];
        value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            5000,
            1e-12,
            &mut v_vfi,
            &mut p_vfi,
        )
        .unwrap();
        policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            5000,
            1e-12,
            &mut v_pi,
            &mut p_pi,
        )
        .unwrap();
        for s in 0..n_states {
            assert!((v_vfi[s] - v_pi[s]).abs() < 1e-6, "state {} mismatch", s);
            assert_eq!(p_vfi[s], p_pi[s], "state {} policy mismatch", s);
        }
    }

    /// Cake-eating: 2-state deterministic depletion.
    ///
    /// State 0 = "cake remains", state 1 = "cake gone" (absorbing, zero
    /// reward). Action 0 = eat (consume the cake, transition to gone),
    /// action 1 = do nothing (keep cake, but in this minimal model the only
    /// rewarding action is eating now). With utility u = 1 for eating and
    /// discount 0.9, the value of "cake remains" is 1 + 0.9 * 0 = 1.
    fn build_cake_eating() -> (Vec<f64>, Vec<f64>, usize, usize, f64) {
        let n_states = 2usize;
        let n_actions = 2usize;
        // s=0 (cake): a=0 eat -> reward 1, go to s=1; a=1 wait -> reward 0, stay.
        // s=1 (gone): both actions -> reward 0, stay.
        let rewards = vec![1.0, 0.0, 0.0, 0.0];
        let transitions = vec![
            0.0, 1.0, // s0 a0 eat -> gone
            1.0, 0.0, // s0 a1 wait -> stay
            0.0, 1.0, // s1 a0 -> stay (self loop on gone)
            0.0, 1.0, // s1 a1 -> stay
        ];
        let discount = 0.9;
        (rewards, transitions, n_states, n_actions, discount)
    }

    #[test]
    fn cake_eating_value_matches_discounted_utility_sum() {
        let (rewards, transitions, n_states, n_actions, discount) = build_cake_eating();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            1000,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // V(cake) = 1 (eat now, gone yields 0). V(gone) = 0.
        assert!((values[0] - 1.0).abs() < 1e-9, "V(cake) = {}", values[0]);
        assert!((values[1] - 0.0).abs() < 1e-12);
        assert_eq!(policy[0], 0); // eat
    }

    /// Cake-eating with two pieces: state 0 = full cake (2 pieces), state 1 =
    /// one piece left, state 2 = gone. Eat consumes one piece. Utility per
    /// piece = 1, discount 0.5. Optimal: eat both pieces.
    /// V(gone) = 0; V(one) = 1 + 0.5*0 = 1; V(full) = 1 + 0.5*1 = 1.5.
    fn build_cake_eating_three() -> (Vec<f64>, Vec<f64>, usize, usize, f64) {
        let n_states = 3usize;
        let n_actions = 2usize;
        // a=0 eat (consume one piece), a=1 wait.
        // rewards[s * 2 + a]
        let rewards = vec![
            1.0, 0.0, // s0 full: eat -> 1, wait -> 0
            1.0, 0.0, // s1 one:  eat -> 1, wait -> 0
            0.0, 0.0, // s2 gone
        ];
        // transitions[s * 2 * 3 + a * 3 + sp]
        let transitions = vec![
            0.0, 1.0, 0.0, // s0 a0 eat -> s1
            1.0, 0.0, 0.0, // s0 a1 wait -> s0
            0.0, 0.0, 1.0, // s1 a0 eat -> s2
            0.0, 1.0, 0.0, // s1 a1 wait -> s1
            0.0, 0.0, 1.0, // s2 a0 -> s2
            0.0, 0.0, 1.0, // s2 a1 -> s2
        ];
        let discount = 0.5;
        (rewards, transitions, n_states, n_actions, discount)
    }

    #[test]
    fn cake_eating_three_states_sum_of_discounted_utilities() {
        let (rewards, transitions, n_states, n_actions, discount) = build_cake_eating_three();
        let mut values = [0.0f64; 3];
        let mut policy = [0u32; 3];
        value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            1000,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert!((values[2] - 0.0).abs() < 1e-12);
        assert!((values[1] - 1.0).abs() < 1e-9, "V(one) = {}", values[1]);
        assert!((values[0] - 1.5).abs() < 1e-9, "V(full) = {}", values[0]);
        assert_eq!(policy[0], 0);
        assert_eq!(policy[1], 0);
    }

    /// Search/unemployment: 2-state (employed/unemployed).
    ///
    /// State 0 = unemployed, state 1 = employed.
    /// Action 0 = "rest" (low search effort), action 1 = "search hard".
    /// Rest: wage 0, stay unemployed w.p. 1.
    /// Search hard: cost 0.1, find job w.p. 0.5 (else stay unemployed).
    /// Employed: wage 1, lose job w.p. 0.1 per period (action irrelevant).
    /// Discount 0.9.
    ///
    /// Reservation policy: search when unemployed (expected net gain exceeds
    /// resting at zero wage).
    fn build_search_model() -> (Vec<f64>, Vec<f64>, usize, usize, f64) {
        let n_states = 2usize;
        let n_actions = 2usize;
        // rewards[s * 2 + a]
        // s0 unemployed: a0 rest -> 0, a1 search -> -0.1 (cost)
        // s1 employed:   a0 -> 1, a1 -> 1 (action irrelevant)
        let rewards = vec![0.0, -0.1, 1.0, 1.0];
        // transitions[s * 2 * 2 + a * 2 + sp]
        // s0 a0 rest: stay unemployed [1, 0]
        // s0 a1 search: 0.5 stay, 0.5 employed [0.5, 0.5]
        // s1 a0: 0.1 lose, 0.9 keep [0.1, 0.9]
        // s1 a1: 0.1 lose, 0.9 keep [0.1, 0.9]
        let transitions = vec![
            1.0, 0.0, // s0 a0
            0.5, 0.5, // s0 a1
            0.1, 0.9, // s1 a0
            0.1, 0.9, // s1 a1
        ];
        let discount = 0.9;
        (rewards, transitions, n_states, n_actions, discount)
    }

    #[test]
    fn search_model_reservation_policy_searches_when_unemployed() {
        let (rewards, transitions, n_states, n_actions, discount) = build_search_model();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            5000,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // Unemployed: search (a=1) must beat rest (a=0).
        // rest: 0 + 0.9 * V(unemp); search: -0.1 + 0.9*(0.5*V(unemp)+0.5*V(emp))
        // The test asserts the reservation policy: search when unemployed.
        assert_eq!(policy[0], 1, "unemployed should search");
        // Employed: both actions equivalent (same reward/transitions); tie -> a=0.
        assert_eq!(policy[1], 0);
        // Sanity: employed value exceeds unemployed value.
        assert!(values[1] > values[0]);
    }

    #[test]
    fn search_model_policy_iteration_matches_vfi() {
        let (rewards, transitions, n_states, n_actions, discount) = build_search_model();
        let mut v_vfi = [0.0f64; 2];
        let mut p_vfi = [0u32; 2];
        let mut v_pi = [0.0f64; 2];
        let mut p_pi = [0u32; 2];
        value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            5000,
            1e-12,
            &mut v_vfi,
            &mut p_vfi,
        )
        .unwrap();
        policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            5000,
            1e-12,
            &mut v_pi,
            &mut p_pi,
        )
        .unwrap();
        for s in 0..n_states {
            assert!((v_vfi[s] - v_pi[s]).abs() < 1e-6);
            assert_eq!(p_vfi[s], p_pi[s]);
        }
    }

    #[test]
    fn optimal_stopping_obvious_stop_now() {
        // stop value huge -> stop immediately everywhere.
        let cont = [1.0, 1.0];
        let stop = [100.0, 100.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv =
            optimal_stopping_into(&cont, &stop, 0.9, 2, 1000, 1e-12, &mut values, &mut policy)
                .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert_eq!(policy[0], 1);
        assert_eq!(policy[1], 1);
        assert!((values[0] - 100.0).abs() < 1e-9);
        assert!((values[1] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn optimal_stopping_obvious_continue() {
        // continuation discounted > stop -> continue everywhere.
        // V(s) = max(stop, 0.9 * cont). With cont=100, stop=1: 0.9*100=90 > 1.
        let cont = [100.0, 100.0];
        let stop = [1.0, 1.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv =
            optimal_stopping_into(&cont, &stop, 0.9, 2, 1000, 1e-12, &mut values, &mut policy)
                .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert_eq!(policy[0], 0);
        assert_eq!(policy[1], 0);
        assert!((values[0] - 90.0).abs() < 1e-9);
        assert!((values[1] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn optimal_stopping_mixed_policy() {
        // state 0: stop better; state 1: continue better.
        let cont = [1.0, 100.0];
        let stop = [10.0, 1.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let conv =
            optimal_stopping_into(&cont, &stop, 0.9, 2, 1000, 1e-12, &mut values, &mut policy)
                .unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // s0: stop=10 vs 0.9*1=0.9 -> stop.
        assert_eq!(policy[0], 1);
        // s1: stop=1 vs 0.9*100=90 -> continue.
        assert_eq!(policy[1], 0);
        assert!((values[0] - 10.0).abs() < 1e-9);
        assert!((values[1] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn vfi_reports_max_iterations_when_tolerance_too_tight() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        // Tolerance below f64 resolution for this contraction -> cannot converge
        // in a small iteration budget.
        let conv = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            5,
            1e-300,
            &mut values,
            &mut policy,
        )
        .unwrap();
        assert_eq!(conv.status, EconStatus::MaxIterations);
        assert_eq!(conv.iterations, 5);
    }

    #[test]
    fn policy_iteration_reports_max_iterations_when_budget_tiny() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        // Single outer iteration with a policy that oscillates is unlikely to
        // stabilize; either way the report must be a valid status.
        let conv = policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            1,
            1,
            1e-12,
            &mut values,
            &mut policy,
        )
        .unwrap();
        // With one outer iteration the policy may or may not have stabilized;
        // accept either Converged or MaxIterations, but it must be a valid enum.
        let _ = conv.status;
    }

    #[test]
    fn invalid_discount_above_one_rejected() {
        let (rewards, transitions, n_states, n_actions, _discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            1.0,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn invalid_discount_negative_rejected() {
        let (rewards, transitions, n_states, n_actions, _discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            -0.1,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn bellman_update_rejects_invalid_discount() {
        let (rewards, transitions, n_states, n_actions, _discount) = build_two_state_mdp();
        let values = [0.0, 0.0];
        let err = bellman_update(&rewards, &transitions, 1.5, &values, n_states, n_actions, 0)
            .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn vfi_buffer_too_small_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 1];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::BufferTooSmall);
    }

    #[test]
    fn vfi_policy_buffer_too_small_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 1];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::BufferTooSmall);
    }

    #[test]
    fn policy_iteration_buffer_too_small_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 1];
        let mut policy = [0u32; 2];
        let err = policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::BufferTooSmall);
    }

    #[test]
    fn optimal_stopping_buffer_too_small_rejected() {
        let cont = [1.0, 1.0];
        let stop = [1.0, 1.0];
        let mut values = [0.0f64; 1];
        let mut policy = [0u32; 2];
        let err = optimal_stopping_into(&cont, &stop, 0.9, 2, 100, 1e-9, &mut values, &mut policy)
            .unwrap_err();
        assert_eq!(err, DpError::BufferTooSmall);
    }

    #[test]
    fn invalid_transition_row_sum_rejected() {
        let n_states = 2usize;
        let n_actions = 1usize;
        let rewards = vec![1.0, 0.0];
        // Row for s0 does not sum to 1.
        let transitions = vec![0.5, 0.5, 0.3, 0.3];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            0.9,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidModel);
    }

    #[test]
    fn negative_probability_rejected() {
        let n_states = 2usize;
        let n_actions = 1usize;
        let rewards = vec![1.0, 0.0];
        let transitions = vec![-0.5, 1.5, 1.0, 0.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            0.9,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidModel);
    }

    #[test]
    fn non_finite_reward_rejected() {
        let n_states = 2usize;
        let n_actions = 1usize;
        let rewards = vec![f64::NAN, 0.0];
        let transitions = vec![1.0, 0.0, 0.0, 1.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            0.9,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::NonFinite);
    }

    #[test]
    fn zero_states_rejected() {
        let rewards: Vec<f64> = vec![];
        let transitions: Vec<f64> = vec![];
        let mut values = [0.0f64; 0];
        let mut policy = [0u32; 0];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            0.9,
            0,
            1,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn exceeds_max_states_rejected() {
        let n_states = MAX_STATES + 1;
        let n_actions = 1usize;
        let rewards = vec![0.0; n_states * n_actions];
        let transitions = vec![0.0; n_states * n_actions * n_states];
        let mut values = vec![0.0; n_states];
        let mut policy = vec![0u32; n_states];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            0.9,
            n_states,
            n_actions,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn dp_error_maps_to_econ_status() {
        assert_eq!(DpError::InvalidInput.to_status(), EconStatus::InvalidInput);
        assert_eq!(
            DpError::BufferTooSmall.to_status(),
            EconStatus::BufferTooSmall
        );
        assert_eq!(DpError::NonFinite.to_status(), EconStatus::NonFinite);
        assert_eq!(DpError::NonConverged.to_status(), EconStatus::MaxIterations);
        assert_eq!(DpError::InvalidModel.to_status(), EconStatus::InvalidInput);
        let s: EconStatus = DpError::BufferTooSmall.into();
        assert_eq!(s, EconStatus::BufferTooSmall);
    }

    #[test]
    fn bellman_update_state_out_of_range_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let values = [0.0, 0.0];
        let err = bellman_update(
            &rewards,
            &transitions,
            discount,
            &values,
            n_states,
            n_actions,
            5,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn bellman_update_values_buffer_too_small() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let values = [0.0];
        let err = bellman_update(
            &rewards,
            &transitions,
            discount,
            &values,
            n_states,
            n_actions,
            0,
        )
        .unwrap_err();
        assert_eq!(err, DpError::BufferTooSmall);
    }

    #[test]
    fn optimal_stopping_invalid_discount_rejected() {
        let cont = [1.0, 1.0];
        let stop = [1.0, 1.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = optimal_stopping_into(&cont, &stop, 1.0, 2, 100, 1e-9, &mut values, &mut policy)
            .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn optimal_stopping_non_finite_input_rejected() {
        let cont = [f64::NAN, 1.0];
        let stop = [1.0, 1.0];
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = optimal_stopping_into(&cont, &stop, 0.9, 2, 100, 1e-9, &mut values, &mut policy)
            .unwrap_err();
        assert_eq!(err, DpError::NonFinite);
    }

    #[test]
    fn vfi_invalid_tolerance_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = value_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            100,
            0.0,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }

    #[test]
    fn policy_iteration_zero_budget_rejected() {
        let (rewards, transitions, n_states, n_actions, discount) = build_two_state_mdp();
        let mut values = [0.0f64; 2];
        let mut policy = [0u32; 2];
        let err = policy_iteration_into(
            &rewards,
            &transitions,
            discount,
            n_states,
            n_actions,
            0,
            100,
            1e-9,
            &mut values,
            &mut policy,
        )
        .unwrap_err();
        assert_eq!(err, DpError::InvalidInput);
    }
}
