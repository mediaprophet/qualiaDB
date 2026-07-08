//! Network economics: centrality, default cascade, and interbank clearing.
//!
//! Allocation class: **HotZeroHeap**. All scratch uses fixed-capacity stack
//! arrays. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Eigenvector centrality assumes a connected, non-bipartite graph for
//!   convergence.
//! - Default cascade assumes synchronous rounds: bank i defaults if its
//!   capital is exceeded by exposures to already-defaulted banks.
//! - Eisenberg-Noe clearing assumes proportional repayment in default (all
//!   creditors of a defaulted bank receive pro-rata shares).

use super::error::{EconConvergence, EconStatus};

/// Maximum nodes in a bounded network.
pub const MAX_NODES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    InvalidInput,
    BufferTooSmall,
    NonFinite,
    NonConverged,
}

fn validate_square(matrix: &[f64], n: usize) -> Result<(), NetworkError> {
    if n == 0 || n > MAX_NODES || matrix.len() < n * n {
        return Err(NetworkError::InvalidInput);
    }
    for v in matrix.iter().take(n * n) {
        if !v.is_finite() {
            return Err(NetworkError::NonFinite);
        }
    }
    Ok(())
}

/// Eigenvector centrality via power iteration on the adjacency matrix.
///
/// Writes centrality into `out[..n]`, normalized to unit L2 norm. Returns
/// convergence report.
pub fn eigenvector_centrality_into(
    adjacency: &[f64],
    n: usize,
    max_iterations: u32,
    tolerance: f64,
    out: &mut [f64],
) -> Result<EconConvergence, NetworkError> {
    validate_square(adjacency, n)?;
    if out.len() < n || tolerance <= 0.0 || !tolerance.is_finite() {
        return Err(NetworkError::InvalidInput);
    }
    // Initialize: uniform.
    let init = (1.0 / n as f64).sqrt();
    for i in 0..n {
        out[i] = init;
    }
    let mut next = [0.0f64; MAX_NODES];
    let mut last_residual = f64::INFINITY;
    for round in 0..max_iterations {
        // next = A * out
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += adjacency[i * n + j] * out[j];
            }
            next[i] = acc;
        }
        // Normalize to unit L2.
        let mut norm = 0.0;
        for i in 0..n {
            norm += next[i] * next[i];
        }
        norm = norm.sqrt();
        if norm < 1e-15 {
            return Ok(EconConvergence::stalled(EconStatus::Singular, round + 1, 0.0));
        }
        let mut delta = 0.0;
        for i in 0..n {
            next[i] /= norm;
            delta += (next[i] - out[i]).abs();
            out[i] = next[i];
        }
        last_residual = delta;
        if delta < tolerance {
            return Ok(EconConvergence::converged(round + 1, delta));
        }
    }
    Ok(EconConvergence::stalled(EconStatus::MaxIterations, max_iterations, last_residual))
}

/// Out-degree centrality: row sums of the adjacency matrix.
pub fn degree_centrality_into(
    adjacency: &[f64],
    n: usize,
    out: &mut [f64],
) -> Result<usize, NetworkError> {
    validate_square(adjacency, n)?;
    if out.len() < n {
        return Err(NetworkError::BufferTooSmall);
    }
    for i in 0..n {
        let mut acc = 0.0;
        for j in 0..n {
            acc += adjacency[i * n + j];
        }
        out[i] = acc;
    }
    Ok(n)
}

