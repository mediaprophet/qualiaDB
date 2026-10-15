//! P20.6 — 3-D Tree Point-Cloud Skeletonization & Allometry:
//! - **Xu, Gossett & Chen (2007)**: "Knowledge and Heuristic-Based Modeling of
//!   Laser-Scanned Trees", *ACM TOG* 26(4) — Geodesic distance-from-root
//!   quotient-graph clustering (`skeleton_from_root_distance_bins`).
//! - **Livny et al. (2010)**: "Automatic Reconstruction of Tree Skeletal
//!   Structures from Point Clouds", *ACM TOG* 29(6) — Subtree carried length,
//!   branch orientation/position optimization, and **Shinozaki / Leonardo da
//!   Vinci Pipe Model** radius estimation (`pipe_model_radii`).
//! - **Tagliasacchi et al. (2009)**: Cross-sectional anisotropic contraction
//!   (`contract_points_anisotropic`).

use super::primitives::Point3;
use super::space_colonization::ROOT_PARENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSkeletonError {
    EmptyInput,
    InvalidParameters,
    CycleDetected,
    OutputTooSmall { required: usize },
}

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn dist(a: Point3, b: Point3) -> f64 {
    let d = sub(a, b);
    (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
}

/// Compute branch radii from tips to root using Leonardo da Vinci / Shinozaki's
/// **Pipe Model** law (PlantGL `estimate_radii_from_pipemodel`):
///
/// For leaf (tip) nodes: $r_i = r_{\text{tip}}$.
/// For internal nodes: $r_i^{\eta} = \sum_{c \in \text{children}(i)} r_c^{\eta}$,
/// where $\eta$ is `pipe_exponent` (typically $2.0 \le \eta \le 2.5$).
///
/// Tier-1 `HotZeroHeap`: zero heap allocations, uses caller-owned `out_radii`
/// and `remaining_children_scratch` (both of length `>= parents.len()`).
pub fn pipe_model_radii(
    parents: &[u32],
    tip_radius: f64,
    pipe_exponent: f64,
    remaining_children_scratch: &mut [u32],
    out_radii: &mut [f64],
) -> Result<usize, TreeSkeletonError> {
    let n = parents.len();
    if n == 0 {
        return Err(TreeSkeletonError::EmptyInput);
    }
    if !(tip_radius > 0.0
        && pipe_exponent > 0.0
        && tip_radius.is_finite()
        && pipe_exponent.is_finite())
    {
        return Err(TreeSkeletonError::InvalidParameters);
    }
    if remaining_children_scratch.len() < n || out_radii.len() < n {
        return Err(TreeSkeletonError::OutputTooSmall { required: n });
    }

    remaining_children_scratch[..n].fill(0);
    out_radii[..n].fill(0.0);

    for (i, &p) in parents.iter().enumerate() {
        if p != ROOT_PARENT && (p as usize) < n && (p as usize) != i {
            remaining_children_scratch[p as usize] += 1;
        }
    }

    let tip_pow = tip_radius.powf(pipe_exponent);
    for i in 0..n {
        if remaining_children_scratch[i] == 0 {
            out_radii[i] = tip_pow;
        }
    }

    // Iterative topological accumulation from leaves to root (zero-heap, non-recursive).
    // Uses high bit (1 << 31) of remaining_children_scratch to mark processed nodes.
    const PROCESSED_BIT: u32 = 1 << 31;
    let mut processed = 0usize;
    while processed < n {
        let mut progressed = false;
        for i in 0..n {
            if remaining_children_scratch[i] == 0 {
                remaining_children_scratch[i] = PROCESSED_BIT;
                processed += 1;
                progressed = true;
                let p = parents[i];
                if p != ROOT_PARENT && (p as usize) < n && (p as usize) != i {
                    let pi = p as usize;
                    out_radii[pi] += out_radii[i];
                    remaining_children_scratch[pi] -= 1;
                }
            }
        }
        if !progressed {
            return Err(TreeSkeletonError::CycleDetected);
        }
    }

    let inv_exp = 1.0 / pipe_exponent;
    for r in &mut out_radii[..n] {
        *r = r.max(tip_pow).powf(inv_exp);
    }

    Ok(n)
}

/// Compute the carried subtree branch length for every skeleton node
/// (PlantGL `carried_length`, Livny et al. 2010).
///
/// Tier-1 `HotZeroHeap`.
pub fn carried_subtree_length(
    nodes: &[Point3],
    parents: &[u32],
    remaining_children_scratch: &mut [u32],
    out_carried_length: &mut [f64],
) -> Result<usize, TreeSkeletonError> {
    let n = nodes.len();
    if n == 0 || parents.len() != n {
        return Err(TreeSkeletonError::EmptyInput);
    }
    if remaining_children_scratch.len() < n || out_carried_length.len() < n {
        return Err(TreeSkeletonError::OutputTooSmall { required: n });
    }

    remaining_children_scratch[..n].fill(0);
    out_carried_length[..n].fill(0.0);

    for (i, &p) in parents.iter().enumerate() {
        if p != ROOT_PARENT && (p as usize) < n && (p as usize) != i {
            remaining_children_scratch[p as usize] += 1;
        }
    }

    const PROCESSED_BIT: u32 = 1 << 31;
    let mut processed = 0usize;
    while processed < n {
        let mut progressed = false;
        for i in 0..n {
            if remaining_children_scratch[i] == 0 {
                remaining_children_scratch[i] = PROCESSED_BIT;
                processed += 1;
                progressed = true;
                let p = parents[i];
                if p != ROOT_PARENT && (p as usize) < n && (p as usize) != i {
                    let pi = p as usize;
                    let edge_len = dist(nodes[i], nodes[pi]);
                    out_carried_length[pi] += out_carried_length[i] + edge_len;
                    remaining_children_scratch[pi] -= 1;
                }
            }
        }
        if !progressed {
            return Err(TreeSkeletonError::CycleDetected);
        }
    }
    Ok(n)
}

/// Livny et al. (2010) subtree-weighted branch position smoothing
/// (PlantGL `optimize_positions`).
///
/// Each non-root interior node is blended towards the weighted average of its
/// parent and its children (weighted by `weights`, e.g., carried length).
pub fn smooth_skeleton_positions(
    nodes: &[Point3],
    parents: &[u32],
    weights: &[f64],
    damping: f64,
    out_nodes: &mut [Point3],
) -> Result<usize, TreeSkeletonError> {
    let n = nodes.len();
    if n == 0 || parents.len() != n || weights.len() != n || !(0.0..=1.0).contains(&damping) {
        return Err(TreeSkeletonError::InvalidParameters);
    }
    if out_nodes.len() < n {
        return Err(TreeSkeletonError::OutputTooSmall { required: n });
    }

    let mut child_sum = vec![Point3::new(0.0, 0.0, 0.0); n];
    let mut child_w = vec![0.0f64; n];
    for (i, &p) in parents.iter().enumerate() {
        if p != ROOT_PARENT && (p as usize) < n && (p as usize) != i {
            let w = weights[i].max(1e-6);
            child_sum[p as usize] = add(child_sum[p as usize], scale(nodes[i], w));
            child_w[p as usize] += w;
        }
    }

    for i in 0..n {
        let p = parents[i];
        if p == ROOT_PARENT || (p as usize) >= n || (p as usize) == i || child_w[i] <= 0.0 {
            out_nodes[i] = nodes[i];
            continue;
        }
        let pw = weights[p as usize].max(1e-6);
        let target = scale(
            add(scale(nodes[p as usize], pw), child_sum[i]),
            1.0 / (pw + child_w[i]),
        );
        out_nodes[i] = add(scale(nodes[i], 1.0 - damping), scale(target, damping));
    }

    Ok(n)
}

/// Xu et al. (2007) Geodesic Quotient-Graph Tree Skeletonization
/// (PlantGL `skeleton_from_distance_to_root_clusters`).
///
/// 1. Connects each point to its `k_neighbors` closest neighbors (and ensures
///    global connectivity by linking any disconnected component to its closest
///    point in the root's component).
/// 2. Computes Dijkstra shortest-path geodesic distances from `root_index`.
/// 3. Partitions points into geodesic distance shells $[b \cdot \Delta, (b+1) \cdot \Delta)$
///    of width `bin_size`.
/// 4. Extracts connected components within each shell, computes their centroids
///    as skeleton nodes, and connects each component to its parent component in
///    the preceding shell.
pub fn skeleton_from_root_distance_bins(
    points: &[Point3],
    root_index: usize,
    bin_size: f64,
    k_neighbors: usize,
    out_nodes: &mut [Point3],
    out_parents: &mut [u32],
) -> Result<usize, TreeSkeletonError> {
    let n = points.len();
    if n == 0 || root_index >= n || !(bin_size > 0.0 && bin_size.is_finite()) || k_neighbors == 0 {
        return Err(TreeSkeletonError::InvalidParameters);
    }

    // Build symmetric kNN adjacency list
    let k = k_neighbors.min(n.saturating_sub(1)).max(1);
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        let mut dists: Vec<(f64, usize)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (dist(points[i], points[j]), j))
            .collect();
        dists.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        for &(_, j) in dists.iter().take(k) {
            if !adj[i].contains(&j) {
                adj[i].push(j);
            }
            if !adj[j].contains(&i) {
                adj[j].push(i);
            }
        }
    }

    // Dijkstra shortest-path distances from root_index (reconnecting isolated components)
    let mut geo_dist = vec![f64::INFINITY; n];
    let mut pred = vec![root_index; n];
    let mut visited = vec![false; n];
    geo_dist[root_index] = 0.0;

    for _ in 0..n {
        let mut u = None;
        let mut best_d = f64::INFINITY;
        for i in 0..n {
            if !visited[i] && geo_dist[i] < best_d {
                best_d = geo_dist[i];
                u = Some(i);
            }
        }
        let u = if let Some(idx) = u {
            idx
        } else {
            // Reconnect closest unvisited point to visited set (PlantGL connect_all_connex_components)
            let mut best_pair = (0usize, 0usize, f64::INFINITY);
            for i in 0..n {
                if visited[i] {
                    for j in 0..n {
                        if !visited[j] {
                            let d = dist(points[i], points[j]);
                            if d < best_pair.2 {
                                best_pair = (i, j, d);
                            }
                        }
                    }
                }
            }
            let (vi, uj, d) = best_pair;
            adj[vi].push(uj);
            adj[uj].push(vi);
            geo_dist[uj] = geo_dist[vi] + d;
            pred[uj] = vi;
            uj
        };

        visited[u] = true;
        for &v in &adj[u] {
            if !visited[v] {
                let alt = geo_dist[u] + dist(points[u], points[v]);
                if alt < geo_dist[v] {
                    geo_dist[v] = alt;
                    pred[v] = u;
                }
            }
        }
    }

    // Assign each point to its geodesic distance bin
    let mut bin_of = vec![0usize; n];
    let mut max_bin = 0usize;
    for i in 0..n {
        let b = (geo_dist[i] / bin_size).floor() as usize;
        bin_of[i] = b;
        max_bin = max_bin.max(b);
    }

    // Extract connected components within each bin
    let mut comp_of_point = vec![usize::MAX; n];
    let mut centroids: Vec<Point3> = Vec::new();
    let mut comp_bin: Vec<usize> = Vec::new();
    let mut comp_members: Vec<Vec<usize>> = Vec::new();

    for b in 0..=max_bin {
        for i in 0..n {
            if bin_of[i] != b || comp_of_point[i] != usize::MAX {
                continue;
            }
            let cid = centroids.len();
            let mut queue = vec![i];
            comp_of_point[i] = cid;
            let mut head = 0usize;
            let mut sum = Point3::new(0.0, 0.0, 0.0);
            while head < queue.len() {
                let cur = queue[head];
                head += 1;
                sum = add(sum, points[cur]);
                for &nb in &adj[cur] {
                    if bin_of[nb] == b && comp_of_point[nb] == usize::MAX {
                        comp_of_point[nb] = cid;
                        queue.push(nb);
                    }
                }
            }
            centroids.push(scale(sum, 1.0 / (queue.len() as f64)));
            comp_bin.push(b);
            comp_members.push(queue);
        }
    }

    let num_nodes = centroids.len();
    if out_nodes.len() < num_nodes || out_parents.len() < num_nodes {
        return Err(TreeSkeletonError::OutputTooSmall {
            required: num_nodes,
        });
    }

    let root_comp = comp_of_point[root_index];
    for cid in 0..num_nodes {
        out_nodes[cid] = centroids[cid];
        if cid == root_comp {
            out_parents[cid] = ROOT_PARENT;
            continue;
        }
        // Trace geodesic predecessors from members of `cid` to find parent component in an earlier bin
        let mut parent_cid = root_comp;
        let my_bin = comp_bin[cid];
        'search: for &pt_idx in &comp_members[cid] {
            let mut cur = pred[pt_idx];
            for _ in 0..n {
                let cand = comp_of_point[cur];
                if comp_bin[cand] < my_bin {
                    parent_cid = cand;
                    break 'search;
                }
                if cur == root_index {
                    break;
                }
                cur = pred[cur];
            }
        }
        out_parents[cid] = parent_cid as u32;
    }

    Ok(num_nodes)
}

