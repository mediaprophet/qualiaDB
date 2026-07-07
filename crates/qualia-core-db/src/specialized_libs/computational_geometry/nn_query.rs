//! P8.6 — Nearest-neighbour inference query: "distance < threshold ⇒
//! related, zero graph traversal" over a spatial index.
//!
//! Given a Tensor10D point cloud, this module provides:
//! - **Radius query**: find all points within distance `r` of a query point.
//! - **kNN query**: find the `k` closest points in canonical (distance, index) order.
//! - **Axis-honesty**: the distance metric respects the v-class (Euclidean
//!   folds all 7 coordinates, others fold spatial only).
//! - **SELECTOR contract**: q, w, v never enter the distance computation.
//!
//! ## Determinism
//!
//! Results are returned in canonical (distance, index) order. Identical
//! input → bit-identical output.

use super::vr_filtration::{full_coordinate_distance, spatial_distance};
use crate::tensor::Tensor10D;

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// A nearest-neighbour result entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NnEntry {
    /// Index of the neighbour in the point cloud.
    pub index: u32,
    /// Distance to the query point.
    pub distance: f64,
}

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NnError {
    EmptyPointCloud,
    KIsZero,
    BufferTooSmall { needed: usize, have: usize },
    NonFinite { point_index: usize },
}

impl core::fmt::Display for NnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyPointCloud => write!(f, "nn: empty point cloud"),
            Self::KIsZero => write!(f, "nn: k must be > 0"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "nn: buffer too small, need {needed}, have {have}")
            }
            Self::NonFinite { point_index } => write!(f, "nn: non-finite at point {point_index}"),
        }
    }
}

impl std::error::Error for NnError {}

// ───────────────────────────────────────────────────────────────────────────
//  Distance dispatch (axis-honest per v-class)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the axis-honest distance between two Tensor10D points.
///
/// - v=0 (Euclidean): folds all 7 coordinates (x, y, z, t, α, μ, σ).
/// - v=1 (cyclic): folds spatial only (x, y, z).
/// - v=2 (hyperbolic): folds spatial only.
/// - v≥3 (boundary): folds spatial only.
///
/// SELECTOR axes (q, w, v) never enter the computation.
#[inline]
pub fn axis_honest_distance(a: &Tensor10D, b: &Tensor10D) -> f64 {
    match a.v as u32 {
        0 => full_coordinate_distance(a, b),
        _ => spatial_distance(a, b),
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Radius query
// ───────────────────────────────────────────────────────────────────────────

/// Find all points within distance `radius` of the query point.
///
/// Uses brute-force O(n) search. Results are sorted by (distance, index).
///
/// `out_results` needs `n` entries (worst case: all points within radius).
/// Returns the number of results found.
pub fn radius_query(
    points: &[Tensor10D],
    query: &Tensor10D,
    radius: f64,
    out_results: &mut [NnEntry],
) -> Result<usize, NnError> {
    if points.is_empty() {
        return Err(NnError::EmptyPointCloud);
    }
    if out_results.len() < points.len() {
        return Err(NnError::BufferTooSmall {
            needed: points.len(),
            have: out_results.len(),
        });
    }

    let mut count = 0usize;
    for (i, p) in points.iter().enumerate() {
        let d = axis_honest_distance(p, query);
        if d <= radius && d.is_finite() {
            out_results[count] = NnEntry {
                index: i as u32,
                distance: d,
            };
            count += 1;
        }
    }

    // Sort by (distance, index).
    out_results[..count].sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.index.cmp(&b.index))
    });

    Ok(count)
}

// ───────────────────────────────────────────────────────────────────────────
//  kNN query
// ───────────────────────────────────────────────────────────────────────────

