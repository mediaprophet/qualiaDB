// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Spatial Join with Summary Aggregations, Hausdorff & Discrete Fréchet Distance,
//! Silverman Quartic Kernel Density Estimation (KDE), and DBSCAN / K-Means++ Clustering.
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `gp_hausdorff_distance`, `gp_discrete_frechet_distance`,
//!   `gp_kernel_density_quartic_into`.
//! - Tier-2 (`ColdBounded`): `gp_spatial_join_points_to_polygons`, `gp_cluster_dbscan`,
//!   `gp_cluster_kmeans_pp` (enforces caller-supplied `byte_budget`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialJoinPredicate { Intersects, WithinDistance(u64), ContainsPoint }

#[derive(Debug, Clone, PartialEq)]
pub struct SpatialJoinSummary {
    pub target_id: u64,
    pub matched_count: usize,
    pub val_sum: f64,
    pub val_mean: f64,
    pub val_min: f64,
    pub val_max: f64,
    pub nearest_dist: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpSpatialError {
    EmptyDataset,
    InvalidBandwidth,
    InvalidClusterK,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GpSpatialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyDataset => write!(f, "Empty dataset provided to spatial operation"),
            Self::InvalidBandwidth => write!(f, "Invalid bandwidth parameter (>0 and finite)"),
            Self::InvalidClusterK => write!(f, "Invalid cluster count K (>0 and <= point count)"),
            Self::ByteBudgetExceeded { requested, budget } => {
                write!(f, "Budget exceeded: req {requested} B, budget {budget} B")
            }
        }
    }
}
impl std::error::Error for GpSpatialError {}

#[inline]
fn dist_sq_pt(p1: [f64; 2], p2: [f64; 2]) -> f64 {
    (p1[0] - p2[0]) * (p1[0] - p2[0]) + (p1[1] - p2[1]) * (p1[1] - p2[1])
}

#[inline]
fn dist_sq_pt_to_seg(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (abx, aby, apx, apy) = (b[0] - a[0], b[1] - a[1], p[0] - a[0], p[1] - a[1]);
    let len_sq = abx * abx + aby * aby;
    if len_sq <= 1e-18 { return apx * apx + apy * apy; }
    let t = ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0);
    let (dx, dy) = (p[0] - (a[0] + t * abx), p[1] - (a[1] + t * aby));
    dx * dx + dy * dy
}

