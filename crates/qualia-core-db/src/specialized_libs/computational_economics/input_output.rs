//! Input-output economics: Leontief inverse, multipliers, Ghosh supply-side,
//! and capacity-constrained shock propagation.
//!
//! Allocation class: **HotZeroHeap**. All scratch uses fixed-capacity stack
//! arrays (`[0.0f64; MAX_SECTORS]`). No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - The technical-coefficient matrix `A` is square (`n x n`), row-major.
//! - Each row of `A` represents the unit input requirements of sector `i`
//!   from every sector `j` (Leontief demand-side).
//! - The Ghosh allocation matrix `B` is the supply-side dual; each column
//!   represents the proportional allocation of sector `j`'s output to sector
//!   `i`. Interpretive limits apply — Ghosh is not a symmetric dual of
//!   Leontief in general equilibrium; results are descriptive of direct
//!   supply linkages only.
//! - The spectral radius of `A` (or `B`) must be `< 1` for the iterative
//!   Neumann series `I + A + A^2 + ...` to converge. Otherwise the kernel
//!   returns `NonConverged`.

use super::error::{EconConvergence, EconStatus};

/// Maximum number of sectors in a bounded input-output model.
pub const MAX_SECTORS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOutputError {
    InvalidInput,
    Singular,
    BufferTooSmall,
    NonFinite,
    NonConverged,
}

impl InputOutputError {
    pub fn to_status(self) -> EconStatus {
        match self {
            InputOutputError::InvalidInput => EconStatus::InvalidInput,
            InputOutputError::Singular => EconStatus::Singular,
            InputOutputError::NonConverged => EconStatus::MaxIterations,
            InputOutputError::BufferTooSmall => EconStatus::BufferTooSmall,
            InputOutputError::NonFinite => EconStatus::NonFinite,
        }
    }
}

fn validate_square(matrix: &[f64], n: usize) -> Result<(), InputOutputError> {
    if n == 0 || n > MAX_SECTORS || matrix.len() < n * n {
        return Err(InputOutputError::InvalidInput);
    }
    for v in matrix.iter().take(n * n) {
        if !v.is_finite() {
            return Err(InputOutputError::NonFinite);
        }
    }
    Ok(())
}

