//! Nonparametric & multiple-comparison tests (CI-SKM ch 6) — distribution-free
//! tests for paired and multi-group comparisons, the standard tools for comparing
//! classifiers/estimators across datasets. Real p-values from the χ²/F CDFs in
//! [`distributions`](super::super::distributions); within-block ranking reuses the
//! statistics ranker.

use super::super::distributions::{chi_squared, fisher_f};

/// Result of a χ²-based test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonparametricResult {
    pub statistic: f64,
    pub p_value: f64,
    pub dof: f64,
}

/// McNemar's test for two paired binary classifiers: `b` is the count where A is
/// right and B wrong, `c` where A is wrong and B right (the discordant cells of the
/// 2×2 agreement table). Uses the continuity-corrected statistic
/// `(|b−c|−1)²/(b+c)` ~ χ²₁. `None` if `b + c == 0` (no discordance).
pub fn mcnemar(b: u64, c: u64) -> Option<NonparametricResult> {
    let nb = b as f64;
    let nc = c as f64;
    if b + c == 0 {
        return None;
    }
    let diff = (nb - nc).abs();
    let stat = if diff >= 1.0 {
        (diff - 1.0).powi(2) / (nb + nc)
    } else {
        0.0
    };
    Some(NonparametricResult {
        statistic: stat,
        p_value: chi_squared::upper_p(stat, 1.0),
        dof: 1.0,
    })
}

/// Friedman test result, including the Iman-Davenport F-correction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FriedmanResult {
    /// Friedman χ² statistic.
    pub chi_square: f64,
    /// p-value of the χ² statistic (df = k−1).
    pub chi_p_value: f64,
    pub df: f64,
    /// Iman-Davenport F statistic (less conservative than the χ²).
    pub iman_davenport_f: f64,
    /// p-value of the F statistic (df1 = k−1, df2 = (k−1)(n−1)).
    pub f_p_value: f64,
}

/// Friedman test for `k` treatments across `n` blocks (e.g. classifiers × datasets),
/// `data[block]` of length `k` (the measurements, higher = better). Ranks within
/// each block (ties averaged), then tests whether the treatments differ. `None` if
/// fewer than 2 blocks / 2 treatments or ragged input.
pub fn friedman(data: &[&[f64]]) -> Option<FriedmanResult> {
    let n = data.len();
    if n < 2 {
        return None;
    }
    let k = data[0].len();
    if k < 2 || data.iter().any(|b| b.len() != k) {
        return None;
    }

    // Average rank per treatment across blocks (rank ascending so larger value →
    // larger rank; ties share the average rank).
    let mut rank_sum = vec![0.0; k];
    let mut idx = vec![0usize; k];
    let mut ranks = vec![0.0; k];
    for block in data {
        super::super::correlation::rank_into(block, &mut idx, &mut ranks)?;
        for j in 0..k {
            rank_sum[j] += ranks[j];
        }
    }
    let mean_rank: Vec<f64> = rank_sum.iter().map(|&s| s / n as f64).collect();

    let kf = k as f64;
    let nf = n as f64;
    // χ²_F = 12n/(k(k+1)) · Σ (R̄_j − (k+1)/2)².
    let grand = (kf + 1.0) / 2.0;
    let ss: f64 = mean_rank.iter().map(|&r| (r - grand).powi(2)).sum();
    let chi = 12.0 * nf / (kf * (kf + 1.0)) * ss;
    let df = kf - 1.0;
    let chi_p = chi_squared::upper_p(chi, df);

    // Iman-Davenport F.
    let denom = nf * (kf - 1.0) - chi;
    let (f_stat, f_p) = if denom.abs() > 1e-12 && (nf - 1.0) > 0.0 {
        let f = (nf - 1.0) * chi / denom;
        let df1 = kf - 1.0;
        let df2 = (kf - 1.0) * (nf - 1.0);
        let f_clamped = f.max(0.0);
        (f_clamped, fisher_f::upper_p(f_clamped, df1, df2))
    } else {
        (f64::INFINITY, 0.0)
    };

    Some(FriedmanResult {
        chi_square: chi,
        chi_p_value: chi_p,
        df,
        iman_davenport_f: f_stat,
        f_p_value: f_p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcnemar_detects_disagreement() {
        // Classifier A right/B wrong 30 times; A wrong/B right 5 → significant.
        let r = mcnemar(30, 5).unwrap();
        assert_eq!(r.dof, 1.0);
        assert!(r.statistic > 15.0);
        assert!(r.p_value < 0.001);
        // Symmetric discordance → not significant.
        let sym = mcnemar(20, 18).unwrap();
        assert!(sym.p_value > 0.5);
        assert!(mcnemar(0, 0).is_none());
    }

    #[test]
    fn friedman_detects_a_consistent_ordering() {
        // Treatment 2 always best, 0 always worst, across 5 blocks → significant.
        let b1 = [1.0, 2.0, 3.0];
        let b2 = [1.1, 2.2, 3.3];
        let b3 = [0.9, 2.1, 3.1];
        let b4 = [1.0, 2.5, 3.4];
        let b5 = [1.2, 2.0, 3.0];
        let r = friedman(&[&b1, &b2, &b3, &b4, &b5]).unwrap();
        assert_eq!(r.df, 2.0);
        assert!(r.chi_square > 6.0, "chi2 {}", r.chi_square);
        assert!(r.chi_p_value < 0.05);
        assert!(r.f_p_value < 0.05);
    }

    #[test]
    fn friedman_no_difference_is_not_significant() {
        // Random-ish orderings with no consistent winner.
        let b1 = [1.0, 2.0, 3.0];
        let b2 = [3.0, 1.0, 2.0];
        let b3 = [2.0, 3.0, 1.0];
        let r = friedman(&[&b1, &b2, &b3]).unwrap();
        assert!(r.chi_p_value > 0.2, "p {}", r.chi_p_value);
    }

    #[test]
    fn guards() {
        assert!(friedman(&[&[1.0, 2.0][..]]).is_none()); // < 2 blocks
        let ragged: [&[f64]; 2] = [&[1.0, 2.0], &[1.0]];
        assert!(friedman(&ragged).is_none());
    }
}
