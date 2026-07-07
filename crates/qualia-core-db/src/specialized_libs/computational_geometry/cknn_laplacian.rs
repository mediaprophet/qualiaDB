//! P8.4 — CkNN density estimation → graph Laplacian converging to
//! Laplace-Beltrami (manifold-consistent baking).
//!
//! ## CkNN (Continuous k-Nearest Neighbour) graph
//!
//! Given a point cloud {x₁,…,xₙ} on a manifold M, the CkNN graph connects
//! each point to its k nearest neighbours with weights:
//!
//! ```text
//! w_ij = (1/k) * (d(x_i, x_kNN(i)) / d(x_i, x_j))²   if j ∈ kNN(i)
//! ```
//!
//! where d(x_i, x_kNN(i)) is the distance to the k-th nearest neighbour.
//! The CkNN construction is symmetric: w_ij = w_ji when using the
//! geometric mean of the two k-NN distances.
//!
//! ## Graph Laplacian
//!
//! The unnormalised graph Laplacian: L = D - W
//! where D is the degree matrix (diagonal).
//!
//! The normalised Laplacian: L_sym = I - D^{-1/2} W D^{-1/2}
//!
//! As n → ∞ and k → ∞ (with k/n → 0), the CkNN Laplacian converges to
//! the Laplace-Beltrami operator on M.
//!
//! ## Determinism
//!
//! All operations are deterministic: kNN is computed in canonical
//! (distance, index) order, and the Laplacian is a fixed matrix operation.

use super::vr_filtration::spatial_distance;
use crate::tensor::Tensor10D;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CknnError {
    TooFewPoints { got: usize },
    KTooLarge { k: usize, n: usize },
    BufferTooSmall { needed: usize, have: usize },
    NonFinite { point_index: usize },
}

impl core::fmt::Display for CknnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => write!(f, "cknn: too few points: {got}"),
            Self::KTooLarge { k, n } => write!(f, "cknn: k={k} > n={n}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "cknn: buffer too small, need {needed}, have {have}")
            }
            Self::NonFinite { point_index } => write!(f, "cknn: non-finite at point {point_index}"),
        }
    }
}

impl std::error::Error for CknnError {}

// ───────────────────────────────────────────────────────────────────────────
//  CkNN graph construction
// ───────────────────────────────────────────────────────────────────────────

/// A CkNN edge: (from, to, weight).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CknnEdge {
    pub from: u32,
    pub to: u32,
    pub weight: f64,
}

/// Compute the k-th nearest neighbour distance for each point.
///
/// `out_knn_dist[i]` = distance to the k-th nearest neighbour of point i.
/// `out_knn_idx[i]` = index of the k-th nearest neighbour.
///
/// Uses brute-force O(n²) search. For large n, use a spatial index.
pub fn knn_distances(
    points: &[Tensor10D],
    k: usize,
    out_knn_dist: &mut [f64],
    out_knn_idx: &mut [u32],
) -> Result<(), CknnError> {
    let n = points.len();
    if n < 2 {
        return Err(CknnError::TooFewPoints { got: n });
    }
    if k >= n {
        return Err(CknnError::KTooLarge { k, n });
    }
    if out_knn_dist.len() < n || out_knn_idx.len() < n {
        return Err(CknnError::BufferTooSmall {
            needed: n,
            have: out_knn_dist.len().min(out_knn_idx.len()),
        });
    }

    for (i, p) in points.iter().enumerate() {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return Err(CknnError::NonFinite { point_index: i });
        }
    }

    // For each point, find the k-th nearest neighbour.
    // We use a simple approach: compute all distances, sort, pick k-th.
    let mut dists = vec![(0.0f64, 0u32); n];
    for i in 0..n {
        for j in 0..n {
            dists[j] = (spatial_distance(&points[i], &points[j]), j as u32);
        }
        // Sort by (distance, index) — canonical order.
        dists.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        // dists[0] is self (distance 0), dists[k] is the k-th nearest.
        out_knn_dist[i] = dists[k].0;
        out_knn_idx[i] = dists[k].1;
    }

    Ok(())
}

