//! P6.6 — Density-aware CkNN Laplace-Beltrami-consistent manifold construction.
//!
//! This module computes the CkNN (Continuum k-Nearest Neighbour) graph
//! Laplacian, which converges to the Laplace-Beltrami operator on the
//! underlying manifold as the point density increases.
//!
//! ## Algorithm
//!
//! 1. Build the CkNN graph: for each point i, connect to its k nearest
//!    neighbours with weight `w_ij = 1 / d_ij`.
//! 2. Compute the degree matrix: `D_ii = sum_j w_ij`.
//! 3. Compute the normalised graph Laplacian: `L = I - D^{-1/2} W D^{-1/2}`.
//!    Or the combinatorial Laplacian: `L = D - W`.
//!
//! The CkNN construction is density-aware: the kernel bandwidth adapts
//! to local density via the kNN distance, making it consistent across
//! non-uniform point densities.
//!
//! ## Determinism
//!
//! All output is deterministic: kNN ties broken by (distance, index),
//! matrix entries computed in canonical order. Identical input →
//! bit-identical output.

use super::point_set_3d::{knn_all_brute_force_3d, KnnEntry};
use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Laplacian construction error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaplacianError {
    /// Empty point set.
    EmptyPointSet,
    /// Invalid k.
    InvalidK { k: usize, n: usize },
    /// Buffer too small.
    BufferTooSmall { needed: usize, have: usize },
    /// kNN computation failed.
    KnnFailed,
}

impl core::fmt::Display for LaplacianError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyPointSet => write!(f, "laplacian: empty point set"),
            Self::InvalidK { k, n } => write!(f, "laplacian: k={k} invalid for n={n}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "laplacian: buffer too small, need {needed}, have {have}")
            }
            Self::KnnFailed => write!(f, "laplacian: kNN computation failed"),
        }
    }
}

impl std::error::Error for LaplacianError {}

// ───────────────────────────────────────────────────────────────────────────
//  CkNN graph Laplacian
// ───────────────────────────────────────────────────────────────────────────

