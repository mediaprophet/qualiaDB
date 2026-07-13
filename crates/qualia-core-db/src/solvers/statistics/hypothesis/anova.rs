//! One-way ANOVA — the F-test for equality of `k` group means, with a real F-tail
//! p-value from [`fisher_f`](super::super::distributions::fisher_f).

use super::super::descriptive::mean;
use super::super::distributions::fisher_f;

/// One-way ANOVA result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnovaResult {
    pub f_statistic: f64,
    pub p_value: f64,
    pub df_between: f64,
    pub df_within: f64,
    pub ss_between: f64,
    pub ss_within: f64,
    pub ms_between: f64,
    pub ms_within: f64,
}

/// One-way ANOVA across `groups` (≥ 2 groups, each with ≥ 1 observation, and total
/// observations > number of groups so the within-group dof is positive). `None`
/// otherwise.
pub fn one_way_anova(groups: &[&[f64]]) -> Option<AnovaResult> {
    let k = groups.len();
    if k < 2 || groups.iter().any(|g| g.is_empty()) {
        return None;
    }
    let n_total: usize = groups.iter().map(|g| g.len()).sum();
    if n_total <= k {
        return None; // df_within would be ≤ 0
    }

    let grand_mean = groups.iter().flat_map(|g| g.iter()).sum::<f64>() / n_total as f64;

    let mut ss_between = 0.0;
    let mut ss_within = 0.0;
    for g in groups {
        let gm = mean(g)?;
        ss_between += g.len() as f64 * (gm - grand_mean).powi(2);
        for &x in *g {
            ss_within += (x - gm).powi(2);
        }
    }

    let df_between = (k - 1) as f64;
    let df_within = (n_total - k) as f64;
    let ms_between = ss_between / df_between;
    let ms_within = ss_within / df_within;

    let f = if ms_within > 0.0 {
        ms_between / ms_within
    } else {
        // Zero within-group variance: F is +∞ unless between-group variance is also
        // 0 (all values identical), in which case there is no effect.
        if ms_between > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    };
    let p = if f.is_finite() {
        fisher_f::upper_p(f, df_between, df_within)
    } else {
        0.0
    };

    Some(AnovaResult {
        f_statistic: f,
        p_value: p,
        df_between,
        df_within,
        ss_between,
        ss_within,
        ms_between,
        ms_within,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_real_difference() {
        // Three clearly separated groups → large F, tiny p.
        let g1 = [1.0, 2.0, 1.5, 2.5, 2.0];
        let g2 = [5.0, 6.0, 5.5, 6.5, 6.0];
        let g3 = [10.0, 11.0, 10.5, 11.5, 11.0];
        let r = one_way_anova(&[&g1, &g2, &g3]).unwrap();
        assert_eq!(r.df_between, 2.0);
        assert_eq!(r.df_within, 12.0);
        assert!(r.f_statistic > 50.0);
        assert!(r.p_value < 1e-6);
    }

    #[test]
    fn no_difference_is_not_significant() {
        let g1 = [4.0, 5.0, 6.0, 5.0];
        let g2 = [5.0, 6.0, 4.0, 5.0];
        let g3 = [6.0, 4.0, 5.0, 5.0];
        let r = one_way_anova(&[&g1, &g2, &g3]).unwrap();
        assert!(
            r.p_value > 0.2,
            "similar groups should not be significant: p={}",
            r.p_value
        );
    }

    #[test]
    fn matches_known_worked_example() {
        // Classic textbook example: groups (6,8,4,5,3,4),(8,12,9,11,6,8),(13,9,11,8,7,12).
        let a = [6.0, 8.0, 4.0, 5.0, 3.0, 4.0];
        let b = [8.0, 12.0, 9.0, 11.0, 6.0, 8.0];
        let c = [13.0, 9.0, 11.0, 8.0, 7.0, 12.0];
        let r = one_way_anova(&[&a, &b, &c]).unwrap();
        // Known result: F ≈ 9.26, p ≈ 0.0026.
        assert!((r.f_statistic - 9.264).abs() < 0.05, "F={}", r.f_statistic);
        assert!((r.p_value - 0.00256).abs() < 5e-4, "p={}", r.p_value);
    }

    #[test]
    fn guards_degenerate_input() {
        assert!(one_way_anova(&[&[1.0, 2.0][..]]).is_none()); // < 2 groups
        assert!(one_way_anova(&[&[1.0][..], &[2.0][..]]).is_none()); // n_total == k
    }
}