/// Default cascade: bank i defaults if `capital[i] < sum of exposures to
/// defaulted banks`. Iterates until no new defaults.
///
/// `exposures[i][j]` = bank i's exposure to bank j (n*n row-major).
/// `initially_defaulted` is a slice of bool. Writes final default state into
/// `out[..n]` as f64 (1.0 = defaulted, 0.0 = solvent).
pub fn default_cascade_into(
    exposures: &[f64],
    capital: &[f64],
    n: usize,
    initially_defaulted: &[bool],
    max_rounds: u32,
    out: &mut [f64],
) -> Result<EconConvergence, NetworkError> {
    validate_square(exposures, n)?;
    if capital.len() < n || initially_defaulted.len() < n || out.len() < n {
        return Err(NetworkError::BufferTooSmall);
    }
    for v in capital.iter().take(n) {
        if !v.is_finite() {
            return Err(NetworkError::NonFinite);
        }
    }
    let mut defaulted = [false; MAX_NODES];
    for i in 0..n {
        defaulted[i] = initially_defaulted[i];
        out[i] = if defaulted[i] { 1.0 } else { 0.0 };
    }
    let mut last_new = n;
    let mut rounds = 0u32;
    for round in 0..max_rounds {
        rounds = round + 1;
        let mut new_defaults = 0;
        for i in 0..n {
            if defaulted[i] {
                continue;
            }
            let mut loss = 0.0;
            for j in 0..n {
                if defaulted[j] {
                    loss += exposures[i * n + j];
                }
            }
            if loss > capital[i] {
                defaulted[i] = true;
                new_defaults += 1;
            }
        }
        for i in 0..n {
            out[i] = if defaulted[i] { 1.0 } else { 0.0 };
        }
        if new_defaults == 0 {
            return Ok(EconConvergence::converged(rounds, 0.0));
        }
        last_new = new_defaults;
    }
    let _ = last_new;
    Ok(EconConvergence::stalled(EconStatus::MaxIterations, rounds, 0.0))
}

