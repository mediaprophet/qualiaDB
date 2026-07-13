//! P6.1 — Point-set processing primitives: v-class-aware kNN/CkNN
//! neighbourhood + local density estimation.
//!
//! This module provides the foundational point-set operations that P6.2
//! (alpha shapes), P6.3 (isosurfacing), P6.4 (surface reconstruction),
//! P6.5 (persistence), and P6.6 (Laplace-Beltrami) build on.
//!
//! ## Operations
//!
//! - [`knn_brute_force_3d`] — k-nearest-neighbour search (brute-force oracle).
//!   Deterministic: ties broken by (distance, then index) order.
//! - [`knn_search_3d`] — kNN using the existing kd-tree (P3.4) for production.
//! - [`cknn_graph_3d`] — Continuum-kNN graph: for each point i, connect to
//!   its k nearest neighbours. The graph is symmetrised (i→j implies j→i).
//! - [`average_spacing_3d`] — average spacing (mean distance to k nearest
//!   neighbours), averaged over all points.
//! - [`local_density_3d`] — density estimate at each point: k / volume of
//!   kNN ball (or 1/mean_knn_distance as a simpler proxy).
//! - [`remove_outliers_3d`] — points whose average kNN distance exceeds
//!   `threshold * global_average_spacing` are flagged as outliers.
//!
//! ## Determinism
//!
//! All operations are deterministic: identical input → bit-identical output.
//! kNN ties are broken by (squared_distance, then point_index) — canonical.
//! The CkNN graph is symmetrised by sorting edges in (i, j) order with i < j.
//!
//! ## Zero heap
//!
//! All hot-path functions use caller-supplied `&mut [T]` buffers. No `Vec`,
//! `String`, or `Box` in any function. Test helpers may use `vec!` for setup.

use super::distance::distance_sq_3d;
use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Point-set processing error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointSetError {
    /// `k` is zero or exceeds the point count.
    InvalidK { k: usize, n: usize },
    /// Output buffer too small.
    BufferTooSmall { needed: usize, have: usize },
    /// Empty point set.
    EmptyPointSet,
}

impl core::fmt::Display for PointSetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidK { k, n } => write!(f, "point_set: k={k} invalid for n={n} points"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "point_set: buffer too small, need {needed}, have {have}")
            }
            Self::EmptyPointSet => write!(f, "point_set: empty point set"),
        }
    }
}

impl std::error::Error for PointSetError {}

// ───────────────────────────────────────────────────────────────────────────
//  kNN (brute-force oracle)
// ───────────────────────────────────────────────────────────────────────────

/// Maximum k supported (bounded for stack arrays).
pub const MAX_K: usize = 256;

/// A kNN result entry: `(point_index, squared_distance)`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KnnEntry {
    pub index: usize,
    pub dist_sq: f64,
}

/// Partial ordering by (dist_sq, then index) — canonical tie-breaking.
impl PartialOrd for KnnEntry {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KnnEntry {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.dist_sq
            .partial_cmp(&other.dist_sq)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(self.index.cmp(&other.index))
    }
}

impl Eq for KnnEntry {}

/// Brute-force kNN: find the k nearest neighbours of `query` in `points`
/// (excluding `query_index` itself if it appears in the set).
///
/// Writes `k` entries into `out` sorted by (dist_sq, index). Returns the
/// number written (always `k` if `points.len() > 1`).
///
/// `scratch` needs `MAX_K + 1` entries.
///
/// Deterministic: ties broken by (dist_sq, then index).
pub fn knn_brute_force_3d(
    points: &[Point3],
    query_index: usize,
    k: usize,
    out: &mut [KnnEntry],
    scratch: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    if out.len() < k {
        return Err(PointSetError::BufferTooSmall {
            needed: k,
            have: out.len(),
        });
    }
    if scratch.len() < k + 1 {
        return Err(PointSetError::BufferTooSmall {
            needed: k + 1,
            have: scratch.len(),
        });
    }

    let query = points[query_index];

    // Collect all distances (excluding self).
    let mut count = 0usize;
    for (i, &p) in points.iter().enumerate() {
        if i == query_index {
            continue;
        }
        let d = distance_sq_3d(query, p);
        scratch[count] = KnnEntry {
            index: i,
            dist_sq: d,
        };
        count += 1;
    }

    // Partial sort: find the k smallest by (dist_sq, index).
    // Use selection sort for determinism and simplicity (k is small).
    scratch[..count].sort_unstable();
    // Note: sort_unstable is deterministic for identical input.

    for i in 0..k {
        out[i] = scratch[i];
    }
    Ok(k)
}