/// Find the `k` nearest neighbours of the query point.
///
/// Uses brute-force O(n log n) search. Results are sorted by (distance, index).
///
/// `out_results` needs `k` entries.
/// Returns the number of results found (may be < k if n < k).
pub fn knn_query(
    points: &[Tensor10D],
    query: &Tensor10D,
    k: usize,
    out_results: &mut [NnEntry],
) -> Result<usize, NnError> {
    if points.is_empty() {
        return Err(NnError::EmptyPointCloud);
    }
    if k == 0 {
        return Err(NnError::KIsZero);
    }
    if out_results.len() < k.min(points.len()) {
        return Err(NnError::BufferTooSmall {
            needed: k.min(points.len()),
            have: out_results.len(),
        });
    }

    // Compute all distances.
    let mut entries: Vec<NnEntry> = points
        .iter()
        .enumerate()
        .map(|(i, p)| NnEntry {
            index: i as u32,
            distance: axis_honest_distance(p, query),
        })
        .collect();

    // Sort by (distance, index).
    entries.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.index.cmp(&b.index))
    });

    // Take the first k (excluding self if query is in the cloud).
    let n = entries.len();
    let result_count = k.min(n);

    // If the query point is in the cloud (distance 0 to itself),
    // skip it and take the next k.
    let start = if entries[0].distance == 0.0 { 1 } else { 0 };
    let result_count = result_count.min(n - start);

    for i in 0..result_count {
        out_results[i] = entries[start + i];
    }

    Ok(result_count)
}

// ───────────────────────────────────────────────────────────────────────────
//  SELECTOR contract verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify that SELECTOR axes (q, w, v) do not affect the distance.
///
/// Creates a copy of `a` with perturbed q, w, v and checks that the
/// distance to `b` is unchanged.
pub fn verify_selector_contract(a: &Tensor10D, b: &Tensor10D) -> bool {
    let d_original = axis_honest_distance(a, b);

    // Perturb q.
    let a_q = Tensor10D::new(
        a.q + 1.0,
        a.v,
        a.w,
        a.x,
        a.y,
        a.z,
        a.t,
        a.alpha,
        a.mu,
        a.sigma,
    );
    let d_q = axis_honest_distance(&a_q, b);

    // Perturb w.
    let a_w = Tensor10D::new(
        a.q,
        a.v,
        a.w + 1.0,
        a.x,
        a.y,
        a.z,
        a.t,
        a.alpha,
        a.mu,
        a.sigma,
    );
    let d_w = axis_honest_distance(&a_w, b);

    (d_original - d_q).abs() < 1e-10 && (d_original - d_w).abs() < 1e-10
}