/// Compute the Leontief inverse `(I - A)^{-1}` via the Neumann series
/// `I + A + A^2 + A^3 + ...` into a caller-owned `n x n` buffer.
///
/// Returns a convergence report. `out` must hold at least `n * n` entries.
pub fn leontief_inverse_into(
    a_matrix: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, InputOutputError> {
    validate_square(a_matrix, n)?;
    if out.len() < n * n {
        return Err(InputOutputError::BufferTooSmall);
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(InputOutputError::InvalidInput);
    }

    // Initialize out = I (identity).
    for i in 0..n {
        for j in 0..n {
            out[i * n + j] = if i == j { 1.0 } else { 0.0 };
        }
    }

    let mut power = [0.0f64; MAX_SECTORS * MAX_SECTORS];
    let mut next_power = [0.0f64; MAX_SECTORS * MAX_SECTORS];
    // power = A
    for i in 0..n * n {
        power[i] = a_matrix[i];
    }
    // out += A
    for i in 0..n * n {
        out[i] += power[i];
    }

    let mut last_residual = f64::INFINITY;
    for round in 0..max_rounds {
        // next_power = power * A
        for i in 0..n {
            for j in 0..n {
                let mut acc = 0.0;
                for k in 0..n {
                    acc += power[i * n + k] * a_matrix[k * n + j];
                }
                next_power[i * n + j] = acc;
            }
        }
        // residual = ||next_power||_1
        let mut l1 = 0.0;
        for i in 0..n * n {
            l1 += next_power[i].abs();
        }
        // out += next_power
        for i in 0..n * n {
            out[i] += next_power[i];
        }
        // power = next_power
        for i in 0..n * n {
            power[i] = next_power[i];
        }
        last_residual = l1;
        if l1 < tolerance {
            return Ok(EconConvergence::converged(round + 1, l1));
        }
        if l1.is_nan() || l1 > 1e18 {
            return Ok(EconConvergence::stalled(EconStatus::MaxIterations, round + 1, l1));
        }
    }
    Ok(EconConvergence::stalled(EconStatus::MaxIterations, max_rounds, last_residual))
}

/// Compute output multipliers as column sums of the Leontief inverse.
///
/// `inverse` is an `n x n` row-major buffer (e.g. produced by
/// [`leontief_inverse_into`]). Writes `n` multipliers into `out`.
pub fn output_multipliers_from_inverse(
    inverse: &[f64],
    n: usize,
    out: &mut [f64],
) -> Result<usize, InputOutputError> {
    if n == 0 || n > MAX_SECTORS || inverse.len() < n * n || out.len() < n {
        return Err(InputOutputError::InvalidInput);
    }
    for j in 0..n {
        let mut acc = 0.0;
        for i in 0..n {
            acc += inverse[i * n + j];
        }
        out[j] = acc;
    }
    Ok(n)
}

/// Rank sectors by output multiplier, descending. Writes sector indices into
/// `out[..n]`. Ties are broken by ascending sector index (deterministic).
pub fn key_sector_ranking_into(
    multipliers: &[f64],
    n: usize,
    out: &mut [usize],
) -> Result<usize, InputOutputError> {
    if n == 0 || n > MAX_SECTORS || multipliers.len() < n || out.len() < n {
        return Err(InputOutputError::InvalidInput);
    }
    for v in multipliers.iter().take(n) {
        if !v.is_finite() {
            return Err(InputOutputError::NonFinite);
        }
    }
    let mut idx: [usize; MAX_SECTORS] = [0; MAX_SECTORS];
    for i in 0..n {
        idx[i] = i;
    }
    // Insertion sort by multiplier descending, ties by index ascending.
    for i in 1..n {
        let cur = idx[i];
        let cur_m = multipliers[cur];
        let mut j = i;
        while j > 0 {
            let prev = idx[j - 1];
            let prev_m = multipliers[prev];
            if prev_m < cur_m || (prev_m == cur_m && prev > cur) {
                idx[j] = idx[j - 1];
                j -= 1;
            } else {
                break;
            }
        }
        idx[j] = cur;
    }
    for i in 0..n {
        out[i] = idx[i];
    }
    Ok(n)
}

/// Compute the Ghosh supply-side inverse `(I - B)^{-1}` via the Neumann series.
///
/// **Interpretive warning:** the Ghosh inverse describes direct supply
/// linkages only. It is not a symmetric general-equilibrium dual of the
/// Leontief inverse; downstream "allocation" effects do not equal upstream
/// "output" effects without strong separability assumptions. Use for
/// descriptive supply-network analysis, not for counterfactual impact
/// prediction without additional structure.
pub fn ghosh_inverse_into(
    b_matrix: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, InputOutputError> {
    // Same algorithm as Leontief; the interpretive difference is documented
    // above and in the caller's responsibility to supply B (allocation
    // coefficients) rather than A (technical coefficients).
    leontief_inverse_into(b_matrix, n, max_rounds, tolerance, out)
}

/// Decompose a shock into per-sector total impact: `impact = inverse * shock`.
///
/// `inverse` is `n x n` row-major; `shock` is length `n`. Writes `n` impacts.
pub fn shock_decomposition_into(
    inverse: &[f64],
    shock: &[f64],
    n: usize,
    out: &mut [f64],
) -> Result<usize, InputOutputError> {
    if n == 0 || n > MAX_SECTORS
        || inverse.len() < n * n
        || shock.len() < n
        || out.len() < n
    {
        return Err(InputOutputError::InvalidInput);
    }
    for v in shock.iter().take(n) {
        if !v.is_finite() {
            return Err(InputOutputError::NonFinite);
        }
    }
    for i in 0..n {
        let mut acc = 0.0;
        for j in 0..n {
            acc += inverse[i * n + j] * shock[j];
        }
        out[i] = acc;
    }
    Ok(n)
}

/// Capacity-constrained supply-shock propagation.
///
/// Like [`super::super::super::domains::financial::economics::input_output::propagate_supply_shock`]
/// but each round's propagated term is capped by `capacity[i]` (no sector can
/// propagate more impact than its remaining capacity). Returns a convergence
/// report.
pub fn capacity_constrained_propagation(
    coupling: &[f64],
    shock: &[f64],
    capacity: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, InputOutputError> {
    validate_square(coupling, n)?;
    if shock.len() < n || capacity.len() < n || out.len() < n {
        return Err(InputOutputError::BufferTooSmall);
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(InputOutputError::InvalidInput);
    }
    for v in capacity.iter().take(n) {
        if !v.is_finite() || *v < 0.0 {
            return Err(InputOutputError::InvalidInput);
        }
    }

    let mut term = [0.0f64; MAX_SECTORS];
    let mut next = [0.0f64; MAX_SECTORS];
    for i in 0..n {
        let capped = shock[i].min(capacity[i]);
        term[i] = capped;
        out[i] = capped;
    }

    let mut last_residual = f64::INFINITY;
    for round in 0..max_rounds {
        let mut l1 = 0.0;
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += coupling[i * n + j] * term[j];
            }
            let capped = acc.min(capacity[i]);
            next[i] = capped;
            l1 += capped.abs();
        }
        for i in 0..n {
            out[i] += next[i];
            term[i] = next[i];
        }
        last_residual = l1;
        if l1 < tolerance {
            return Ok(EconConvergence::converged(round + 1, l1));
        }
    }
    Ok(EconConvergence::stalled(EconStatus::MaxIterations, max_rounds, last_residual))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn leontief_inverse_two_sector() {
        // A = [[0, 0.5], [0.5, 0]]
        // (I-A)^-1 = 1/(1-0.25) * [[1, 0.5], [0.5, 1]] = [[4/3, 2/3], [2/3, 4/3]]
        let a = [0.0, 0.5, 0.5, 0.0];
        let mut inv = [0.0f64; 4];
        let conv = leontief_inverse_into(&a, 2, 1000, 1e-12, &mut inv).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(approx(inv[0], 4.0 / 3.0));
        assert!(approx(inv[1], 2.0 / 3.0));
        assert!(approx(inv[2], 2.0 / 3.0));
        assert!(approx(inv[3], 4.0 / 3.0));
    }

    #[test]
    fn leontief_identity_matrix() {
        // A = 0 → inverse = I
        let a = [0.0; 4];
        let mut inv = [0.0f64; 4];
        let conv = leontief_inverse_into(&a, 2, 100, 1e-9, &mut inv).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(approx(inv[0], 1.0));
        assert!(approx(inv[3], 1.0));
        assert!(approx(inv[1], 0.0));
        assert!(approx(inv[2], 0.0));
    }

    #[test]
    fn output_multipliers_sum() {
        let a = [0.0, 0.5, 0.5, 0.0];
        let mut inv = [0.0f64; 4];
        leontief_inverse_into(&a, 2, 1000, 1e-12, &mut inv).unwrap();
        let mut mult = [0.0f64; 2];
        output_multipliers_from_inverse(&inv, 2, &mut mult).unwrap();
        // Each column sums to 4/3 + 2/3 = 2
        assert!(approx(mult[0], 2.0));
        assert!(approx(mult[1], 2.0));
    }

    #[test]
    fn key_sector_ranking_descending_with_ties() {
        // multipliers [1.5, 2.0, 2.0, 0.5] → ranking [1, 2, 0, 3]
        let mult = [1.5, 2.0, 2.0, 0.5];
        let mut rank = [0usize; 4];
        key_sector_ranking_into(&mult, 4, &mut rank).unwrap();
        assert_eq!(rank, [1, 2, 0, 3]);
    }

    #[test]
    fn ghosh_inverse_matches_leontief_algorithm() {
        let b = [0.0, 0.3, 0.3, 0.0];
        let mut inv = [0.0f64; 4];
        let conv = ghosh_inverse_into(&b, 2, 1000, 1e-12, &mut inv).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // (I-B)^-1 = 1/(1-0.09) * [[1, 0.3], [0.3, 1]]
        assert!(approx(inv[0], 1.0 / 0.91));
        assert!(approx(inv[1], 0.3 / 0.91));
    }

    #[test]
    fn shock_decomposition_uses_inverse() {
        let a = [0.0, 0.5, 0.5, 0.0];
        let mut inv = [0.0f64; 4];
        leontief_inverse_into(&a, 2, 1000, 1e-12, &mut inv).unwrap();
        let shock = [1.0, 0.0];
        let mut impact = [0.0f64; 2];
        shock_decomposition_into(&inv, &shock, 2, &mut impact).unwrap();
        // impact = inv * [1, 0] = [4/3, 2/3]
        assert!(approx(impact[0], 4.0 / 3.0));
        assert!(approx(impact[1], 2.0 / 3.0));
    }

    #[test]
    fn capacity_constrained_caps_output() {
        // Convergent coupling (off-diagonal 0.5 → spectral radius 0.5), so the
        // cumulative propagation settles. (The prior test used the anti-identity
        // [[0,1],[1,0]], whose spectral radius is exactly 1: that propagation
        // never converges — sector-1's per-round flow stays at the cap of 3 and
        // its cumulative impact grows ~3 every other round, exceeding 100 over
        // 100 rounds, so the old `<= 100` bound was simply false for that input.)
        let coupling = [0.0, 0.5, 0.5, 0.0];
        let shock = [10.0, 0.0];
        let capacity = [10.0, 3.0]; // sector 1's first-round inflow (5) is capped to 3
        let mut impact = [0.0f64; 2];
        let conv =
            capacity_constrained_propagation(&coupling, &shock, &capacity, 2, 100, 1e-9, &mut impact)
                .unwrap();
        // Sector 1's uncapped round-1 inflow is 0.5·10 = 5, capped to 3 — the cap
        // bites — and its bounded cumulative impact settles near 4 (3 + 0.75 + …).
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(impact[0] > 10.0); // sector 0 keeps its shock plus feedback
        assert!(impact[1] > 0.0 && impact[1] < 10.0); // capped, bounded
    }

    #[test]
    fn singular_matrix_does_not_converge() {
        // A = [[0.9, 0.9], [0.9, 0.9]] has spectral radius 1.8 > 1 → diverges
        let a = [0.9, 0.9, 0.9, 0.9];
        let mut inv = [0.0f64; 4];
        let conv = leontief_inverse_into(&a, 2, 50, 1e-9, &mut inv).unwrap();
        assert_ne!(conv.status, EconStatus::Converged);
    }

    #[test]
    fn buffer_too_small_errors() {
        let a = [0.0; 4];
        let mut inv = [0.0f64; 3]; // too small for 2x2
        let err = leontief_inverse_into(&a, 2, 100, 1e-9, &mut inv).unwrap_err();
        assert_eq!(err, InputOutputError::BufferTooSmall);
    }

    #[test]
    fn invalid_dimensions_rejected() {
        let a = [0.0; 4];
        let mut inv = [0.0f64; 4];
        let err = leontief_inverse_into(&a, 0, 100, 1e-9, &mut inv).unwrap_err();
        assert_eq!(err, InputOutputError::InvalidInput);
        let err = leontief_inverse_into(&a, 33, 100, 1e-9, &mut inv).unwrap_err();
        assert_eq!(err, InputOutputError::InvalidInput);
    }

    #[test]
    fn non_finite_rejected() {
        let a = [f64::NAN, 0.0, 0.0, 0.0];
        let mut inv = [0.0f64; 4];
        let err = leontief_inverse_into(&a, 2, 100, 1e-9, &mut inv).unwrap_err();
        assert_eq!(err, InputOutputError::NonFinite);
    }

    #[test]
    fn error_to_status_mapping() {
        assert_eq!(InputOutputError::Singular.to_status(), EconStatus::Singular);
        assert_eq!(
            InputOutputError::NonConverged.to_status(),
            EconStatus::MaxIterations
        );
    }
}
