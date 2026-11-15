//! P21.6 — Mean-curvature-flow curve skeletonization for triangle meshes
//! (Au, Tai, Chu, Cohen-Or & Lee 2008 style: iterative Laplacian contraction
//! followed by degenerate-vertex welding and skeleton-graph extraction).
//!
//! Pipeline: contract → weld collapsed vertices → cluster graph → BFS parents.
//! Caller supplies the working buffers; assembly is `ColdBounded`.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveSkeletonError {
    EmptyInput,
    DimensionMismatch,
    ScratchTooSmall { required: usize },
    NoSkeleton,
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Iterative mean-curvature contraction: `v_i ← (1−w)·v_i + w·mean(N(i))`
/// repeated `iterations` times with `w` growing geometrically from
/// `initial_weight` toward 1.
///
/// `sum_scratch`/`cnt_scratch` (≥ vertex count) are caller scratch; `work`
/// receives the contracted positions.
///
/// Tier-1 `HotZeroHeap`.
#[allow(clippy::too_many_arguments)]
pub fn contract_to_skeleton(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    iterations: usize,
    initial_weight: f64,
    sum_scratch: &mut [Point3],
    cnt_scratch: &mut [u32],
    work: &mut [Point3],
) -> Result<(), CurveSkeletonError> {
    let n = vertices.len();
    if n == 0 || triangles.is_empty() {
        return Err(CurveSkeletonError::EmptyInput);
    }
    if sum_scratch.len() < n || cnt_scratch.len() < n || work.len() < n {
        return Err(CurveSkeletonError::ScratchTooSmall { required: n });
    }
    work[..n].copy_from_slice(vertices);
    let mut w = initial_weight.clamp(0.05, 0.9);
    for _ in 0..iterations {
        sum_scratch[..n].fill(Point3::default());
        cnt_scratch[..n].fill(0);
        for t in triangles {
            for e in 0..3 {
                let (a, b) = (t[e] as usize, t[(e + 1) % 3] as usize);
                if a >= n || b >= n {
                    return Err(CurveSkeletonError::DimensionMismatch);
                }
                sum_scratch[a] = add(sum_scratch[a], work[b]);
                cnt_scratch[a] += 1;
                sum_scratch[b] = add(sum_scratch[b], work[a]);
                cnt_scratch[b] += 1;
            }
        }
        for i in 0..n {
            if cnt_scratch[i] == 0 {
                continue;
            }
            let m = 1.0 / (cnt_scratch[i] as f64);
            let mean = Point3::new(
                sum_scratch[i].x * m,
                sum_scratch[i].y * m,
                sum_scratch[i].z * m,
            );
            work[i] = Point3::new(
                work[i].x + w * (mean.x - work[i].x),
                work[i].y + w * (mean.y - work[i].y),
                work[i].z + w * (mean.z - work[i].z),
            );
        }
        w = (w * 1.5).min(0.95);
    }
    Ok(())
}

/// Weld contracted vertices within `merge_eps` into cluster nodes (canonical
/// order), then extract the skeleton graph by BFS from the lowest cluster.
/// Returns node count; `out_nodes`/`out_parents` are caller buffers.
///
/// `ColdBounded`.
#[allow(clippy::too_many_arguments)]
pub fn extract_skeleton_graph(
    contracted: &[Point3],
    triangles: &[[u32; 3]],
    merge_eps: f64,
    node_of_vertex: &mut [u32],
    out_nodes: &mut [Point3],
    out_parents: &mut [u32],
    bfs_queue: &mut [u32],
) -> Result<usize, CurveSkeletonError> {
    let n = contracted.len();
    if n == 0 || !(merge_eps > 0.0 && merge_eps.is_finite()) {
        return Err(CurveSkeletonError::EmptyInput);
    }
    if node_of_vertex.len() < n {
        return Err(CurveSkeletonError::ScratchTooSmall { required: n });
    }
    let eps2 = merge_eps * merge_eps;
    let mut n_nodes = 0usize;
    for i in 0..n {
        let mut assigned = false;
        for c in 0..n_nodes {
            let d = sub(contracted[i], out_nodes[c]);
            if dot(d, d) <= eps2 {
                node_of_vertex[i] = c as u32;
                assigned = true;
                break;
            }
        }
        if !assigned {
            if n_nodes >= out_nodes.len() {
                return Err(CurveSkeletonError::ScratchTooSmall {
                    required: n_nodes + 1,
                });
            }
            out_nodes[n_nodes] = contracted[i];
            node_of_vertex[i] = n_nodes as u32;
            n_nodes += 1;
        }
    }
    if n_nodes == 0 || n_nodes > bfs_queue.len() || out_parents.len() < n_nodes {
        return Err(CurveSkeletonError::NoSkeleton);
    }

    // BFS over the cluster graph from the lowest-z node.
    let mut root = 0usize;
    for c in 1..n_nodes {
        if out_nodes[c].z < out_nodes[root].z {
            root = c;
        }
    }
    out_parents[..n_nodes].fill(u32::MAX);
    out_parents[root] = root as u32;
    let (mut head, mut tail) = (0usize, 1usize);
    bfs_queue[0] = root as u32;
    while head < tail {
        let c = bfs_queue[head] as usize;
        head += 1;
        for t in triangles {
            for e in 0..3 {
                let (a, b) = (t[e] as usize, t[(e + 1) % 3] as usize);
                if a >= n || b >= n {
                    continue;
                }
                let (ca, cb) = (node_of_vertex[a] as usize, node_of_vertex[b] as usize);
                let other = if ca == c {
                    cb
                } else if cb == c {
                    ca
                } else {
                    continue;
                };
                if other != c && out_parents[other] == u32::MAX {
                    out_parents[other] = c as u32;
                    if tail < bfs_queue.len() {
                        bfs_queue[tail] = other as u32;
                        tail += 1;
                    }
                }
            }
        }
    }
    Ok(n_nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_grid() -> (Vec<Point3>, Vec<[u32; 3]>) {
        // 4×4 open grid — contracts toward its interior spine.
        let mut v = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                v.push(Point3::new(x as f64, 0.0, y as f64));
            }
        }
        let mut t = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                let i = (y * 4 + x) as u32;
                t.push([i, i + 1, i + 5]);
                t.push([i, i + 5, i + 4]);
            }
        }
        (v, t)
    }

    #[test]
    fn contraction_pulls_grid_inward() {
        let (v, t) = open_grid();
        let n = v.len();
        let mut sum = vec![Point3::default(); n];
        let mut cnt = vec![0u32; n];
        let mut work = vec![Point3::default(); n];
        contract_to_skeleton(&v, &t, 6, 0.2, &mut sum, &mut cnt, &mut work).unwrap();
        // Corners should move toward the centroid (1.5, 0, 1.5).
        let centroid = Point3::new(1.5, 0.0, 1.5);
        let d0 = sub(v[0], centroid);
        let d1 = sub(work[0], centroid);
        assert!(
            dot(d1, d1) < dot(d0, d0),
            "corner should contract toward centroid"
        );

        let mut nov = vec![u32::MAX; n];
        let mut nodes = vec![Point3::default(); n];
        let mut parents = vec![u32::MAX; n];
        let mut queue = vec![0u32; n];
        let nn = extract_skeleton_graph(
            &work,
            &t,
            0.75,
            &mut nov,
            &mut nodes,
            &mut parents,
            &mut queue,
        )
        .unwrap();
        assert!(nn >= 2 && nn < n, "expected reduced skeleton, got {nn}");
    }
}