/// Tagliasacchi et al. (2009) / PlantGL `adaptive_contration`: contract each
/// point towards the centroid of its anisotropic local neighborhood weighted
/// perpendicular to `orientations[i]`.
///
/// Tier-1 `HotZeroHeap`.
pub fn contract_points_anisotropic(
    points: &[Point3],
    orientations: &[Point3],
    radius: f64,
    alpha_axial: f64,
    beta_radial: f64,
    out_contracted: &mut [Point3],
) -> Result<usize, TreeSkeletonError> {
    let n = points.len();
    if n == 0
        || orientations.len() != n
        || !(radius > 0.0 && alpha_axial > 0.0 && beta_radial > 0.0)
    {
        return Err(TreeSkeletonError::InvalidParameters);
    }
    if out_contracted.len() < n {
        return Err(TreeSkeletonError::OutputTooSmall { required: n });
    }

    let r2 = radius * radius;
    for i in 0..n {
        let p = points[i];
        let dir = orientations[i];
        let dir_norm = (dir.x * dir.x + dir.y * dir.y + dir.z * dir.z).sqrt();
        let axis = if dir_norm > 1e-12 {
            scale(dir, 1.0 / dir_norm)
        } else {
            Point3::new(0.0, 0.0, 1.0)
        };

        let mut sum = Point3::new(0.0, 0.0, 0.0);
        let mut count = 0usize;
        for &q in points {
            let diff = sub(q, p);
            let axial = dot(diff, axis);
            let radial_vec = sub(diff, scale(axis, axial));
            let radial_sq = dot(radial_vec, radial_vec);
            let aniso_d2 = (axial * axial) / (alpha_axial * alpha_axial)
                + radial_sq / (beta_radial * beta_radial);
            if aniso_d2 <= r2 {
                sum = add(sum, q);
                count += 1;
            }
        }
        out_contracted[i] = if count > 0 {
            scale(sum, 1.0 / (count as f64))
        } else {
            p
        };
    }

    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xu_skeletonizer_and_pipe_model_reconstruct_y_branch() {
        let mut cloud = Vec::new();
        // Trunk from (0,0,0) to (0,0,1.0)
        for i in 0..=10 {
            let z = i as f64 * 0.1;
            cloud.push(Point3::new(0.0, 0.0, z));
        }
        // Left fork to (-0.8, 0, 1.8) and right fork to (+0.8, 0, 1.8)
        for i in 1..=8 {
            let d = i as f64 * 0.1;
            cloud.push(Point3::new(-d, 0.0, 1.0 + d));
            cloud.push(Point3::new(d, 0.0, 1.0 + d));
        }

        let mut nodes = vec![Point3::default(); 64];
        let mut parents = vec![ROOT_PARENT; 64];
        let num_nodes =
            skeleton_from_root_distance_bins(&cloud, 0, 0.35, 4, &mut nodes, &mut parents).unwrap();
        assert!(
            num_nodes >= 5,
            "expected forked skeleton nodes, got {num_nodes}"
        );

        let mut scratch = vec![0u32; num_nodes];
        let mut radii = vec![0.0f64; num_nodes];
        pipe_model_radii(&parents[..num_nodes], 0.02, 2.0, &mut scratch, &mut radii).unwrap();
        // Root radius must exceed tip radius (0.02) due to two child branches merging
        assert!(
            radii[0] > 0.025,
            "root radius should accumulate child pipes, got {}",
            radii[0]
        );
    }
}