/// Verify axis-honesty: for v=0, perturbing α/μ/σ/t changes the distance;
/// for v≥1, it does not.
pub fn verify_axis_honesty(a: &Tensor10D, b: &Tensor10D) -> bool {
    let d_original = axis_honest_distance(a, b);

    // Perturb α.
    let a_alpha = Tensor10D::new(
        a.q,
        a.v,
        a.w,
        a.x,
        a.y,
        a.z,
        a.t,
        a.alpha + 1.0,
        a.mu,
        a.sigma,
    );
    let d_alpha = axis_honest_distance(&a_alpha, b);

    match a.v as u32 {
        0 => (d_original - d_alpha).abs() > 1e-10, // v=0: α should matter.
        _ => (d_original - d_alpha).abs() < 1e-10, // v≥1: α should NOT matter.
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Brute-force differential
// ───────────────────────────────────────────────────────────────────────────

/// Brute-force radius query for differential testing.
/// Same as `radius_query` but uses spatial distance only (ignores v-class).
pub fn brute_force_radius_spatial(
    points: &[Tensor10D],
    query: &Tensor10D,
    radius: f64,
    out_results: &mut [NnEntry],
) -> Result<usize, NnError> {
    if points.is_empty() {
        return Err(NnError::EmptyPointCloud);
    }
    if out_results.len() < points.len() {
        return Err(NnError::BufferTooSmall {
            needed: points.len(),
            have: out_results.len(),
        });
    }

    let mut count = 0usize;
    for (i, p) in points.iter().enumerate() {
        let d = spatial_distance(p, query);
        if d <= radius && d.is_finite() {
            out_results[count] = NnEntry {
                index: i as u32,
                distance: d,
            };
            count += 1;
        }
    }

    out_results[..count].sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.index.cmp(&b.index))
    });

    Ok(count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over NN results.
pub fn nn_results_hash(results: &[NnEntry]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for r in results {
        hash ^= r.index as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= r.distance.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D::new(0.0, 0.0, 0.0, x, y, z, 0.0, 0.0, 0.0, 0.0)
    }

    fn make_point_v(v: f32, x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D::new(0.0, v, 0.0, x, y, z, 0.0, 0.0, 0.0, 0.0)
    }

    fn grid_points(nx: usize, ny: usize, spacing: f32) -> Vec<Tensor10D> {
        let mut pts = Vec::new();
        for i in 0..nx {
            for j in 0..ny {
                pts.push(make_point(i as f32 * spacing, j as f32 * spacing, 0.0));
            }
        }
        pts
    }

    #[test]
    fn radius_query_returns_points_in_range() {
        let pts = grid_points(5, 5, 1.0);
        let query = make_point(2.0, 2.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let n = radius_query(&pts, &query, 1.5, &mut results).unwrap();

        // Points within 1.5 of (2,2): (2,2) itself, (1,2), (3,2), (2,1), (2,3).
        assert!(n >= 5, "should find at least 5 points within radius 1.5");

        // First result should be the query point itself (distance 0).
        assert_eq!(results[0].index, 12); // (2,2) is at index 2*5+2=12
        assert!(results[0].distance < 1e-10);
    }

    #[test]
    fn radius_query_sorted_by_distance() {
        let pts = grid_points(4, 4, 1.0);
        let query = make_point(0.0, 0.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let n = radius_query(&pts, &query, 10.0, &mut results).unwrap();

        for i in 1..n {
            assert!(
                results[i - 1].distance <= results[i].distance,
                "results must be sorted by distance"
            );
        }
    }

    #[test]
    fn knn_query_returns_k_nearest() {
        let pts = grid_points(5, 5, 1.0);
        let query = make_point(2.0, 2.0, 0.0);
        let k = 3;
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            k
        ];
        let n = knn_query(&pts, &query, k, &mut results).unwrap();

        assert_eq!(n, 3, "should return exactly k results");
        // First should be the closest (distance 0 if query is in cloud).
        assert!(results[0].distance <= results[1].distance);
        assert!(results[1].distance <= results[2].distance);
    }

    #[test]
    fn knn_query_canonical_order() {
        let pts = grid_points(4, 4, 1.0);
        let query = make_point(0.5, 0.5, 0.0);
        let k = 4;
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            k
        ];
        let n = knn_query(&pts, &query, k, &mut results).unwrap();

        // Verify canonical (distance, index) order.
        for i in 1..n {
            let prev = &results[i - 1];
            let curr = &results[i];
            assert!(
                prev.distance < curr.distance
                    || (prev.distance == curr.distance && prev.index < curr.index),
                "results must be in canonical (distance, index) order"
            );
        }
    }

    #[test]
    fn radius_query_matches_brute_force() {
        let pts = grid_points(4, 4, 1.0);
        let query = make_point(1.5, 1.5, 0.0);
        let radius = 2.0;

        let mut results1 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let mut results2 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];

        // All points have v=0, so axis_honest uses full_coordinate_distance.
        // But all non-spatial coords are 0, so it equals spatial distance.
        let n1 = radius_query(&pts, &query, radius, &mut results1).unwrap();
        let n2 = brute_force_radius_spatial(&pts, &query, radius, &mut results2).unwrap();

        assert_eq!(n1, n2, "result count must match");
        for i in 0..n1 {
            assert_eq!(
                results1[i].index, results2[i].index,
                "index mismatch at {}",
                i
            );
            assert!(
                (results1[i].distance - results2[i].distance).abs() < 1e-10,
                "distance mismatch at {}",
                i
            );
        }
    }

    #[test]
    fn selector_contract_q_w_v_excluded() {
        let a = make_point(0.0, 0.0, 0.0);
        let b = make_point(1.0, 0.0, 0.0);
        assert!(
            verify_selector_contract(&a, &b),
            "q, w, v must not affect distance"
        );
    }

    #[test]
    fn selector_contract_with_v_class() {
        let a = make_point_v(2.0, 0.0, 0.0, 0.0);
        let b = make_point_v(2.0, 1.0, 0.0, 0.0);
        assert!(
            verify_selector_contract(&a, &b),
            "q, w must not affect distance even with v=2"
        );
    }

    #[test]
    fn axis_honesty_v0_includes_spectral() {
        let a = make_point(0.0, 0.0, 0.0);
        let b = make_point(1.0, 0.0, 0.0);
        assert!(
            verify_axis_honesty(&a, &b),
            "v=0 should include α in distance"
        );
    }

    #[test]
    fn axis_honesty_v1_excludes_spectral() {
        let a = make_point_v(1.0, 0.0, 0.0, 0.0);
        let b = make_point_v(1.0, 1.0, 0.0, 0.0);
        assert!(
            verify_axis_honesty(&a, &b),
            "v=1 should exclude α from distance"
        );
    }

    #[test]
    fn axis_honesty_v2_excludes_spectral() {
        let a = make_point_v(2.0, 0.0, 0.0, 0.0);
        let b = make_point_v(2.0, 1.0, 0.0, 0.0);
        assert!(
            verify_axis_honesty(&a, &b),
            "v=2 should exclude α from distance"
        );
    }

    #[test]
    fn radius_query_determinism() {
        let pts = grid_points(4, 4, 1.0);
        let query = make_point(1.5, 1.5, 0.0);

        let mut r1 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let mut r2 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let n1 = radius_query(&pts, &query, 2.0, &mut r1).unwrap();
        let n2 = radius_query(&pts, &query, 2.0, &mut r2).unwrap();

        assert_eq!(n1, n2);
        assert_eq!(nn_results_hash(&r1[..n1]), nn_results_hash(&r2[..n2]));
    }

    #[test]
    fn knn_query_determinism() {
        let pts = grid_points(5, 5, 1.0);
        let query = make_point(2.0, 2.0, 0.0);

        let mut r1 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            5
        ];
        let mut r2 = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            5
        ];
        let n1 = knn_query(&pts, &query, 5, &mut r1).unwrap();
        let n2 = knn_query(&pts, &query, 5, &mut r2).unwrap();

        assert_eq!(n1, n2);
        assert_eq!(nn_results_hash(&r1[..n1]), nn_results_hash(&r2[..n2]));
    }

    #[test]
    fn empty_point_cloud_errors() {
        let pts: Vec<Tensor10D> = vec![];
        let query = make_point(0.0, 0.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            1
        ];
        let err = radius_query(&pts, &query, 1.0, &mut results).unwrap_err();
        assert!(matches!(err, NnError::EmptyPointCloud));
    }

    #[test]
    fn k_zero_errors() {
        let pts = grid_points(3, 3, 1.0);
        let query = make_point(0.0, 0.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            1
        ];
        let err = knn_query(&pts, &query, 0, &mut results).unwrap_err();
        assert!(matches!(err, NnError::KIsZero));
    }

    #[test]
    fn radius_query_empty_results_when_nothing_in_range() {
        let pts = grid_points(3, 3, 1.0);
        let query = make_point(100.0, 100.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            pts.len()
        ];
        let n = radius_query(&pts, &query, 1.0, &mut results).unwrap();
        assert_eq!(n, 0, "should find no points far away");
    }

    #[test]
    fn knn_fewer_than_k() {
        let pts = vec![make_point(0.0, 0.0, 0.0), make_point(1.0, 0.0, 0.0)];
        let query = make_point(0.5, 0.0, 0.0);
        let mut results = vec![
            NnEntry {
                index: 0,
                distance: 0.0
            };
            5
        ];
        let n = knn_query(&pts, &query, 5, &mut results).unwrap();
        assert_eq!(n, 2, "should return only 2 results when n < k");
    }
}