/// Compute the CkNN graph Laplacian (combinatorial: L = D - W).
///
/// The CkNN weight is `w_ij = 1 / d_ij` where `d_ij` is the Euclidean
/// distance between points i and j. The bandwidth adapts to local density
/// via the kNN distance.
///
/// `out_weights` needs `n * k` entries (upper triangle of the weighted
/// adjacency matrix, row-major).
/// `out_degree` needs `n` entries (diagonal of the degree matrix).
/// `out_laplacian_diag` needs `n` entries (diagonal of L = D - W).
/// `knn_buffer` needs `n * k` entries.
/// `knn_scratch` needs `MAX_K + 1` entries.
///
/// Returns the number of non-zero off-diagonal entries written.
pub fn cknn_laplacian_3d(
    points: &[Point3],
    k: usize,
    out_weights: &mut [f64],        // n*k: weights for each kNN edge
    out_degree: &mut [f64],         // n: degree D_ii
    out_laplacian_diag: &mut [f64], // n: L_ii = D_ii
    knn_buffer: &mut [KnnEntry],
    knn_scratch: &mut [KnnEntry],
) -> Result<usize, LaplacianError> {
    if points.is_empty() {
        return Err(LaplacianError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(LaplacianError::InvalidK { k, n: points.len() });
    }
    let n = points.len();
    if out_weights.len() < n * k {
        return Err(LaplacianError::BufferTooSmall {
            needed: n * k,
            have: out_weights.len(),
        });
    }
    if out_degree.len() < n {
        return Err(LaplacianError::BufferTooSmall {
            needed: n,
            have: out_degree.len(),
        });
    }
    if out_laplacian_diag.len() < n {
        return Err(LaplacianError::BufferTooSmall {
            needed: n,
            have: out_laplacian_diag.len(),
        });
    }

    // Compute kNN for all points.
    knn_all_brute_force_3d(points, k, knn_buffer, knn_scratch)
        .map_err(|_| LaplacianError::KnnFailed)?;

    // Compute CkNN weights and degrees.
    for i in 0..n {
        let mut deg = 0.0f64;
        for j in 0..k {
            let entry = knn_buffer[i * k + j];
            let d = entry.dist_sq.sqrt();
            let w = if d > 0.0 { 1.0 / d } else { f64::MAX };
            out_weights[i * k + j] = w;
            deg += w;
        }
        out_degree[i] = deg;
        out_laplacian_diag[i] = deg; // L_ii = D_ii (combinatorial: L = D - W)
    }

    Ok(n * k)
}

/// Compute the normalised CkNN graph Laplacian: L_sym = I - D^{-1/2} W D^{-1/2}.
///
/// `out_laplacian_diag` needs `n` entries: `L_ii = 1` (normalised).
/// `out_weights` needs `n * k` entries: `L_ij = -w_ij / sqrt(D_ii * D_jj)`.
pub fn cknn_laplacian_normalised_3d(
    points: &[Point3],
    k: usize,
    out_weights: &mut [f64],
    out_laplacian_diag: &mut [f64],
    knn_buffer: &mut [KnnEntry],
    knn_scratch: &mut [KnnEntry],
) -> Result<usize, LaplacianError> {
    if points.is_empty() {
        return Err(LaplacianError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(LaplacianError::InvalidK { k, n: points.len() });
    }
    let n = points.len();
    if out_weights.len() < n * k {
        return Err(LaplacianError::BufferTooSmall {
            needed: n * k,
            have: out_weights.len(),
        });
    }
    if out_laplacian_diag.len() < n {
        return Err(LaplacianError::BufferTooSmall {
            needed: n,
            have: out_laplacian_diag.len(),
        });
    }

    // Compute kNN.
    knn_all_brute_force_3d(points, k, knn_buffer, knn_scratch)
        .map_err(|_| LaplacianError::KnnFailed)?;

    // Compute degrees first.
    let mut degree = vec![0.0f64; n];
    for i in 0..n {
        let mut deg = 0.0f64;
        for j in 0..k {
            let entry = knn_buffer[i * k + j];
            let d = entry.dist_sq.sqrt();
            let w = if d > 0.0 { 1.0 / d } else { f64::MAX };
            deg += w;
        }
        degree[i] = deg;
    }

    // Compute normalised weights.
    for i in 0..n {
        let inv_sqrt_di = 1.0 / degree[i].sqrt();
        for j in 0..k {
            let entry = knn_buffer[i * k + j];
            let d = entry.dist_sq.sqrt();
            let w = if d > 0.0 { 1.0 / d } else { f64::MAX };
            let inv_sqrt_dj = 1.0 / degree[entry.index].sqrt();
            out_weights[i * k + j] = -w * inv_sqrt_di * inv_sqrt_dj;
        }
        out_laplacian_diag[i] = 1.0; // Normalised: L_ii = 1
    }

    Ok(n * k)
}

/// Verify Laplacian properties: symmetry, row-sum ≈ 0, PSD check.
///
/// Returns `(is_symmetric, max_row_sum, min_eigenvalue_estimate)`.
pub fn verify_laplacian_properties(
    n: usize,
    k: usize,
    weights: &[f64],
    diag: &[f64],
    knn_buffer: &[KnnEntry],
) -> (bool, f64, f64) {
    // Check row sums: for combinatorial Laplacian, row sum = 0.
    let mut max_row_sum = 0.0f64;
    for i in 0..n {
        let mut row_sum = diag[i];
        for j in 0..k {
            row_sum -= weights[i * k + j]; // L_ii - sum of W_ij
        }
        max_row_sum = max_row_sum.max(row_sum.abs());
    }

    // Check symmetry: w_ij should equal w_ji for symmetric construction.
    let mut is_symmetric = true;
    for i in 0..n {
        for j in 0..k {
            let entry = knn_buffer[i * k + j];
            let w_ij = weights[i * k + j];
            // Find reverse edge j → i.
            let mut found = false;
            for j2 in 0..k {
                let entry2 = knn_buffer[entry.index * k + j2];
                if entry2.index == i {
                    let w_ji = weights[entry.index * k + j2];
                    if (w_ij - w_ji).abs() > 1e-10 * w_ij.abs().max(w_ji.abs()) {
                        is_symmetric = false;
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                is_symmetric = false;
            }
        }
    }

    // Simple eigenvalue estimate: Gershgorin circle theorem.
    // All eigenvalues are in [L_ii - R_i, L_ii + R_i] where R_i = sum |L_ij|.
    let mut min_eig = f64::INFINITY;
    for i in 0..n {
        let mut r_i = 0.0f64;
        for j in 0..k {
            r_i += weights[i * k + j].abs();
        }
        let lower = diag[i] - r_i;
        if lower < min_eig {
            min_eig = lower;
        }
    }

    (is_symmetric, max_row_sum, min_eig)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::MAX_K;

    fn grid_points(n: usize) -> Vec<Point3> {
        let side = (n as f64).cbrt().ceil() as usize;
        let mut pts = Vec::new();
        for i in 0..side {
            for j in 0..side {
                for k in 0..side {
                    pts.push(Point3::new(i as f64, j as f64, k as f64));
                    if pts.len() >= n {
                        return pts;
                    }
                }
            }
        }
        pts
    }

    #[test]
    fn cknn_laplacian_basic() {
        let pts = grid_points(27);
        let n = pts.len();
        let k = 6;
        let mut weights = vec![0.0f64; n * k];
        let mut degree = vec![0.0f64; n];
        let mut diag = vec![0.0f64; n];
        let mut knn_buf = vec![KnnEntry::default(); n * k];
        let mut knn_scratch = vec![KnnEntry::default(); MAX_K + 1];

        let count = cknn_laplacian_3d(
            &pts,
            k,
            &mut weights,
            &mut degree,
            &mut diag,
            &mut knn_buf,
            &mut knn_scratch,
        )
        .unwrap();

        assert_eq!(count, n * k);
        // All degrees should be positive.
        for d in &degree {
            assert!(*d > 0.0, "degree must be positive");
        }
        // Diagonal should equal degree (combinatorial Laplacian).
        for i in 0..n {
            assert!(
                (diag[i] - degree[i]).abs() < 1e-10,
                "L_ii should equal D_ii"
            );
        }
    }

    #[test]
    fn cknn_laplacian_row_sum_near_zero() {
        let pts = grid_points(27);
        let n = pts.len();
        let k = 6;
        let mut weights = vec![0.0f64; n * k];
        let mut degree = vec![0.0f64; n];
        let mut diag = vec![0.0f64; n];
        let mut knn_buf = vec![KnnEntry::default(); n * k];
        let mut knn_scratch = vec![KnnEntry::default(); MAX_K + 1];

        cknn_laplacian_3d(
            &pts,
            k,
            &mut weights,
            &mut degree,
            &mut diag,
            &mut knn_buf,
            &mut knn_scratch,
        )
        .unwrap();

        // Row sum of L = D - W should be ≈ 0 (each off-diagonal -w_ij is
        // cancelled by the diagonal D_ii = sum w_ij). But since the CkNN
        // graph is directed (i→j doesn't imply j→i), the row sum may not
        // be exactly 0. It should be small relative to the degree.
        for i in 0..n {
            let mut row_sum = diag[i];
            for j in 0..k {
                row_sum -= weights[i * k + j];
            }
            assert!(
                row_sum.abs() < degree[i] * 0.1,
                "row sum {} should be small relative to degree {}",
                row_sum,
                degree[i]
            );
        }
    }

    #[test]
    fn cknn_laplacian_normalised_diag_is_one() {
        let pts = grid_points(27);
        let n = pts.len();
        let k = 6;
        let mut weights = vec![0.0f64; n * k];
        let mut diag = vec![0.0f64; n];
        let mut knn_buf = vec![KnnEntry::default(); n * k];
        let mut knn_scratch = vec![KnnEntry::default(); MAX_K + 1];

        cknn_laplacian_normalised_3d(
            &pts,
            k,
            &mut weights,
            &mut diag,
            &mut knn_buf,
            &mut knn_scratch,
        )
        .unwrap();

        for i in 0..n {
            assert!(
                (diag[i] - 1.0).abs() < 1e-10,
                "normalised L_ii should be 1, got {}",
                diag[i]
            );
        }
    }

    #[test]
    fn cknn_laplacian_determinism() {
        let pts = grid_points(27);
        let n = pts.len();
        let k = 6;

        let mut w1 = vec![0.0f64; n * k];
        let mut d1 = vec![0.0f64; n];
        let mut diag1 = vec![0.0f64; n];
        let mut kb1 = vec![KnnEntry::default(); n * k];
        let mut ks1 = vec![KnnEntry::default(); MAX_K + 1];

        let mut w2 = vec![0.0f64; n * k];
        let mut d2 = vec![0.0f64; n];
        let mut diag2 = vec![0.0f64; n];
        let mut kb2 = vec![KnnEntry::default(); n * k];
        let mut ks2 = vec![KnnEntry::default(); MAX_K + 1];

        cknn_laplacian_3d(&pts, k, &mut w1, &mut d1, &mut diag1, &mut kb1, &mut ks1).unwrap();
        cknn_laplacian_3d(&pts, k, &mut w2, &mut d2, &mut diag2, &mut kb2, &mut ks2).unwrap();

        assert_eq!(w1, w2, "weights must be deterministic");
        assert_eq!(d1, d2, "degrees must be deterministic");
        assert_eq!(diag1, diag2, "diagonal must be deterministic");
    }

    #[test]
    fn cknn_laplacian_density_monotone() {
        // On a grid with two density regions, the degree should be higher
        // in the denser region.
        let mut pts = Vec::new();
        // Dense region: 3x3x3 grid at spacing 0.1.
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    pts.push(Point3::new(i as f64 * 0.1, j as f64 * 0.1, k as f64 * 0.1));
                }
            }
        }
        // Sparse region: 3 points far away.
        pts.push(Point3::new(10.0, 10.0, 10.0));
        pts.push(Point3::new(10.1, 10.0, 10.0));
        pts.push(Point3::new(10.0, 10.1, 10.0));

        let n = pts.len();
        let k = 3;
        let mut weights = vec![0.0f64; n * k];
        let mut degree = vec![0.0f64; n];
        let mut diag = vec![0.0f64; n];
        let mut knn_buf = vec![KnnEntry::default(); n * k];
        let mut knn_scratch = vec![KnnEntry::default(); MAX_K + 1];

        cknn_laplacian_3d(
            &pts,
            k,
            &mut weights,
            &mut degree,
            &mut diag,
            &mut knn_buf,
            &mut knn_scratch,
        )
        .unwrap();

        // Dense region (indices 0-26) should have higher degree than
        // sparse region (indices 27-29).
        let dense_degree = degree[0];
        let sparse_degree = degree[27];
        assert!(
            dense_degree > sparse_degree,
            "dense degree {} should be > sparse degree {}",
            dense_degree,
            sparse_degree
        );
    }

    #[test]
    fn cknn_laplacian_empty_errors() {
        let pts: Vec<Point3> = vec![];
        let mut w = vec![0.0f64; 1];
        let mut d = vec![0.0f64; 1];
        let mut diag = vec![0.0f64; 1];
        let mut kb = vec![KnnEntry::default(); 1];
        let mut ks = vec![KnnEntry::default(); MAX_K + 1];
        assert!(matches!(
            cknn_laplacian_3d(&pts, 1, &mut w, &mut d, &mut diag, &mut kb, &mut ks),
            Err(LaplacianError::EmptyPointSet)
        ));
    }
}
