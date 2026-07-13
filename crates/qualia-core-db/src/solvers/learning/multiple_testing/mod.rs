//! Multiple-testing corrections (ISL ch 13) — adjust a set of p-values for the
//! number of hypotheses tested, controlling either the family-wise error rate
//! (Bonferroni, Holm) or the false discovery rate (Benjamini–Hochberg).
//!
//! Each returns **adjusted p-values in the original input order**; a hypothesis is
//! rejected at level α iff its adjusted p-value ≤ α. Pure rank arithmetic — no
//! external dependency, CPU (not GPU-amenable).

/// Bonferroni: `adjᵢ = min(1, m·pᵢ)` (controls FWER, most conservative).
pub fn bonferroni(p: &[f64]) -> Vec<f64> {
    let m = p.len() as f64;
    p.iter().map(|&pi| (m * pi).min(1.0)).collect()
}

/// Holm step-down (controls FWER, uniformly more powerful than Bonferroni).
pub fn holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    if m == 0 {
        return Vec::new();
    }
    // Sort indices by ascending p.
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| {
        p[a].partial_cmp(&p[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut adj = vec![0.0; m];
    let mut running = 0.0_f64;
    for (rank, &idx) in order.iter().enumerate() {
        // Holm weight for the rank-th smallest: (m - rank).
        let val = ((m - rank) as f64 * p[idx]).min(1.0);
        running = running.max(val); // enforce monotone non-decreasing in rank
        adj[idx] = running;
    }
    adj
}

/// Benjamini–Hochberg (controls the FDR). Adjusted value for the rank-`i` (1-based)
/// p-value is `min_{k≥i} ( m/k · p_(k) )`, clamped to 1.
pub fn benjamini_hochberg(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    if m == 0 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| {
        p[a].partial_cmp(&p[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mf = m as f64;
    let mut adj = vec![0.0; m];
    // Walk from the largest p downward, carrying the running minimum.
    let mut running = f64::INFINITY;
    for rank in (0..m).rev() {
        let idx = order[rank];
        let k = (rank + 1) as f64; // 1-based rank
        let val = (mf / k * p[idx]).min(1.0);
        running = running.min(val);
        adj[idx] = running;
    }
    adj
}

/// Count rejections at level `alpha` from a vector of adjusted p-values.
pub fn n_rejected(adjusted: &[f64], alpha: f64) -> usize {
    adjusted.iter().filter(|&&q| q <= alpha).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonferroni_scales_by_count() {
        let p = [0.01, 0.04, 0.03, 0.005];
        let adj = bonferroni(&p);
        assert!((adj[0] - 0.04).abs() < 1e-12); // 4 * 0.01
        assert!((adj[3] - 0.02).abs() < 1e-12); // 4 * 0.005
                                                // Capped at 1.
        assert_eq!(bonferroni(&[0.5, 0.9])[1], 1.0);
    }

    #[test]
    fn holm_is_less_conservative_than_bonferroni() {
        let p = [0.01, 0.02, 0.03, 0.04, 0.05];
        let b = bonferroni(&p);
        let h = holm(&p);
        // Holm adjusted ≤ Bonferroni adjusted for every hypothesis.
        for i in 0..p.len() {
            assert!(h[i] <= b[i] + 1e-12, "holm[{i}]={} bonf={}", h[i], b[i]);
        }
        // Smallest p: Holm weight m=5 → 0.05, same as Bonferroni for the minimum.
        assert!((h[0] - 0.05).abs() < 1e-12);
    }

    #[test]
    fn bh_matches_known_example() {
        // Classic BH worked example.
        let p = [0.005, 0.009, 0.019, 0.022, 0.051, 0.101, 0.361, 0.957];
        let adj = benjamini_hochberg(&p);
        // Monotone non-decreasing in the sorted order; all in [0,1].
        assert!(adj.iter().all(|&q| (0.0..=1.0).contains(&q)));
        // p=0.005 (rank1): 8/1*0.005=0.04, but the BH step-up running-min carries
        // rank2's 8/2*0.009=0.036 down → adjusted smallest = 0.036.
        assert!((adj[0] - 0.036).abs() < 1e-9, "adj0={}", adj[0]);
        // At FDR 0.05, the first four are discoveries.
        assert_eq!(n_rejected(&adj, 0.05), 4);
    }

    #[test]
    fn bh_is_at_most_one_and_monotone_by_rank() {
        let p = [0.9, 0.1, 0.5, 0.02];
        let adj = benjamini_hochberg(&p);
        // Order by p: 0.02,0.1,0.5,0.9 → adjusted must be non-decreasing in that order.
        let mut order: Vec<usize> = (0..4).collect();
        order.sort_by(|&a, &b| p[a].partial_cmp(&p[b]).unwrap());
        for w in order.windows(2) {
            assert!(adj[w[0]] <= adj[w[1]] + 1e-12);
        }
    }

    #[test]
    fn empty_input() {
        assert!(bonferroni(&[]).is_empty());
        assert!(holm(&[]).is_empty());
        assert!(benjamini_hochberg(&[]).is_empty());
    }
}
