//! Native `Neighbor` op-node — spatial proximity queries (`Frnn`/`Knn`/`Range`): the SPH /
//! N-body cutoff and the ANN/kNN candidate-generation primitive (plan §2/§6, P7).
//!
//! # What `Neighbor` returns (and what it does *not*)
//!
//! Per plan §6 `Neighbor` is **broad-phase / candidate-generation only**: for each query it
//! returns a **shortlist of candidate point indices**; the exact metric (and any reranking) is
//! a downstream `MatMul + Reduce`. RT-core acceleration of this op is **optional** and limited
//! to a 3-D point cloud; it is **never** dense linear algebra.
//!
//! # This module is the exact-grid path — the correctness floor *and* the mandatory oracle
//!
//! §6 requires that any RT/approximate proximity be graded against an **exact grid**, and that
//! `dims>3` (without a faithful 3-D projection) **fall back to the grid**. So the exact search
//! here is load-bearing twice over: it is the always-available, any-dimensional **correct
//! implementation** of `Neighbor`, and it is the differential oracle an RT path must match. The
//! RT-core accelerator (an AABB bounding-sphere BLAS + `ray_query` collection over the certified
//! ray path) is an opt-in 3-D speedup layered on top — it does not change these results, only
//! their speed, and only where [`legalize`] admits it.

use crate::wgsl_forge::ir::graph::{NbKind, NeighborEnc};

/// Which execution path [`legalize`] admits for a `Neighbor` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborPath {
    /// The exact grid / brute-force search — correct for any dimensionality. Always available.
    Grid,
    /// RT-core eligible: a 3-D (or faithfully 3-D-projected) point cloud, so the AABB-BLAS +
    /// `ray_query` accelerator *may* be used (it still grades against [`NeighborPath::Grid`]).
    RtEligible,
}

/// Decide the execution path for a `Neighbor` node (plan §6's `legalize`). RT cores are a 3-D
/// fixed-function engine, so RT is admitted **only** for `dims ≤ 3` natively, or `dims ≤ 3`
/// after a declared faithful projection. A higher-dimensional `Native3D` request is **refused
/// RT and routed to the exact grid** — never silently run on a lossy 3-D embedding.
pub fn legalize(dims: u8, enc: NeighborEnc) -> NeighborPath {
    match enc {
        NeighborEnc::Native3D if dims <= 3 => NeighborPath::RtEligible,
        // A declared projection promises a faithful ≤3-D embedding (the method is a later
        // detail); the high-dim source is allowed because the BVH sees only the projection.
        NeighborEnc::Project => NeighborPath::RtEligible,
        // dims>3 with no projection: RT cannot represent it → exact grid.
        NeighborEnc::Native3D => NeighborPath::Grid,
    }
}

/// Squared Euclidean distance between row `i` of `a` and row `j` of `b` (each `d`-dimensional,
/// row-major).
#[inline]
fn sq_dist(a: &[f32], i: usize, b: &[f32], j: usize, d: usize) -> f32 {
    let (ai, bj) = (&a[i * d..i * d + d], &b[j * d..j * d + d]);
    ai.iter().zip(bj).map(|(&x, &y)| (x - y) * (x - y)).sum()
}

/// **Fixed-radius nearest neighbours** (`Frnn`/`Range`): for each of the `q` query points, the
/// indices of all `n` points within Euclidean radius `r` (i.e. `‖p−query‖² ≤ r²`), sorted
/// ascending by index. Exact; the SPH/N-body cutoff and the proximity ground truth.
pub fn frnn_grid_cpu(
    points: &[f32],
    query: &[f32],
    n: usize,
    q: usize,
    d: usize,
    r: f32,
) -> Vec<Vec<u32>> {
    let r2 = r * r;
    (0..q)
        .map(|j| {
            (0..n)
                .filter(|&i| sq_dist(points, i, query, j, d) <= r2)
                .map(|i| i as u32)
                .collect()
        })
        .collect()
}

/// **k nearest neighbours** (`Knn`): for each query point, the indices of the `k` closest of
/// the `n` points, **sorted ascending by distance** (ties broken by index). Exact.
pub fn knn_grid_cpu(
    points: &[f32],
    query: &[f32],
    n: usize,
    q: usize,
    d: usize,
    k: usize,
) -> Vec<Vec<u32>> {
    let k = k.min(n);
    (0..q)
        .map(|j| {
            let mut d2: Vec<(f32, u32)> = (0..n)
                .map(|i| (sq_dist(points, i, query, j, d), i as u32))
                .collect();
            // Sort by (distance, index) — deterministic, exact.
            d2.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            d2.into_iter().take(k).map(|(_, i)| i).collect()
        })
        .collect()
}