/// Eisenberg-Noe interbank clearing vector.
///
/// `exposures[i][j]` = bank i's liability to bank j. Bank i's total due is
/// `L_i = sum_j exposures[i][j]`. Bank i receives `R_i = sum_j p_j *
/// exposures[j][i] / L_j` (pro-rata from defaulted banks). Clearing payment
/// `p_i = min(L_i, R_i)`. Iterates until convergence.
///
/// Writes payments into `payments_out[..n]`. Returns convergence report.
pub fn interbank_clearing_into(
    exposures: &[f64],
    capital: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    payments_out: &mut [f64],
) -> Result<EconConvergence, NetworkError> {
    validate_square(exposures, n)?;
    if capital.len() < n || payments_out.len() < n || tolerance <= 0.0 {
        return Err(NetworkError::InvalidInput);
    }
    // Total liabilities.
    let mut total_liab = [0.0f64; MAX_NODES];
    for i in 0..n {
        let mut acc = 0.0;
        for j in 0..n {
            acc += exposures[i * n + j];
        }
        total_liab[i] = acc;
    }
    // Initialize payments = full liabilities (assume all pay in full).
    for i in 0..n {
        payments_out[i] = total_liab[i];
    }
    let mut received = [0.0f64; MAX_NODES];
    let mut new_payments = [0.0f64; MAX_NODES];
    let mut last_residual = f64::INFINITY;
    for round in 0..max_rounds {
        // Compute received for each bank.
        for i in 0..n {
            let mut acc = capital[i]; // start with own capital
            for j in 0..n {
                if total_liab[j] > 0.0 {
                    // Pro-rata share of j's payment to i.
                    acc += payments_out[j] * exposures[j * n + i] / total_liab[j];
                }
            }
            received[i] = acc;
        }
        // New payment = min(total_liab, received).
        let mut delta = 0.0;
        for i in 0..n {
            new_payments[i] = total_liab[i].min(received[i]).max(0.0);
            delta += (new_payments[i] - payments_out[i]).abs();
        }
        for i in 0..n {
            payments_out[i] = new_payments[i];
        }
        last_residual = delta;
        if delta < tolerance {
            return Ok(EconConvergence::converged(round + 1, delta));
        }
    }
    Ok(EconConvergence::stalled(EconStatus::MaxIterations, max_rounds, last_residual))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn eigenvector_centrality_chain() {
        // Chain 1-2-3. Middle node (1) should have highest centrality.
        // Adjacency (undirected): A[0][1]=1, A[1][0]=1, A[1][2]=1, A[2][1]=1
        let adj = [0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mut cent = [0.0f64; 3];
        let conv = eigenvector_centrality_into(&adj, 3, 1000, 1e-9, &mut cent).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert!(cent[1] > cent[0] && cent[1] > cent[2]);
        // Symmetric endpoints should have equal centrality.
        assert!(approx(cent[0], cent[2], 1e-6));
    }

    #[test]
    fn degree_centrality_star() {
        // Star: center (0) connected to 1, 2, 3.
        let adj = [
            0.0, 1.0, 1.0, 1.0,
            1.0, 0.0, 0.0, 0.0,
            1.0, 0.0, 0.0, 0.0,
            1.0, 0.0, 0.0, 0.0,
        ];
        let mut deg = [0.0f64; 4];
        degree_centrality_into(&adj, 4, &mut deg).unwrap();
        assert_eq!(deg[0], 3.0);
        assert_eq!(deg[1], 1.0);
    }

    #[test]
    fn default_cascade_chain() {
        // 3 banks. Bank 0 defaults initially.
        // Bank 1 exposed 100 to bank 0, capital 50 → defaults.
        // Bank 2 exposed 100 to bank 1, capital 50 → defaults.
        let exposures = [
            0.0, 0.0, 0.0,
            100.0, 0.0, 0.0,
            0.0, 100.0, 0.0,
        ];
        let capital = [100.0, 50.0, 50.0];
        let initial = [true, false, false];
        let mut out = [0.0f64; 3];
        let conv = default_cascade_into(&exposures, &capital, 3, &initial, 100, &mut out).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        assert_eq!(out, [1.0, 1.0, 1.0]); // all default via contagion
    }

    #[test]
    fn default_cascade_well_capitalized() {
        // No cascade: banks have enough capital.
        let exposures = [
            0.0, 100.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 100.0, 0.0,
        ];
        let capital = [1000.0, 1000.0, 1000.0];
        let initial = [true, false, false];
        let mut out = [0.0f64; 3];
        default_cascade_into(&exposures, &capital, 3, &initial, 100, &mut out).unwrap();
        assert_eq!(out, [1.0, 0.0, 0.0]); // only bank 0 defaults
    }

    #[test]
    fn interbank_clearing_simple() {
        // 2 banks. Bank 0 owes 100 to bank 1. Bank 1 owes 50 to bank 0.
        // Both have capital 0. In proportional clearing, they settle at 50/50.
        // exposures[i][j] = i owes j.
        let exposures = [
            0.0, 100.0,
            50.0, 0.0,
        ];
        let capital = [0.0, 0.0];
        let mut payments = [0.0f64; 2];
        let conv = interbank_clearing_into(&exposures, &capital, 2, 1000, 1e-9, &mut payments).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // With zero capital, mutual obligations clear at the min that can be supported: 50 each.
        assert!(approx(payments[0], 50.0, 1e-3));
        assert!(approx(payments[1], 50.0, 1e-3));
    }

    #[test]
    fn interbank_clearing_partial_default() {
        // Bank 0 owes 100 to bank 1, has capital 0, receives nothing.
        // Bank 1 owes 0. Bank 0 defaults partially.
        let exposures = [
            0.0, 100.0,
            0.0, 0.0,
        ];
        let capital = [0.0, 0.0];
        let mut payments = [0.0f64; 2];
        let conv = interbank_clearing_into(&exposures, &capital, 2, 100, 1e-9, &mut payments).unwrap();
        assert_eq!(conv.status, EconStatus::Converged);
        // Bank 0 receives 0 from bank 1 (bank 1 owes nothing), so p0 = min(100, 0) = 0.
        assert!(approx(payments[0], 0.0, 1e-6));
        assert!(approx(payments[1], 0.0, 1e-6));
    }

    #[test]
    fn invalid_dimensions_rejected() {
        let adj = [0.0; 4];
        let mut out = [0.0f64; 4];
        let err = eigenvector_centrality_into(&adj, 0, 100, 1e-9, &mut out).unwrap_err();
        assert_eq!(err, NetworkError::InvalidInput);
    }

    #[test]
    fn buffer_too_small() {
        let adj = [0.0; 4];
        let mut out = [0.0f64; 1];
        let err = degree_centrality_into(&adj, 2, &mut out).unwrap_err();
        assert_eq!(err, NetworkError::BufferTooSmall);
    }
}
