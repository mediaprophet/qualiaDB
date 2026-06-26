//! χ² tests — goodness-of-fit and independence (contingency table), with a real
//! upper-tail p-value from [`chi_squared`](super::super::distributions::chi_squared).

use super::super::distributions::chi_squared;

/// χ² test result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChiSquareResult {
    pub statistic: f64,
    pub p_value: f64,
    pub dof: f64,
}

/// Pearson goodness-of-fit: `Σ (Oᵢ − Eᵢ)² / Eᵢ` with `dof = k − 1`. `None` if the
/// vectors differ in length, are shorter than 2, or any expected count is ≤ 0.
pub fn chi_square_gof(observed: &[f64], expected: &[f64]) -> Option<ChiSquareResult> {
    if observed.len() != expected.len() || observed.len() < 2 {
        return None;
    }
    if expected.iter().any(|&e| e <= 0.0) {
        return None;
    }
    let stat: f64 = observed
        .iter()
        .zip(expected.iter())
        .map(|(&o, &e)| (o - e).powi(2) / e)
        .sum();
    let dof = (observed.len() - 1) as f64;
    Some(ChiSquareResult {
        statistic: stat,
        p_value: chi_squared::upper_p(stat, dof),
        dof,
    })
}

/// χ² test of independence on an `R×C` contingency table of counts. Expected
/// `Eᵢⱼ = rowᵢ·colⱼ / total`, `dof = (R−1)(C−1)`. `None` if the table is not at
/// least 2×2, is ragged, or the grand total is 0.
pub fn chi_square_independence(table: &[&[f64]]) -> Option<ChiSquareResult> {
    let rows = table.len();
    if rows < 2 {
        return None;
    }
    let cols = table[0].len();
    if cols < 2 || table.iter().any(|r| r.len() != cols) {
        return None;
    }
    let row_sums: Vec<f64> = table.iter().map(|r| r.iter().sum()).collect();
    let mut col_sums = vec![0.0; cols];
    for r in table {
        for (j, &v) in r.iter().enumerate() {
            col_sums[j] += v;
        }
    }
    let total: f64 = row_sums.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut stat = 0.0;
    for (i, r) in table.iter().enumerate() {
        for (j, &o) in r.iter().enumerate() {
            let e = row_sums[i] * col_sums[j] / total;
            if e > 0.0 {
                stat += (o - e).powi(2) / e;
            }
        }
    }
    let dof = ((rows - 1) * (cols - 1)) as f64;
    Some(ChiSquareResult {
        statistic: stat,
        p_value: chi_squared::upper_p(stat, dof),
        dof,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gof_fair_die_is_not_rejected() {
        // 60 rolls of a fair die, observed close to expected 10 each.
        let observed = [9.0, 11.0, 10.0, 12.0, 8.0, 10.0];
        let expected = [10.0; 6];
        let r = chi_square_gof(&observed, &expected).unwrap();
        assert_eq!(r.dof, 5.0);
        assert!(r.p_value > 0.5, "fair die should not be rejected: p={}", r.p_value);
    }

    #[test]
    fn gof_loaded_die_is_rejected() {
        let observed = [5.0, 5.0, 5.0, 5.0, 5.0, 35.0]; // sixes way over
        let expected = [10.0; 6];
        let r = chi_square_gof(&observed, &expected).unwrap();
        assert!(r.statistic > 11.07, "statistic {}", r.statistic); // > χ²_{0.95,5}
        assert!(r.p_value < 0.001);
    }

    #[test]
    fn independence_known_example() {
        // 2×2 table with a clear association.
        let table: [&[f64]; 2] = [&[90.0, 60.0], &[30.0, 120.0]];
        let r = chi_square_independence(&table).unwrap();
        assert_eq!(r.dof, 1.0);
        // Strong association → large statistic, tiny p.
        assert!(r.statistic > 30.0, "statistic {}", r.statistic);
        assert!(r.p_value < 1e-6);
    }

    #[test]
    fn independence_of_independent_table() {
        // Rows proportional → no association → small statistic, large p.
        let table: [&[f64]; 2] = [&[10.0, 20.0], &[20.0, 40.0]];
        let r = chi_square_independence(&table).unwrap();
        assert!(r.statistic < 1e-9);
        assert!((r.p_value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn guards_bad_shapes() {
        assert!(chi_square_gof(&[1.0], &[1.0]).is_none());
        assert!(chi_square_gof(&[1.0, 2.0], &[1.0, 0.0]).is_none()); // zero expected
        let ragged: [&[f64]; 2] = [&[1.0, 2.0], &[3.0]];
        assert!(chi_square_independence(&ragged).is_none());
    }
}