/// Unified exact-grid `Neighbor` dispatch on the op kind. `k_or_r` is the radius for
/// `Frnn`/`Range` and (rounded) `k` for `Knn`. Returns a per-query candidate shortlist.
pub fn neighbor_grid_cpu(
    points: &[f32],
    query: &[f32],
    n: usize,
    q: usize,
    d: usize,
    kind: NbKind,
    k_or_r: f32,
) -> Vec<Vec<u32>> {
    match kind {
        NbKind::Frnn | NbKind::Range => frnn_grid_cpu(points, query, n, q, d, k_or_r),
        NbKind::Knn => knn_grid_cpu(points, query, n, q, d, k_or_r.max(0.0).round() as usize),
    }
}

/// Recall of an approximate (e.g. RT) shortlist against the exact grid truth: the fraction of
/// true neighbours that appear in `approx`, averaged over queries. `1.0` == no misses — the
/// metric the **mandatory** RT differential oracle (§6) must meet. Empty truth → perfect.
pub fn recall_vs_grid(truth: &[Vec<u32>], approx: &[Vec<u32>]) -> f32 {
    let mut total = 0usize;
    let mut hit = 0usize;
    for (t, a) in truth.iter().zip(approx.iter()) {
        total += t.len();
        for idx in t {
            if a.contains(idx) {
                hit += 1;
            }
        }
    }
    if total == 0 {
        1.0
    } else {
        hit as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `legalize`: RT only for a ≤3-D native cloud (or a declared projection); a 4-D native
    /// request is routed to the exact grid (never a lossy 3-D embedding).
    #[test]
    fn legalize_gates_rt_on_3d() {
        assert_eq!(legalize(3, NeighborEnc::Native3D), NeighborPath::RtEligible);
        assert_eq!(legalize(2, NeighborEnc::Native3D), NeighborPath::RtEligible);
        assert_eq!(legalize(4, NeighborEnc::Native3D), NeighborPath::Grid);
        assert_eq!(legalize(128, NeighborEnc::Project), NeighborPath::RtEligible);
    }

    /// FRNN on a 1-D line: points at x=0..4, query at x=2, r=1.5 → {1,2,3} (|Δ|≤1.5).
    #[test]
    fn frnn_hand_checked() {
        let points = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let query = [2.0f32];
        let got = frnn_grid_cpu(&points, &query, 5, 1, 1, 1.5);
        assert_eq!(got, vec![vec![1u32, 2, 3]]);
        // r just under 1 → only the exact point.
        let tight = frnn_grid_cpu(&points, &query, 5, 1, 1, 0.99);
        assert_eq!(tight, vec![vec![2u32]]);
    }

    /// kNN on a 2-D grid: query near (0,0); the 3 nearest are the closest cells, distance-sorted.
    #[test]
    fn knn_hand_checked() {
        // points: (0,0),(1,0),(0,1),(2,2)
        let points = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 2.0, 2.0];
        let query = [0.1f32, 0.1];
        let got = knn_grid_cpu(&points, &query, 4, 1, 2, 3);
        // nearest is (0,0) [d²=0.02], then (1,0) and (0,1) tie [d²=0.82] → index order 1,2.
        assert_eq!(got, vec![vec![0u32, 1, 2]]);
    }

    /// `recall_vs_grid`: an approximate shortlist that drops one true neighbour scores < 1.
    #[test]
    fn recall_metric() {
        let truth = vec![vec![1u32, 2, 3]];
        assert_eq!(recall_vs_grid(&truth, &vec![vec![1u32, 2, 3]]), 1.0);
        assert!((recall_vs_grid(&truth, &vec![vec![1u32, 2]]) - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(recall_vs_grid(&[vec![]], &[vec![]]), 1.0);
    }

    /// The unified dispatch routes each kind to the right exact search.
    #[test]
    fn neighbor_dispatch_routes_kinds() {
        let points = [0.0f32, 1.0, 2.0, 3.0, 4.0];
        let query = [2.0f32];
        let frnn = neighbor_grid_cpu(&points, &query, 5, 1, 1, NbKind::Frnn, 1.5);
        assert_eq!(frnn, vec![vec![1u32, 2, 3]]);
        let knn = neighbor_grid_cpu(&points, &query, 5, 1, 1, NbKind::Knn, 2.0);
        assert_eq!(knn, vec![vec![2u32, 1]]); // nearest 2: x=2 (d=0), x=1 or 3 (d=1) → index 1 wins tie
    }
}