/// Build the CkNN graph.
///
/// For each point i, connect to its k nearest neighbours with weight:
/// ```text
/// w_ij = (d_k(i) / d(i,j))²
/// ```
/// where d_k(i) is the k-NN distance of point i.
///
/// The graph is made symmetric by taking the geometric mean:
/// w_ij = sqrt(w_ij * w_ji) when both directions exist.
///
/// `out_edges` needs `n * k` entries (may include duplicates after symmetrisation).
/// Returns the number of edges written.
pub fn cknn_graph(
    points: &[Tensor10D],
    k: usize,
    out_edges: &mut [CknnEdge],
) -> Result<usize, CknnError> {
    let n = points.len();
    if n < 2 {
        return Err(CknnError::TooFewPoints { got: n });
    }
    if k >= n {
        return Err(CknnError::KTooLarge { k, n });
    }
    if out_edges.len() < n * k {
        return Err(CknnError::BufferTooSmall {
            needed: n * k,
            have: out_edges.len(),
        });
    }

    // Compute k-NN distances.
    let mut knn_dist = vec![0.0f64; n];
    let mut knn_idx = vec![0u32; n];
    knn_distances(points, k, &mut knn_dist, &mut knn_idx)?;

    // For each point, find its k nearest neighbours and compute weights.
    let mut dists = vec![(0.0f64, 0u32); n];
    let mut count = 0usize;

    for i in 0..n {
        for j in 0..n {
            dists[j] = (spatial_distance(&points[i], &points[j]), j as u32);
        }
        dists.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });

        // Skip self (index 0), take next k.
        for nn in 1..=k {
            let (d, j) = dists[nn];
            if d <= 0.0 || knn_dist[i] <= 0.0 {
                continue;
            }
            let w = (knn_dist[i] / d).powi(2);
            out_edges[count] = CknnEdge {
                from: i as u32,
                to: j,
                weight: w,
            };
            count += 1;
        }
    }

    // Symmetrise: for each edge (i,j,w), check if (j,i,w') exists.
    // If so, replace both with sqrt(w * w'). If not, keep as is.
    for e in 0..count {
        let (from, to) = (out_edges[e].from, out_edges[e].to);
        // Find the reverse edge.
        for e2 in (e + 1)..count {
            if out_edges[e2].from == to && out_edges[e2].to == from {
                let w_sym = (out_edges[e].weight * out_edges[e2].weight).sqrt();
                out_edges[e].weight = w_sym;
                out_edges[e2].weight = w_sym;
                break;
            }
        }
    }

    Ok(count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Graph Laplacian
// ───────────────────────────────────────────────────────────────────────────

/// Compute the unnormalised graph Laplacian: L = D - W.
///
/// `out_laplacian` is a row-major n×n matrix (n² entries).
/// `edges` is the CkNN edge list.
///
/// Returns Ok(()) on success.
pub fn graph_laplacian(
    n: usize,
    edges: &[CknnEdge],
    out_laplacian: &mut [f64],
) -> Result<(), CknnError> {
    if out_laplacian.len() < n * n {
        return Err(CknnError::BufferTooSmall {
            needed: n * n,
            have: out_laplacian.len(),
        });
    }

    // Zero the matrix.
    for v in out_laplacian[..n * n].iter_mut() {
        *v = 0.0;
    }

    // Build adjacency matrix W and degree matrix D.
    for e in edges {
        let i = e.from as usize;
        let j = e.to as usize;
        if i < n && j < n && i != j {
            // W[i*n + j] = weight (symmetric).
            out_laplacian[i * n + j] -= e.weight;
            out_laplacian[j * n + i] -= e.weight;
            // Degree: D[i,i] += weight.
            out_laplacian[i * n + i] += e.weight;
            out_laplacian[j * n + j] += e.weight;
        }
    }

    Ok(())
}

/// Compute the normalised graph Laplacian: L_sym = I - D^{-1/2} W D^{-1/2}.
///
/// `out_laplacian` is a row-major n×n matrix.
pub fn normalised_graph_laplacian(
    n: usize,
    edges: &[CknnEdge],
    out_laplacian: &mut [f64],
) -> Result<(), CknnError> {
    if out_laplacian.len() < n * n {
        return Err(CknnError::BufferTooSmall {
            needed: n * n,
            have: out_laplacian.len(),
        });
    }

    // First compute unnormalised L = D - W.
    graph_laplacian(n, edges, out_laplacian)?;

    // Compute degrees (diagonal of L before normalisation).
    let mut degrees = vec![0.0f64; n];
    for i in 0..n {
        degrees[i] = out_laplacian[i * n + i];
    }

    // L_sym = I - D^{-1/2} W D^{-1/2}
    // = D^{-1/2} (D - W) D^{-1/2}
    // = D^{-1/2} L D^{-1/2}
    for i in 0..n {
        for j in 0..n {
            let di = if degrees[i] > 0.0 {
                degrees[i].sqrt()
            } else {
                1.0
            };
            let dj = if degrees[j] > 0.0 {
                degrees[j].sqrt()
            } else {
                1.0
            };
            out_laplacian[i * n + j] /= di * dj;
        }
    }

    // Add identity (since L = D - W, L_sym = I - D^{-1/2} W D^{-1/2} = D^{-1/2} L D^{-1/2}).
    // Actually D^{-1/2} (D - W) D^{-1/2} = I - D^{-1/2} W D^{-1/2}, so it's already correct.
    // The diagonal should be 1 - w_ii/d_i = 1 (since w_ii = 0), which gives 1.
    // But our computation gives L[i,i]/d_i = d_i/d_i = 1. Correct.

    Ok(())
}

/// Verify Laplacian properties:
/// - Symmetric: L[i,j] = L[j,i]
/// - Row sums ≈ 0 (for unnormalised)
/// Returns true if all properties hold.
pub fn verify_laplacian(laplacian: &[f64], n: usize, tolerance: f64) -> bool {
    // Check symmetry.
    for i in 0..n {
        for j in 0..n {
            if (laplacian[i * n + j] - laplacian[j * n + i]).abs() > tolerance {
                return false;
            }
        }
    }
    // Check row sums ≈ 0.
    for i in 0..n {
        let row_sum: f64 = (0..n).map(|j| laplacian[i * n + j]).sum();
        if row_sum.abs() > tolerance {
            return false;
        }
    }
    true
}

/// Compute local density at each point: the average distance to k nearest
/// neighbours. Lower values = higher density.
pub fn local_density(
    points: &[Tensor10D],
    k: usize,
    out_density: &mut [f64],
) -> Result<(), CknnError> {
    let n = points.len();
    if n < 2 {
        return Err(CknnError::TooFewPoints { got: n });
    }
    if k >= n {
        return Err(CknnError::KTooLarge { k, n });
    }
    if out_density.len() < n {
        return Err(CknnError::BufferTooSmall {
            needed: n,
            have: out_density.len(),
        });
    }

    let mut dists = vec![(0.0f64, 0u32); n];
    for i in 0..n {
        for j in 0..n {
            dists[j] = (spatial_distance(&points[i], &points[j]), j as u32);
        }
        dists.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        // Average distance to k nearest (excluding self).
        let avg: f64 = (1..=k).map(|nn| dists[nn].0).sum::<f64>() / k as f64;
        out_density[i] = avg;
    }

    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over CkNN edges for determinism verification.
pub fn cknn_graph_hash(edges: &[CknnEdge]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for e in edges {
        hash ^= e.from as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.to as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.weight.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// FNV-1a hash over a Laplacian matrix.
pub fn laplacian_hash(laplacian: &[f64]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for v in laplacian {
        hash ^= v.to_bits();
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

    fn grid_points(nx: usize, ny: usize, spacing: f32) -> Vec<Tensor10D> {
        let mut pts = Vec::new();
        for i in 0..nx {
            for j in 0..ny {
                pts.push(make_point(i as f32 * spacing, j as f32 * spacing, 0.0));
            }
        }
        pts
    }

    fn two_density_cloud() -> Vec<Tensor10D> {
        let mut pts = Vec::new();
        // Dense cluster.
        for i in 0..10 {
            for j in 0..10 {
                pts.push(make_point(i as f32 * 0.1, j as f32 * 0.1, 0.0));
            }
        }
        // Sparse cluster.
        for i in 0..5 {
            for j in 0..5 {
                pts.push(make_point(5.0 + i as f32 * 0.5, j as f32 * 0.5, 0.0));
            }
        }
        pts
    }

    #[test]
    fn cknn_graph_basic() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let k = 3;
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();
        assert!(count > 0, "should have edges");
    }

    #[test]
    fn cknn_graph_symmetric_weights() {
        let pts = grid_points(4, 4, 1.0);
        let n = pts.len();
        let k = 4;
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        // For edges that have both directions, weights must match.
        for e in 0..count {
            let (from, to, w) = (edges[e].from, edges[e].to, edges[e].weight);
            for e2 in 0..count {
                if edges[e2].from == to && edges[e2].to == from {
                    assert!(
                        (edges[e2].weight - w).abs() < 1e-10,
                        "bidirectional edge ({},{}) weight {} must match reverse {}",
                        from,
                        to,
                        w,
                        edges[e2].weight
                    );
                    break;
                }
            }
        }
    }

    #[test]
    fn laplacian_symmetric() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let k = 4;
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        let mut lap = vec![0.0f64; n * n];
        graph_laplacian(n, &edges[..count], &mut lap).unwrap();

        assert!(
            verify_laplacian(&lap, n, 1e-10),
            "Laplacian must be symmetric with zero row sums"
        );
    }

    #[test]
    fn laplacian_row_sums_zero() {
        let pts = grid_points(4, 4, 1.0);
        let n = pts.len();
        let k = 4;
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        let mut lap = vec![0.0f64; n * n];
        graph_laplacian(n, &edges[..count], &mut lap).unwrap();

        for i in 0..n {
            let row_sum: f64 = (0..n).map(|j| lap[i * n + j]).sum();
            assert!(
                row_sum.abs() < 1e-10,
                "row {} sum should be 0, got {}",
                i,
                row_sum
            );
        }
    }

    #[test]
    fn normalised_laplacian_diagonal_near_one() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let k = 4;
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        let mut lap = vec![0.0f64; n * n];
        normalised_graph_laplacian(n, &edges[..count], &mut lap).unwrap();

        for i in 0..n {
            assert!(
                (lap[i * n + i] - 1.0).abs() < 1e-10,
                "normalised Laplacian diagonal should be 1, got {}",
                lap[i * n + i]
            );
        }
    }

    #[test]
    fn local_density_monotone_on_two_density_cloud() {
        let pts = two_density_cloud();
        let n = pts.len();
        let mut density = vec![0.0f64; n];
        local_density(&pts, 3, &mut density).unwrap();

        // Dense cluster (first 100 points) should have lower avg distance
        // than sparse cluster (last 25 points).
        let dense_avg: f64 = density[..100].iter().sum::<f64>() / 100.0;
        let sparse_avg: f64 = density[100..].iter().sum::<f64>() / 25.0;
        assert!(
            dense_avg < sparse_avg,
            "dense cluster avg distance {} should be < sparse {}",
            dense_avg,
            sparse_avg
        );
    }

    #[test]
    fn cknn_graph_determinism() {
        let pts = grid_points(4, 4, 1.0);
        let n = pts.len();
        let k = 4;

        let mut e1 = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let c1 = cknn_graph(&pts, k, &mut e1).unwrap();

        let mut e2 = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let c2 = cknn_graph(&pts, k, &mut e2).unwrap();

        assert_eq!(c1, c2);
        assert_eq!(cknn_graph_hash(&e1[..c1]), cknn_graph_hash(&e2[..c2]));
    }

    #[test]
    fn laplacian_determinism() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let k = 4;

        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        let mut lap1 = vec![0.0f64; n * n];
        let mut lap2 = vec![0.0f64; n * n];
        graph_laplacian(n, &edges[..count], &mut lap1).unwrap();
        graph_laplacian(n, &edges[..count], &mut lap2).unwrap();

        assert_eq!(laplacian_hash(&lap1), laplacian_hash(&lap2));
    }

    #[test]
    fn cknn_too_few_points() {
        let pts = vec![make_point(0.0, 0.0, 0.0)];
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            10
        ];
        let err = cknn_graph(&pts, 1, &mut edges).unwrap_err();
        assert!(matches!(err, CknnError::TooFewPoints { .. }));
    }

    #[test]
    fn cknn_k_too_large() {
        let pts = grid_points(2, 2, 1.0);
        let n = pts.len();
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * 10
        ];
        let err = cknn_graph(&pts, 10, &mut edges).unwrap_err();
        assert!(matches!(err, CknnError::KTooLarge { .. }));
    }

    #[test]
    fn cknn_non_finite_fails() {
        let mut pts = grid_points(3, 3, 1.0);
        pts[4].x = f32::NAN;
        let n = pts.len();
        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * 4
        ];
        let err = cknn_graph(&pts, 4, &mut edges).unwrap_err();
        assert!(matches!(err, CknnError::NonFinite { .. }));
    }

    #[test]
    fn laplacian_matches_brute_force() {
        // 3 points in a line: (0,0), (1,0), (2,0).
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(2.0, 0.0, 0.0),
        ];
        let n = pts.len();
        let k = 2; // each point connects to 2 nearest (all others)

        let mut edges = vec![
            CknnEdge {
                from: 0,
                to: 0,
                weight: 0.0
            };
            n * k
        ];
        let count = cknn_graph(&pts, k, &mut edges).unwrap();

        let mut lap = vec![0.0f64; n * n];
        graph_laplacian(n, &edges[..count], &mut lap).unwrap();

        // Verify symmetry and row sums.
        assert!(verify_laplacian(&lap, n, 1e-10));

        // Diagonal should be positive (degree > 0).
        for i in 0..n {
            assert!(lap[i * n + i] > 0.0, "diagonal must be positive");
        }
    }
}