fn point_in_poly(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    if ring.len() < 3 { return false; }
    let (mut inside, n, mut j) = (false, ring.len(), ring.len() - 1);
    for i in 0..n {
        let (xi, yi, xj, yj) = (ring[i][0], ring[i][1], ring[j][0], ring[j][1]);
        if ((yi > p[1]) != (yj > p[1])) && (p[0] < (xj - xi) * (p[1] - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_pt_to_poly_boundary(p: [f64; 2], ring: &[[f64; 2]]) -> f64 {
    if ring.is_empty() { return f64::INFINITY; }
    if ring.len() == 1 { return dist_sq_pt(p, ring[0]).sqrt(); }
    let (mut min_d2, n, mut j) = (f64::INFINITY, ring.len(), ring.len() - 1);
    for i in 0..n {
        let d2 = dist_sq_pt_to_seg(p, ring[i], ring[j]);
        if d2 < min_d2 { min_d2 = d2; }
        j = i;
    }
    min_d2.sqrt()
}

/// Joins points to target polygons, computing summary statistics and nearest distance.
pub fn gp_spatial_join_points_to_polygons(
    target_polys: &[(u64, Vec<[f64; 2]>)],
    join_points_xy_val: &[([f64; 2], f64)],
    max_search_radius: f64,
    include_within_radius: bool,
    byte_budget: usize,
) -> Result<Vec<SpatialJoinSummary>, GpSpatialError> {
    if target_polys.is_empty() || join_points_xy_val.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    let requested = target_polys.len().saturating_mul(core::mem::size_of::<SpatialJoinSummary>());
    if requested > byte_budget { return Err(GpSpatialError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let mut summaries = Vec::with_capacity(target_polys.len());
    for (target_id, poly) in target_polys {
        let (mut count, mut val_sum) = (0usize, 0.0f64);
        let (mut val_min, mut val_max, mut nearest_dist) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY);

        for &(pt, val) in join_points_xy_val {
            let inside = point_in_poly(pt, poly);
            let dist = if inside { 0.0 } else { dist_pt_to_poly_boundary(pt, poly) };
            if inside || (include_within_radius && dist <= max_search_radius) {
                count += 1;
                val_sum += val;
                val_min = val_min.min(val);
                val_max = val_max.max(val);
                nearest_dist = nearest_dist.min(dist);
            }
        }
        summaries.push(SpatialJoinSummary {
            target_id: *target_id,
            matched_count: count,
            val_sum,
            val_mean: if count > 0 { val_sum / count as f64 } else { 0.0 },
            val_min: if count > 0 { val_min } else { 0.0 },
            val_max: if count > 0 { val_max } else { 0.0 },
            nearest_dist: if count > 0 { nearest_dist } else { f64::INFINITY },
        });
    }
    Ok(summaries)
}

/// Tier-1 `HotZeroHeap` symmetric directed Hausdorff distance between two curves or point sets.
pub fn gp_hausdorff_distance(curve_a: &[[f64; 2]], curve_b: &[[f64; 2]]) -> Result<f64, GpSpatialError> {
    if curve_a.is_empty() || curve_b.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    let dir_h = |from: &[[f64; 2]], to: &[[f64; 2]]| -> f64 {
        let mut max_d2 = 0.0f64;
        for &p in from {
            let mut min_d2 = f64::INFINITY;
            for &q in to { min_d2 = min_d2.min(dist_sq_pt(p, q)); }
            max_d2 = max_d2.max(min_d2);
        }
        max_d2
    };
    Ok(dir_h(curve_a, curve_b).max(dir_h(curve_b, curve_a)).sqrt())
}

/// Tier-1 `HotZeroHeap` discrete Fréchet curve similarity metric using caller-supplied DP scratch.
pub fn gp_discrete_frechet_distance(
    curve_a: &[[f64; 2]],
    curve_b: &[[f64; 2]],
    dp_scratch: &mut [f64],
) -> Result<f64, GpSpatialError> {
    if curve_a.is_empty() || curve_b.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    let (n, m) = (curve_a.len(), curve_b.len());
    let requested = n.checked_mul(m).ok_or(GpSpatialError::ByteBudgetExceeded { requested: usize::MAX, budget: dp_scratch.len() })?;
    if dp_scratch.len() < requested { return Err(GpSpatialError::ByteBudgetExceeded { requested, budget: dp_scratch.len() }); }

    dp_scratch[0] = dist_sq_pt(curve_a[0], curve_b[0]).sqrt();
    for j in 1..m { dp_scratch[j] = dp_scratch[j - 1].max(dist_sq_pt(curve_a[0], curve_b[j]).sqrt()); }
    for i in 1..n { dp_scratch[i * m] = dp_scratch[(i - 1) * m].max(dist_sq_pt(curve_a[i], curve_b[0]).sqrt()); }
    for i in 1..n {
        for j in 1..m {
            let d = dist_sq_pt(curve_a[i], curve_b[j]).sqrt();
            let prev_min = dp_scratch[(i - 1) * m + j].min(dp_scratch[i * m + (j - 1)]).min(dp_scratch[(i - 1) * m + (j - 1)]);
            dp_scratch[i * m + j] = prev_min.max(d);
        }
    }
    Ok(dp_scratch[(n - 1) * m + (m - 1)])
}

/// Tier-1 `HotZeroHeap` Silverman Quartic Biweight Kernel Density Estimation.
pub fn gp_kernel_density_quartic_into(
    points_xy_weight: &[([f64; 2], f64)],
    bandwidth: f64,
    bbox: [f64; 4],
    width: usize,
    height: usize,
    out_density: &mut [f32],
) -> Result<f32, GpSpatialError> {
    if points_xy_weight.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    if bandwidth <= 0.0 || !bandwidth.is_finite() { return Err(GpSpatialError::InvalidBandwidth); }
    let total_cells = width.checked_mul(height).ok_or(GpSpatialError::ByteBudgetExceeded { requested: usize::MAX, budget: out_density.len() })?;
    if width == 0 || height == 0 || out_density.len() < total_cells {
        return Err(GpSpatialError::ByteBudgetExceeded { requested: total_cells, budget: out_density.len() });
    }

    out_density[..total_cells].fill(0.0);
    let (dx, dy) = ((bbox[2] - bbox[0]) / width as f64, (bbox[3] - bbox[1]) / height as f64);
    let (h2, norm) = (bandwidth * bandwidth, 3.0 / (core::f64::consts::PI * bandwidth * bandwidth));

    for r in 0..height {
        let y = bbox[1] + (r as f64 + 0.5) * dy;
        for c in 0..width {
            let x = bbox[0] + (c as f64 + 0.5) * dx;
            let mut cell_sum = 0.0f64;
            for &(pt, w) in points_xy_weight {
                let d2 = dist_sq_pt([x, y], pt);
                if d2 < h2 {
                    let term = 1.0 - d2 / h2;
                    cell_sum += w * norm * term * term;
                }
            }
            out_density[r * width + c] = cell_sum as f32;
        }
    }
    let mut peak = 0.0f32;
    for &val in &out_density[..total_cells] {
        if val > peak { peak = val; }
    }
    Ok(peak)
}

/// Tier-2 `ColdBounded` DBSCAN spatial clustering with noise classification (`-1`).
pub fn gp_cluster_dbscan(
    points: &[[f64; 2]],
    eps_radius: f64,
    min_pts: usize,
    byte_budget: usize,
) -> Result<Vec<i32>, GpSpatialError> {
    if points.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    if eps_radius <= 0.0 || !eps_radius.is_finite() { return Err(GpSpatialError::InvalidBandwidth); }
    let n = points.len();
    let requested = n.saturating_mul(24);
    if requested > byte_budget { return Err(GpSpatialError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let (eps2, mut labels, mut visited, mut cluster_id) = (eps_radius * eps_radius, vec![-1i32; n], vec![false; n], 0i32);
    for i in 0..n {
        if visited[i] { continue; }
        visited[i] = true;
        let neighbors: Vec<usize> = (0..n).filter(|&j| dist_sq_pt(points[i], points[j]) <= eps2).collect();
        if neighbors.len() < min_pts {
            labels[i] = -1;
        } else {
            labels[i] = cluster_id;
            let (mut queue, mut in_queue) = (Vec::new(), vec![false; n]);
            for &nb in &neighbors {
                if nb != i { queue.push(nb); in_queue[nb] = true; }
            }
            let mut head = 0;
            while head < queue.len() {
                let p = queue[head];
                head += 1;
                if !visited[p] {
                    visited[p] = true;
                    let p_nb: Vec<usize> = (0..n).filter(|&j| dist_sq_pt(points[p], points[j]) <= eps2).collect();
                    if p_nb.len() >= min_pts {
                        for q in p_nb {
                            if !in_queue[q] { in_queue[q] = true; queue.push(q); }
                        }
                    }
                }
                if labels[p] == -1 { labels[p] = cluster_id; }
            }
            cluster_id += 1;
        }
    }
    Ok(labels)
}

/// Tier-2 `ColdBounded` deterministic K-Means++ clustering with Lloyd iterations.
pub fn gp_cluster_kmeans_pp(
    points: &[[f64; 2]],
    k: usize,
    max_iters: usize,
    seed: u64,
    byte_budget: usize,
) -> Result<(Vec<usize>, Vec<[f64; 2]>), GpSpatialError> {
    if points.is_empty() { return Err(GpSpatialError::EmptyDataset); }
    if k == 0 || k > points.len() { return Err(GpSpatialError::InvalidClusterK); }
    let n = points.len();
    let requested = n.saturating_mul(16).saturating_add(k.saturating_mul(48));
    if requested > byte_budget { return Err(GpSpatialError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let mut rng_state = if seed == 0 { 0x853c_49e6_748f_ea9b } else { seed };
    let mut next_u64 = || {
        rng_state = rng_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };

    let mut centroids = Vec::with_capacity(k);
    centroids.push(points[(next_u64() as usize) % n]);
    let mut d2 = vec![f64::INFINITY; n];

    while centroids.len() < k {
        let last_c = centroids[centroids.len() - 1];
        let mut sum_d2 = 0.0f64;
        for i in 0..n {
            let d = dist_sq_pt(points[i], last_c);
            if d < d2[i] { d2[i] = d; }
            sum_d2 += d2[i];
        }
        if sum_d2 <= 1e-12 {
            for &p in points {
                if !centroids.iter().any(|&c| dist_sq_pt(c, p) < 1e-12) {
                    centroids.push(p);
                    break;
                }
            }
            if centroids.len() < k { centroids.push(points[0]); }
        } else {
            let r_f64 = (next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            let target = r_f64 * sum_d2;
            let (mut acc, mut chosen) = (0.0f64, 0);
            for (i, &d) in d2.iter().enumerate() {
                acc += d;
                if acc >= target { chosen = i; break; }
            }
            centroids.push(points[chosen]);
        }
    }

    let mut assignments = vec![0usize; n];
    for iter in 0..max_iters.max(1) {
        let mut changed = false;
        for i in 0..n {
            let p = points[i];
            let (mut best_c, mut min_d) = (0, f64::INFINITY);
            for (c_idx, &c) in centroids.iter().enumerate() {
                let d = dist_sq_pt(p, c);
                if d < min_d { min_d = d; best_c = c_idx; }
            }
            if assignments[i] != best_c { assignments[i] = best_c; changed = true; }
        }
        if !changed && iter > 0 { break; }
        if max_iters > 0 {
            let mut sums = vec![[0.0f64; 2]; k];
            let mut counts = vec![0usize; k];
            for i in 0..n {
                let c = assignments[i];
                sums[c][0] += points[i][0];
                sums[c][1] += points[i][1];
                counts[c] += 1;
            }
            for c in 0..k {
                if counts[c] > 0 {
                    centroids[c] = [sums[c][0] / counts[c] as f64, sums[c][1] / counts[c] as f64];
                }
            }
        }
    }
    Ok((assignments, centroids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_join_points_to_polygons() {
        let poly_square = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]];
        let targets = vec![(42u64, poly_square)];
        let points = vec![
            ([1.0, 1.0], 10.0), ([0.5, 0.5], 20.0), ([1.5, 1.5], 30.0), ([5.0, 5.0], 100.0),
        ];
        let res = gp_spatial_join_points_to_polygons(&targets, &points, 0.0, false, 1024).unwrap();
        assert_eq!(res.len(), 1);
        let s = &res[0];
        assert_eq!(s.target_id, 42);
        assert_eq!(s.matched_count, 3);
        assert!((s.val_sum - 60.0).abs() < 1e-9 && (s.val_mean - 20.0).abs() < 1e-9);
        assert!((s.val_min - 10.0).abs() < 1e-9 && (s.val_max - 30.0).abs() < 1e-9);
        assert!((s.nearest_dist - 0.0).abs() < 1e-9);

        let res_radius = gp_spatial_join_points_to_polygons(&targets, &points, 5.0, true, 1024).unwrap();
        assert_eq!(res_radius[0].matched_count, 4);
    }

    #[test]
    fn test_hausdorff_and_frechet_distance() {
        let curve_a = vec![[0.0, 0.0], [5.0, 0.0], [10.0, 0.0]];
        let curve_b = vec![[0.0, 3.0], [5.0, 3.0], [10.0, 3.0]];
        let h_dist = gp_hausdorff_distance(&curve_a, &curve_b).unwrap();
        assert!((h_dist - 3.0).abs() < 1e-9);

        let mut scratch = [0.0f64; 16];
        let f_dist = gp_discrete_frechet_distance(&curve_a, &curve_b, &mut scratch).unwrap();
        assert!((f_dist - 3.0).abs() < 1e-9);

        let zig_a = vec![[0.0, 0.0], [1.0, 2.0], [2.0, 0.0]];
        let zig_b = vec![[0.0, 1.0], [1.0, 3.0], [2.0, 1.0]];
        let h_zig = gp_hausdorff_distance(&zig_a, &zig_b).unwrap();
        let f_zig = gp_discrete_frechet_distance(&zig_a, &zig_b, &mut scratch).unwrap();
        assert!(h_zig > 0.0 && f_zig >= h_zig);
    }

    #[test]
    fn test_kernel_density_quartic_into() {
        let pts = vec![
            ([5.5, 5.5], 1.0), ([5.6, 5.5], 1.0), ([5.4, 5.5], 1.0), ([5.5, 5.6], 1.0), ([5.5, 5.4], 1.0),
        ];
        let mut grid = [0.0f32; 100];
        let peak = gp_kernel_density_quartic_into(&pts, 2.0, [0.0, 0.0, 10.0, 10.0], 10, 10, &mut grid).unwrap();
        assert!(peak > 0.0);
        assert_eq!(peak, grid[5 * 10 + 5]);
    }

    #[test]
    fn test_cluster_dbscan_and_kmeans_pp() {
        let points = vec![
            [0.0, 0.0], [0.1, 0.0], [0.0, 0.1],
            [10.0, 10.0], [10.1, 10.0], [10.0, 10.1],
            [50.0, 50.0],
        ];
        let db_labels = gp_cluster_dbscan(&points, 0.5, 2, 4096).unwrap();
        assert_eq!(db_labels.len(), 7);
        assert_eq!(db_labels[0], db_labels[1]);
        assert_eq!(db_labels[1], db_labels[2]);
        assert_ne!(db_labels[0], -1);
        assert_eq!(db_labels[3], db_labels[4]);
        assert_eq!(db_labels[4], db_labels[5]);
        assert_ne!(db_labels[3], -1);
        assert_ne!(db_labels[0], db_labels[3]);
        assert_eq!(db_labels[6], -1);

        let pts2 = &points[..6];
        let (assign, centroids) = gp_cluster_kmeans_pp(pts2, 2, 20, 42, 4096).unwrap();
        assert_eq!(assign.len(), 6);
        assert_eq!(centroids.len(), 2);
        assert_eq!(assign[0], assign[1]);
        assert_eq!(assign[1], assign[2]);
        assert_eq!(assign[3], assign[4]);
        assert_eq!(assign[4], assign[5]);
        assert_ne!(assign[0], assign[3]);
    }

    #[test]
    fn test_byte_budget_exceeded() {
        let pts = vec![[0.0, 0.0], [1.0, 1.0]];
        let err = gp_cluster_dbscan(&pts, 0.5, 2, 10).unwrap_err();
        assert!(matches!(err, GpSpatialError::ByteBudgetExceeded { .. }));
    }
}
