//! Correlation kernels — zero-allocation over caller-owned slices.
//!
//! Canonical home for Pearson / Spearman / Kendall correlation. Specialized
//! libraries call these rather than re-implementing them. Ranking (for Spearman)
//! writes into a caller-owned buffer so this layer allocates nothing.

use super::descriptive::mean;

/// Pearson product-moment correlation. `None` if the lengths differ or n < 2.
/// Returns `Some(0.0)` when either series has zero variance (matches the
/// historical call sites this replaced).
pub fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n < 2 {
        return None;
    }
    let mx = mean(x)?;
    let my = mean(y)?;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    let mut i = 0;
    while i < n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        num += dx * dy;
        dx2 += dx * dx;
        dy2 += dy * dy;
        i += 1;
    }
    let denom = (dx2 * dy2).sqrt();
    if denom == 0.0 {
        return Some(0.0);
    }
    Some(num / denom)
}

/// Rank `values` (1-based, ties averaged) into the caller-owned `ranks_out`.
/// `idx_scratch` is a caller-owned index buffer; both must equal `values.len()`.
/// Returns `None` on a length mismatch. No allocation.
pub fn rank_into(values: &[f64], idx_scratch: &mut [usize], ranks_out: &mut [f64]) -> Option<()> {
    let n = values.len();
    if idx_scratch.len() != n || ranks_out.len() != n {
        return None;
    }
    for i in 0..n {
        idx_scratch[i] = i;
    }
    idx_scratch.sort_unstable_by(|&a, &b| {
        values[a]
            .partial_cmp(&values[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    // Walk groups of equal values. A group occupying sorted positions i..=j
    // (0-based) spans 1-based ranks (i+1)..=(j+1); ties share their average,
    // ((i+1)+(j+1))/2. (The call sites this replaced had a latent bug that did
    // not average ties correctly — the engine kernel is the correct authority.)
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && values[idx_scratch[j + 1]] == values[idx_scratch[i]] {
            j += 1;
        }
        let avg_rank = ((i + 1) as f64 + (j + 1) as f64) / 2.0;
        for k in i..=j {
            ranks_out[idx_scratch[k]] = avg_rank;
        }
        i = j + 1;
    }
    Some(())
}

/// Kendall's correlation (concordant−discordant over total pairs). O(n²), no
/// allocation. `None` if the lengths differ or n < 2; `Some(0.0)` if no pairs differ.
pub fn kendall(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n < 2 {
        return None;
    }
    let mut concordant: i64 = 0;
    let mut discordant: i64 = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            let p = (x[i] - x[j]) * (y[i] - y[j]);
            if p > 0.0 {
                concordant += 1;
            } else if p < 0.0 {
                discordant += 1;
            }
        }
    }
    let total = concordant + discordant;
    if total == 0 {
        return Some(0.0);
    }
    Some((concordant - discordant) as f64 / total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn pearson_perfect_and_anti() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let up = [2.0, 4.0, 6.0, 8.0];
        let down = [8.0, 6.0, 4.0, 2.0];
        assert!((pearson(&x, &up).unwrap() - 1.0).abs() < EPS);
        assert!((pearson(&x, &down).unwrap() + 1.0).abs() < EPS);
    }

    #[test]
    fn pearson_guards_and_zero_variance() {
        assert_eq!(pearson(&[1.0], &[1.0]), None); // n < 2
        assert_eq!(pearson(&[1.0, 2.0], &[1.0]), None); // length mismatch
        assert_eq!(pearson(&[5.0, 5.0, 5.0], &[1.0, 2.0, 3.0]), Some(0.0)); // zero variance
    }

    #[test]
    fn rank_handles_ties() {
        let v = [10.0, 20.0, 20.0, 40.0];
        let mut idx = [0usize; 4];
        let mut ranks = [0.0; 4];
        rank_into(&v, &mut idx, &mut ranks).unwrap();
        // 10→1, the two 20s share (2+3)/2=2.5, 40→4
        assert!((ranks[0] - 1.0).abs() < EPS);
        assert!((ranks[1] - 2.5).abs() < EPS);
        assert!((ranks[2] - 2.5).abs() < EPS);
        assert!((ranks[3] - 4.0).abs() < EPS);
        assert!(rank_into(&v, &mut [0usize; 3], &mut ranks).is_none()); // bad scratch len
    }

    #[test]
    fn spearman_via_rank_then_pearson_is_monotonic_1() {
        // Spearman of a monotone-but-nonlinear relation is 1.0.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [1.0, 4.0, 9.0, 16.0, 25.0];
        let mut ix = [0usize; 5];
        let mut iy = [0usize; 5];
        let mut rx = [0.0; 5];
        let mut ry = [0.0; 5];
        rank_into(&x, &mut ix, &mut rx).unwrap();
        rank_into(&y, &mut iy, &mut ry).unwrap();
        assert!((pearson(&rx, &ry).unwrap() - 1.0).abs() < EPS);
    }

    #[test]
    fn kendall_signs() {
        let x = [1.0, 2.0, 3.0];
        assert!((kendall(&x, &[1.0, 2.0, 3.0]).unwrap() - 1.0).abs() < EPS);
        assert!((kendall(&x, &[3.0, 2.0, 1.0]).unwrap() + 1.0).abs() < EPS);
        assert_eq!(kendall(&[1.0], &[1.0]), None);
    }
}