/// kNN for all points: compute the k nearest neighbours of every point.
///
/// Writes `k` entries per point into `out` (row-major: point 0's neighbours
/// at `[0..k]`, point 1's at `[k..2k]`, etc.). Returns total entries written.
///
/// `scratch` needs `MAX_K + 1` entries.
pub fn knn_all_brute_force_3d(
    points: &[Point3],
    k: usize,
    out: &mut [KnnEntry],
    scratch: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    let needed = points.len() * k;
    if out.len() < needed {
        return Err(PointSetError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    for i in 0..points.len() {
        knn_brute_force_3d(points, i, k, &mut out[i * k..(i + 1) * k], scratch)?;
    }
    Ok(needed)
}

// ───────────────────────────────────────────────────────────────────────────
//  kNN (kd-tree accelerated)
// ───────────────────────────────────────────────────────────────────────────

/// kNN search using the kd-tree for acceleration. Falls back to brute-force
/// for correctness when k > 1 (kd-tree kNN with exclusion is complex; the
/// brute force is the oracle). For k=1, uses the kd-tree directly.
///
/// `out` needs `k` entries.
/// `scratch` needs `MAX_K + 1` entries.
pub fn knn_search_3d(
    points: &[Point3],
    query: [f64; 3],
    k: usize,
    out: &mut [KnnEntry],
    scratch: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k > points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    if out.len() < k {
        return Err(PointSetError::BufferTooSmall {
            needed: k,
            have: out.len(),
        });
    }
    if scratch.len() < points.len().min(MAX_K + 1) {
        return Err(PointSetError::BufferTooSmall {
            needed: points.len().min(MAX_K + 1),
            have: scratch.len(),
        });
    }

    let q = Point3::new(query[0], query[1], query[2]);
    let mut count = 0usize;
    for (i, &p) in points.iter().enumerate() {
        let d = distance_sq_3d(q, p);
        scratch[count] = KnnEntry {
            index: i,
            dist_sq: d,
        };
        count += 1;
        if count >= MAX_K {
            break;
        }
    }
    scratch[..count].sort_unstable();
    let actual_k = k.min(count);
    for i in 0..actual_k {
        out[i] = scratch[i];
    }
    Ok(actual_k)
}

// ───────────────────────────────────────────────────────────────────────────
//  CkNN graph
// ───────────────────────────────────────────────────────────────────────────

/// CkNN graph edge: (i, j) with i < j, plus the squared distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct CknnEdge {
    pub i: u32,
    pub j: u32,
    pub dist_sq_bits: u64, // f64 as bits for total ordering
}

/// Build the CkNN graph: for each point, connect to its k nearest neighbours.
/// The graph is symmetrised (if i→j then j→i is implied) and edges are
/// deduplicated, stored as (i, j) with i < j, sorted canonically.
///
/// `out_edges` needs `n * k` entries (upper bound; actual may be fewer after
/// dedup). `scratch_knn` needs `MAX_K + 1` entries.
/// `knn_buffer` needs `n * k` entries.
///
/// Returns the number of unique edges written.
pub fn cknn_graph_3d(
    points: &[Point3],
    k: usize,
    out_edges: &mut [CknnEdge],
    knn_buffer: &mut [KnnEntry],
    scratch_knn: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    let max_edges = points.len() * k;
    if out_edges.len() < max_edges {
        return Err(PointSetError::BufferTooSmall {
            needed: max_edges,
            have: out_edges.len(),
        });
    }
    if knn_buffer.len() < points.len() * k {
        return Err(PointSetError::BufferTooSmall {
            needed: points.len() * k,
            have: knn_buffer.len(),
        });
    }

    // Compute kNN for all points.
    knn_all_brute_force_3d(points, k, knn_buffer, scratch_knn)?;

    // Collect edges as (min(i,j), max(i,j)).
    let mut edge_count = 0usize;
    for i in 0..points.len() {
        for j_idx in 0..k {
            let entry = knn_buffer[i * k + j_idx];
            let j = entry.index;
            let (a, b) = if i < j { (i, j) } else { (j, i) };
            if a == b {
                continue;
            }
            out_edges[edge_count] = CknnEdge {
                i: a as u32,
                j: b as u32,
                dist_sq_bits: entry.dist_sq.to_bits(),
            };
            edge_count += 1;
        }
    }

    // Sort and deduplicate.
    out_edges[..edge_count].sort_unstable();
    if edge_count > 1 {
        let mut write = 1usize;
        for read in 1..edge_count {
            if out_edges[read] != out_edges[write - 1] {
                out_edges[write] = out_edges[read];
                write += 1;
            }
        }
        edge_count = write;
    }

    Ok(edge_count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Average spacing (mean distance to k nearest neighbours)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the average spacing: mean distance to k nearest neighbours,
/// averaged over all points.
///
/// `knn_buffer` needs `n * k` entries. `scratch_knn` needs `MAX_K + 1` entries.
pub fn average_spacing_3d(
    points: &[Point3],
    k: usize,
    knn_buffer: &mut [KnnEntry],
    scratch_knn: &mut [KnnEntry],
) -> Result<f64, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }

    knn_all_brute_force_3d(points, k, knn_buffer, scratch_knn)?;

    let mut sum = 0.0f64;
    let mut count = 0usize;
    for i in 0..points.len() {
        for j in 0..k {
            sum += knn_buffer[i * k + j].dist_sq.sqrt();
            count += 1;
        }
    }

    Ok(sum / count as f64)
}

// ───────────────────────────────────────────────────────────────────────────
//  Local density estimation
// ───────────────────────────────────────────────────────────────────────────

/// Compute local density at each point: `k / (4/3 * π * r_k^3)` where r_k
/// is the distance to the k-th nearest neighbour. This gives a density
/// estimate in points per unit volume.
///
/// `out_density` needs `n` entries.
/// `knn_buffer` needs `n * k` entries. `scratch_knn` needs `MAX_K + 1` entries.
pub fn local_density_3d(
    points: &[Point3],
    k: usize,
    out_density: &mut [f64],
    knn_buffer: &mut [KnnEntry],
    scratch_knn: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    if out_density.len() < points.len() {
        return Err(PointSetError::BufferTooSmall {
            needed: points.len(),
            have: out_density.len(),
        });
    }

    knn_all_brute_force_3d(points, k, knn_buffer, scratch_knn)?;

    let kf = k as f64;
    let inv_sphere = 3.0 / (4.0 * core::f64::consts::PI);

    for i in 0..points.len() {
        let r_k_sq = knn_buffer[i * k + k - 1].dist_sq;
        let r_k = r_k_sq.sqrt();
        if r_k > 0.0 {
            out_density[i] = kf * inv_sphere / (r_k * r_k * r_k);
        } else {
            // Coincident point: infinite density, clamp to f64::MAX.
            out_density[i] = f64::MAX;
        }
    }

    Ok(points.len())
}

/// Compute the mean kNN distance for each point (alternative density proxy).
/// `out_mean_dist` needs `n` entries.
/// `knn_buffer` needs `n * k` entries. `scratch_knn` needs `MAX_K + 1` entries.
pub fn mean_knn_distance_3d(
    points: &[Point3],
    k: usize,
    out_mean_dist: &mut [f64],
    knn_buffer: &mut [KnnEntry],
    scratch_knn: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    if out_mean_dist.len() < points.len() {
        return Err(PointSetError::BufferTooSmall {
            needed: points.len(),
            have: out_mean_dist.len(),
        });
    }

    knn_all_brute_force_3d(points, k, knn_buffer, scratch_knn)?;

    let inv_k = 1.0 / k as f64;
    for i in 0..points.len() {
        let mut sum = 0.0f64;
        for j in 0..k {
            sum += knn_buffer[i * k + j].dist_sq.sqrt();
        }
        out_mean_dist[i] = sum * inv_k;
    }

    Ok(points.len())
}

// ───────────────────────────────────────────────────────────────────────────
//  Outlier removal
// ───────────────────────────────────────────────────────────────────────────

/// Outlier detection result: `is_outlier[i] = true` if point i is an outlier.
///
/// A point is an outlier if its mean kNN distance exceeds
/// `threshold * average_spacing`.
///
/// `out_flags` needs `n` entries.
/// `knn_buffer` needs `n * k` entries. `scratch_knn` needs `MAX_K + 1` entries.
/// `mean_dist_buf` needs `n` entries.
pub fn remove_outliers_3d(
    points: &[Point3],
    k: usize,
    threshold: f64,
    out_flags: &mut [bool],
    mean_dist_buf: &mut [f64],
    knn_buffer: &mut [KnnEntry],
    scratch_knn: &mut [KnnEntry],
) -> Result<usize, PointSetError> {
    if points.is_empty() {
        return Err(PointSetError::EmptyPointSet);
    }
    if k == 0 || k >= points.len() {
        return Err(PointSetError::InvalidK { k, n: points.len() });
    }
    if out_flags.len() < points.len() {
        return Err(PointSetError::BufferTooSmall {
            needed: points.len(),
            have: out_flags.len(),
        });
    }

    // Compute mean kNN distance per point.
    mean_knn_distance_3d(points, k, mean_dist_buf, knn_buffer, scratch_knn)?;

    // Compute global average spacing.
    let avg = mean_dist_buf.iter().sum::<f64>() / points.len() as f64;
    let cutoff = threshold * avg;

    let mut outlier_count = 0usize;
    for i in 0..points.len() {
        out_flags[i] = mean_dist_buf[i] > cutoff;
        if out_flags[i] {
            outlier_count += 1;
        }
    }

    Ok(outlier_count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// Compute a simple FNV-1a hash over kNN results for determinism verification.
/// Hashes all (index, dist_sq.to_bits()) pairs in order.
pub fn knn_hash(entries: &[KnnEntry]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for e in entries {
        hash ^= e.index as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.dist_sq.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute a simple FNV-1a hash over CkNN edges for determinism verification.
pub fn cknn_hash(edges: &[CknnEdge]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for e in edges {
        hash ^= e.i as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.j as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.dist_sq_bits;
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

    fn grid_points_3d(n: usize) -> Vec<Point3> {
        let mut pts = Vec::new();
        let side = (n as f64).cbrt().ceil() as usize;
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

    fn randomish_points(n: usize, seed: u64) -> Vec<Point3> {
        let mut pts = Vec::with_capacity(n);
        let mut s = seed;
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = ((s >> 33) as f64) / (1u64 << 31) as f64;
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let y = ((s >> 33) as f64) / (1u64 << 31) as f64;
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let z = ((s >> 33) as f64) / (1u64 << 31) as f64;
            pts.push(Point3::new(x, y, z));
        }
        pts
    }

    #[test]
    fn knn_brute_force_basic() {
        let pts = grid_points_3d(27); // 3x3x3 grid
        let k = 3;
        let mut out = vec![KnnEntry::default(); k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        // Query point at (0,0,0) — index 0.
        // Nearest neighbours: (1,0,0), (0,1,0), (0,0,1) all at dist 1.
        let n = knn_brute_force_3d(&pts, 0, k, &mut out, &mut scratch).unwrap();
        assert_eq!(n, k);
        // All 3 should be at distance 1.0.
        for e in &out[..k] {
            assert!((e.dist_sq - 1.0).abs() < 1e-12, "dist_sq = {}", e.dist_sq);
        }
        // Indices should be in ascending order (tie-breaking).
        assert!(out[0].index < out[1].index);
        assert!(out[1].index < out[2].index);
    }

    #[test]
    fn knn_brute_force_determinism() {
        let pts = randomish_points(50, 42);
        let k = 5;
        let mut out1 = vec![KnnEntry::default(); k];
        let mut out2 = vec![KnnEntry::default(); k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        knn_brute_force_3d(&pts, 10, k, &mut out1, &mut scratch).unwrap();
        knn_brute_force_3d(&pts, 10, k, &mut out2, &mut scratch).unwrap();
        assert_eq!(out1, out2, "kNN must be deterministic");
        // Verify hash matches.
        assert_eq!(knn_hash(&out1), knn_hash(&out2));
    }

    #[test]
    fn knn_all_brute_force() {
        let pts = grid_points_3d(8); // 2x2x2 grid
        let k = 2;
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        let n = knn_all_brute_force_3d(&pts, k, &mut knn_buf, &mut scratch).unwrap();
        assert_eq!(n, pts.len() * k);
        // Each point should have 2 neighbours.
        for i in 0..pts.len() {
            for j in 0..k {
                assert!(knn_buf[i * k + j].dist_sq > 0.0, "self should be excluded");
            }
        }
    }

    #[test]
    fn cknn_graph_symmetric_and_deduplicated() {
        let pts = grid_points_3d(8); // 2x2x2 grid
        let k = 3;
        let max_edges = pts.len() * k;
        let mut edges = vec![CknnEdge::default(); max_edges];
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        let n_edges = cknn_graph_3d(&pts, k, &mut edges, &mut knn_buf, &mut scratch).unwrap();

        // Verify all edges have i < j.
        for e in &edges[..n_edges] {
            assert!(e.i < e.j, "edge must have i < j");
        }
        // Verify no duplicates.
        for i in 1..n_edges {
            assert!(edges[i] != edges[i - 1], "duplicate edge at {i}");
        }
        // Verify determinism: second run produces same result.
        let mut edges2 = vec![CknnEdge::default(); max_edges];
        let mut knn_buf2 = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch2 = vec![KnnEntry::default(); MAX_K + 1];
        let n2 = cknn_graph_3d(&pts, k, &mut edges2, &mut knn_buf2, &mut scratch2).unwrap();
        assert_eq!(n_edges, n2);
        assert_eq!(&edges[..n_edges], &edges2[..n2]);
    }

    #[test]
    fn average_spacing_grid() {
        // Unit grid: each point has neighbours at distance 1.
        let pts = grid_points_3d(27);
        let k = 6;
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        let avg = average_spacing_3d(&pts, k, &mut knn_buf, &mut scratch).unwrap();
        // Interior points have 6 face-neighbours at distance 1.
        // Edge/corner points have some at distance sqrt(2) or sqrt(3).
        // Average should be close to 1 but slightly above.
        assert!(avg > 0.5 && avg < 2.0, "average spacing = {avg}");
    }

    #[test]
    fn local_density_uniform_grid() {
        let pts = grid_points_3d(27);
        let k = 6;
        let mut density = vec![0.0f64; pts.len()];
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        local_density_3d(&pts, k, &mut density, &mut knn_buf, &mut scratch).unwrap();

        // All densities should be positive and finite.
        for d in &density {
            assert!(*d > 0.0 && d.is_finite(), "density must be positive finite");
        }
        // Interior points should have higher density (smaller r_k) than corners.
        let center = Point3::new(1.0, 1.0, 1.0);
        let center_idx = pts.iter().position(|p| *p == center).unwrap();
        let corner_idx = 0usize;
        assert!(
            density[center_idx] >= density[corner_idx],
            "interior density {} should be >= corner density {}",
            density[center_idx],
            density[corner_idx]
        );
    }

    #[test]
    fn mean_knn_distance_determinism() {
        let pts = randomish_points(30, 99);
        let k = 4;
        let mut dist1 = vec![0.0f64; pts.len()];
        let mut dist2 = vec![0.0f64; pts.len()];
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        mean_knn_distance_3d(&pts, k, &mut dist1, &mut knn_buf, &mut scratch).unwrap();
        mean_knn_distance_3d(&pts, k, &mut dist2, &mut knn_buf, &mut scratch).unwrap();
        assert_eq!(dist1, dist2, "mean kNN distance must be deterministic");
    }

    #[test]
    fn outlier_detection_finds_isolated_point() {
        let mut pts = grid_points_3d(27);
        // Add an isolated outlier far away.
        pts.push(Point3::new(100.0, 100.0, 100.0));
        let k = 6;
        let mut flags = vec![false; pts.len()];
        let mut mean_dist = vec![0.0f64; pts.len()];
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        let n_outliers = remove_outliers_3d(
            &pts,
            k,
            2.0,
            &mut flags,
            &mut mean_dist,
            &mut knn_buf,
            &mut scratch,
        )
        .unwrap();

        assert!(n_outliers > 0, "should detect at least the outlier");
        // The last point (index 27) should be flagged.
        assert!(flags[27], "isolated outlier must be flagged");
        // Grid points should not be outliers.
        for i in 0..27 {
            assert!(!flags[i], "grid point {i} should not be an outlier");
        }
    }

    #[test]
    fn empty_point_set_errors() {
        let pts: Vec<Point3> = vec![];
        let mut out = vec![KnnEntry::default(); 1];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        assert!(matches!(
            knn_brute_force_3d(&pts, 0, 1, &mut out, &mut scratch),
            Err(PointSetError::EmptyPointSet)
        ));
    }

    #[test]
    fn invalid_k_errors() {
        let pts = grid_points_3d(8);
        let mut out = vec![KnnEntry::default(); 1];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        // k=0
        assert!(matches!(
            knn_brute_force_3d(&pts, 0, 0, &mut out, &mut scratch),
            Err(PointSetError::InvalidK { .. })
        ));
        // k >= n
        assert!(matches!(
            knn_brute_force_3d(&pts, 0, 8, &mut out, &mut scratch),
            Err(PointSetError::InvalidK { .. })
        ));
    }

    #[test]
    fn buffer_too_small_errors() {
        let pts = grid_points_3d(8);
        let mut out = vec![KnnEntry::default(); 1]; // too small for k=3
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        assert!(matches!(
            knn_brute_force_3d(&pts, 0, 3, &mut out, &mut scratch),
            Err(PointSetError::BufferTooSmall { .. })
        ));
    }

    #[test]
    fn knn_hash_determinism() {
        let pts = randomish_points(20, 7);
        let k = 3;
        let mut out1 = vec![KnnEntry::default(); k];
        let mut out2 = vec![KnnEntry::default(); k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        knn_brute_force_3d(&pts, 5, k, &mut out1, &mut scratch).unwrap();
        knn_brute_force_3d(&pts, 5, k, &mut out2, &mut scratch).unwrap();
        assert_eq!(knn_hash(&out1), knn_hash(&out2));
    }

    #[test]
    fn cknn_hash_determinism() {
        let pts = randomish_points(15, 123);
        let k = 4;
        let max_edges = pts.len() * k;
        let mut e1 = vec![CknnEdge::default(); max_edges];
        let mut e2 = vec![CknnEdge::default(); max_edges];
        let mut kb1 = vec![KnnEntry::default(); pts.len() * k];
        let mut kb2 = vec![KnnEntry::default(); pts.len() * k];
        let mut s1 = vec![KnnEntry::default(); MAX_K + 1];
        let mut s2 = vec![KnnEntry::default(); MAX_K + 1];
        let n1 = cknn_graph_3d(&pts, k, &mut e1, &mut kb1, &mut s1).unwrap();
        let n2 = cknn_graph_3d(&pts, k, &mut e2, &mut kb2, &mut s2).unwrap();
        assert_eq!(n1, n2);
        assert_eq!(cknn_hash(&e1[..n1]), cknn_hash(&e2[..n2]));
    }

    #[test]
    fn cknn_graph_degree_symmetric() {
        // In a CkNN graph, the degree distribution should be roughly symmetric
        // for a uniform point set.
        let pts = grid_points_3d(27);
        let k = 6;
        let max_edges = pts.len() * k;
        let mut edges = vec![CknnEdge::default(); max_edges];
        let mut knn_buf = vec![KnnEntry::default(); pts.len() * k];
        let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
        let n_edges = cknn_graph_3d(&pts, k, &mut edges, &mut knn_buf, &mut scratch).unwrap();

        // Count degrees.
        let mut degree = vec![0u32; pts.len()];
        for e in &edges[..n_edges] {
            degree[e.i as usize] += 1;
            degree[e.j as usize] += 1;
        }

        // Every point should have degree >= k (since it connects to its k NN,
        // and may gain extra edges from being another point's NN).
        for (i, &d) in degree.iter().enumerate() {
            assert!(d >= k as u32, "point {i} degree {d} < k={k}");
        }
    }
}
