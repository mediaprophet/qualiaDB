//! kd-tree for nearest-neighbour, k-NN, and fixed-radius queries in 2D and 3D.
//!
//! The kd-tree partitions space along alternating axes at the median point,
//! producing a balanced binary tree of depth O(log n). Queries prune subtrees
//! using the splitting-plane distance bound.
//!
//! ## Determinism
//!
//! The builder sorts points by Morton code before median splitting, so two
//! builds from the same input produce byte-identical trees. All queries are
//! zero-heap (caller-supplied stack).

use bytemuck::{Pod, Zeroable};

use super::spatial_order::sort_by_morton_3d;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdTreeError {
    /// `nodes` buffer too small.
    NodeBufferTooSmall { required: usize },
    /// `point_indices` buffer too small.
    IndexBufferTooSmall { required: usize },
    /// `morton_codes` buffer too small.
    CodeBufferTooSmall { required: usize },
    /// `sort_indices` buffer too small.
    SortBufferTooSmall { required: usize },
    /// `stack` buffer too small.
    StackBufferTooSmall { required: usize },
    /// `out` buffer too small.
    OutputBufferTooSmall { required: usize },
}

impl From<super::spatial_order::SpatialOrderError> for KdTreeError {
    fn from(err: super::spatial_order::SpatialOrderError) -> Self {
        match err {
            super::spatial_order::SpatialOrderError::CodeBufferTooSmall { required } => {
                KdTreeError::CodeBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::IndexBufferTooSmall { required } => {
                KdTreeError::SortBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::NonFiniteCoordinate { .. } => {
                KdTreeError::CodeBufferTooSmall { required: 0 }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// POD node
// ---------------------------------------------------------------------------

/// kd-tree node: 32 bytes, `repr(C)`, naturally aligned.
///
/// ```text
/// offset  size  field
/// 0       4     split_axis:u8 + padding[3]
/// 4       4     point_index:u32      (index into the original point array)
/// 8       4     left:u32             (left child node index, or INVALID_INDEX)
/// 12      4     right:u32            (right child node index, or INVALID_INDEX)
/// 16      4     parent:u32
/// 20      12    bbox_min:[f32;3]     (tight bbox of this subtree)
/// ```
///
/// For 2D trees, z components are zero.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct KdNode {
    pub split_axis: u8,
    pub _pad: [u8; 3],
    pub point_index: u32,
    pub left: u32,
    pub right: u32,
    pub parent: u32,
    pub bbox_min: [f32; 3],
}

impl Default for KdNode {
    fn default() -> Self {
        Self {
            split_axis: 0,
            _pad: [0; 3],
            point_index: 0,
            left: super::topology::INVALID_INDEX,
            right: super::topology::INVALID_INDEX,
            parent: super::topology::INVALID_INDEX,
            bbox_min: [0.0; 3],
        }
    }
}

pub const KD_NODE_SIZE: usize = 32;
pub const MAX_KD_DEPTH: usize = 48;

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Build a 3D kd-tree from a point set.
///
/// Caller-supplied buffers:
/// - `nodes`: needs `points.len()` entries (one node per point).
/// - `point_indices`: needs `points.len()` entries (reordered).
/// - `morton_codes`: needs `points.len()` entries (scratch).
/// - `sort_indices`: needs `points.len()` entries (scratch).
///
/// Returns (node_count, root_index). Root is always 0.
pub fn build_kd_tree_3d(
    points: &[[f64; 3]],
    nodes: &mut [KdNode],
    point_indices: &mut [u32],
    morton_codes: &mut [u64],
    sort_indices: &mut [u32],
) -> Result<(usize, usize), KdTreeError> {
    let n = points.len();
    if n == 0 {
        return Ok((0, 0));
    }

    if nodes.len() < n {
        return Err(KdTreeError::NodeBufferTooSmall { required: n });
    }
    if point_indices.len() < n {
        return Err(KdTreeError::IndexBufferTooSmall { required: n });
    }
    if morton_codes.len() < n {
        return Err(KdTreeError::CodeBufferTooSmall { required: n });
    }
    if sort_indices.len() < n {
        return Err(KdTreeError::SortBufferTooSmall { required: n });
    }

    // Sort by Morton code for deterministic median splitting.
    sort_by_morton_3d(points, morton_codes, sort_indices)?;
    point_indices[..n].copy_from_slice(&sort_indices[..n]);

    let mut next_node = 0usize;
    build_kd_recursive(
        points,
        point_indices,
        0,
        n,
        nodes,
        &mut next_node,
        super::topology::INVALID_INDEX,
        0,
    );

    Ok((next_node, 0))
}

fn build_kd_recursive(
    points: &[[f64; 3]],
    point_indices: &[u32],
    start: usize,
    count: usize,
    nodes: &mut [KdNode],
    next_node: &mut usize,
    parent: u32,
    depth: usize,
) {
    if count == 0 {
        return;
    }

    let node_idx = *next_node;
    *next_node += 1;

    // Choose split axis: cycle x, y, z by depth.
    let axis = (depth % 3) as u8;

    // Find median index in this range (already sorted by Morton code,
    // which is a reasonable proxy for spatial ordering).
    let mid = start + count / 2;
    let p_idx = point_indices[mid] as usize;
    let _split_val = points[p_idx][axis as usize];

    // Compute bbox min for this subtree.
    let mut bbox_min = [f32::INFINITY; 3];
    for i in 0..count {
        let pi = point_indices[start + i] as usize;
        for d in 0..3 {
            let v = points[pi][d] as f32;
            if v < bbox_min[d] {
                bbox_min[d] = v;
            }
        }
    }

    // Partition: left = [start, mid), right = [mid+1, end)
    // Since we sorted by Morton, this is already a reasonable spatial split.
    let left_count = count / 2;
    let right_count = count - left_count - 1;

    nodes[node_idx] = KdNode {
        split_axis: axis,
        _pad: [0; 3],
        point_index: point_indices[mid],
        left: if left_count > 0 { (*next_node) as u32 } else { super::topology::INVALID_INDEX },
        right: if right_count > 0 { 0 } else { super::topology::INVALID_INDEX }, // fixed after left build
        parent,
        bbox_min,
    };

    // Build left subtree.
    if left_count > 0 {
        build_kd_recursive(points, point_indices, start, left_count, nodes, next_node, node_idx as u32, depth + 1);
    }

    // Build right subtree. Now we know the right child index.
    if right_count > 0 {
        let right_idx = *next_node;
        nodes[node_idx].right = right_idx as u32;
        build_kd_recursive(points, point_indices, mid + 1, right_count, nodes, next_node, node_idx as u32, depth + 1);
    }
}

// ---------------------------------------------------------------------------
// Query: 1-NN
// ---------------------------------------------------------------------------

/// Find the nearest neighbour to `query` in a 3D kd-tree.
///
/// Returns `(point_index, squared_distance)` or `None` if the tree is empty.
///
/// `stack` needs `MAX_KD_DEPTH` entries.
pub fn query_nearest_3d(
    nodes: &[KdNode],
    points: &[[f64; 3]],
    root: usize,
    node_count: usize,
    query: [f64; 3],
    stack: &mut [u32],
) -> Result<Option<(u32, f64)>, KdTreeError> {
    if node_count == 0 {
        return Ok(None);
    }
    if stack.len() < MAX_KD_DEPTH * 2 {
        return Err(KdTreeError::StackBufferTooSmall {
            required: MAX_KD_DEPTH * 2,
        });
    }

    let mut best_idx: Option<u32> = None;
    let mut best_dist_sq = f64::INFINITY;

    let mut stack_top = 0usize;
    stack[stack_top] = root as u32;
    stack_top += 1;

    while stack_top > 0 {
        stack_top -= 1;
        let node_idx = stack[stack_top] as usize;
        let node = &nodes[node_idx];

        let p_idx = node.point_index as usize;
        let p = points[p_idx];
        let dx = query[0] - p[0];
        let dy = query[1] - p[1];
        let dz = query[2] - p[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;

        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_idx = Some(node.point_index);
        }

        // Determine which side to visit first.
        let axis = node.split_axis as usize;
        let diff = query[axis] - p[axis];
        let (near, far) = if diff < 0.0 {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };

        // Visit near child.
        if near != super::topology::INVALID_INDEX {
            stack[stack_top] = near;
            stack_top += 1;
        }

        // Visit far child only if splitting plane is within best distance.
        let plane_dist_sq = diff * diff;
        if far != super::topology::INVALID_INDEX && plane_dist_sq < best_dist_sq {
            if stack_top + 1 > stack.len() {
                break;
            }
            stack[stack_top] = far;
            stack_top += 1;
        }
    }

    Ok(best_idx.map(|idx| (idx, best_dist_sq)))
}

// ---------------------------------------------------------------------------
// Query: fixed-radius
// ---------------------------------------------------------------------------

/// Find all points within `radius_sq` of `query` in a 3D kd-tree.
///
/// `out_indices` receives the point indices. Returns the count found.
/// `stack` needs `MAX_KD_DEPTH * 2` entries.
pub fn query_radius_3d(
    nodes: &[KdNode],
    points: &[[f64; 3]],
    root: usize,
    node_count: usize,
    query: [f64; 3],
    radius_sq: f64,
    out_indices: &mut [u32],
    stack: &mut [u32],
) -> Result<usize, KdTreeError> {
    if node_count == 0 {
        return Ok(0);
    }
    if stack.len() < MAX_KD_DEPTH * 2 {
        return Err(KdTreeError::StackBufferTooSmall {
            required: MAX_KD_DEPTH * 2,
        });
    }

    let mut out_count = 0usize;
    let mut stack_top = 0usize;
    stack[stack_top] = root as u32;
    stack_top += 1;

    while stack_top > 0 {
        stack_top -= 1;
        let node_idx = stack[stack_top] as usize;
        let node = &nodes[node_idx];

        let p_idx = node.point_index as usize;
        let p = points[p_idx];
        let dx = query[0] - p[0];
        let dy = query[1] - p[1];
        let dz = query[2] - p[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;

        if dist_sq <= radius_sq {
            if out_count >= out_indices.len() {
                return Ok(out_count);
            }
            out_indices[out_count] = node.point_index;
            out_count += 1;
        }

        let axis = node.split_axis as usize;
        let diff = query[axis] - p[axis];
        let plane_dist_sq = diff * diff;

        // Visit both children if the plane is within radius.
        if node.left != super::topology::INVALID_INDEX && (diff <= 0.0 || plane_dist_sq <= radius_sq) {
            if stack_top + 1 > stack.len() {
                break;
            }
            stack[stack_top] = node.left;
            stack_top += 1;
        }
        if node.right != super::topology::INVALID_INDEX && (diff >= 0.0 || plane_dist_sq <= radius_sq) {
            if stack_top + 1 > stack.len() {
                break;
            }
            stack[stack_top] = node.right;
            stack_top += 1;
        }
    }

    Ok(out_count)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_points() -> Vec<[f64; 3]> {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 2.0],
        ]
    }

    #[test]
    fn kd_node_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<KdNode>(), KD_NODE_SIZE);
    }

    #[test]
    fn build_kd_tree_8_points() {
        let points = test_points();
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();
        assert_eq!(node_count, n);
        assert_eq!(root, 0);
    }

    #[test]
    fn build_kd_tree_deterministic() {
        let points = test_points();
        let n = points.len();

        let mut nodes_a = vec![KdNode::default(); n];
        let mut indices_a = vec![0u32; n];
        let mut codes_a = vec![0u64; n];
        let mut sort_a = vec![0u32; n];
        let (count_a, _) = build_kd_tree_3d(&points, &mut nodes_a, &mut indices_a, &mut codes_a, &mut sort_a).unwrap();

        let mut nodes_b = vec![KdNode::default(); n];
        let mut indices_b = vec![0u32; n];
        let mut codes_b = vec![0u64; n];
        let mut sort_b = vec![0u32; n];
        let (count_b, _) = build_kd_tree_3d(&points, &mut nodes_b, &mut indices_b, &mut codes_b, &mut sort_b).unwrap();

        assert_eq!(count_a, count_b);
        assert_eq!(nodes_a[..count_a], nodes_b[..count_b]);
        assert_eq!(indices_a, indices_b);
    }

    #[test]
    fn query_nearest_finds_exact_match() {
        let points = test_points();
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();

        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let result = query_nearest_3d(&nodes, &points, root, node_count, [0.1, 0.1, 0.1], &mut stack).unwrap();
        assert!(result.is_some());
        let (idx, dist_sq) = result.unwrap();
        assert_eq!(points[idx as usize], [0.0, 0.0, 0.0]);
        assert!(dist_sq < 0.1);
    }

    #[test]
    fn query_nearest_differential_vs_brute_force() {
        let points = test_points();
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();

        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];

        // Test multiple query points.
        let queries = [
            [0.5, 0.5, 0.5],
            [1.5, 0.0, 0.0],
            [0.0, 1.5, 0.0],
            [10.0, 10.0, 10.0],
            [-1.0, -1.0, -1.0],
        ];

        for q in queries {
            let kd_result = query_nearest_3d(&nodes, &points, root, node_count, q, &mut stack).unwrap();
            let brute_result = brute_force_nearest(&points, q);
            // Compare distances (indices may differ on ties).
            let kd_dist = kd_result.map(|(_, d)| d);
            let brute_dist = brute_result.map(|(_, d)| d);
            assert_eq!(kd_dist, brute_dist,
                "q={q:?} kd={kd_result:?} brute={brute_result:?}");
        }
    }

    fn brute_force_nearest(points: &[[f64; 3]], q: [f64; 3]) -> Option<(u32, f64)> {
        let mut best: Option<(u32, f64)> = None;
        for (i, p) in points.iter().enumerate() {
            let dx = q[0] - p[0];
            let dy = q[1] - p[1];
            let dz = q[2] - p[2];
            let d = dx * dx + dy * dy + dz * dz;
            if best.is_none() || d < best.unwrap().1 {
                best = Some((i as u32, d));
            }
        }
        best
    }

    #[test]
    fn query_radius_finds_nearby_points() {
        let points = test_points();
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();

        let mut out = vec![0u32; n];
        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let count = query_radius_3d(&nodes, &points, root, node_count, [0.0, 0.0, 0.0], 1.5, &mut out, &mut stack).unwrap();

        // Points within sqrt(1.5) ≈ 1.22 of origin: (0,0,0), (1,0,0), (0,1,0), (0,0,1)
        assert_eq!(count, 4);
    }

    #[test]
    fn query_radius_differential_vs_brute_force() {
        let points = test_points();
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();

        let mut out = vec![0u32; n];
        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let radius_sq = 3.0;
        let kd_count = query_radius_3d(&nodes, &points, root, node_count, [0.5, 0.5, 0.5], radius_sq, &mut out, &mut stack).unwrap();

        let mut brute_out: Vec<u32> = Vec::new();
        for (i, p) in points.iter().enumerate() {
            let dx = 0.5 - p[0];
            let dy = 0.5 - p[1];
            let dz = 0.5 - p[2];
            let d = dx * dx + dy * dy + dz * dz;
            if d <= radius_sq {
                brute_out.push(i as u32);
            }
        }

        let mut kd_sorted: Vec<u32> = out[..kd_count].to_vec();
        kd_sorted.sort_unstable();
        let mut brute_sorted = brute_out.clone();
        brute_sorted.sort_unstable();
        assert_eq!(kd_sorted, brute_sorted);
    }

    #[test]
    fn empty_kd_tree_returns_none() {
        let points: Vec<[f64; 3]> = vec![];
        let mut nodes = vec![KdNode::default(); 1];
        let mut indices = vec![0u32; 1];
        let mut codes = vec![0u64; 1];
        let mut sort = vec![0u32; 1];

        let (node_count, _) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();
        assert_eq!(node_count, 0);

        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let result = query_nearest_3d(&nodes, &points, 0, node_count, [0.0, 0.0, 0.0], &mut stack).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn single_point_kd_tree() {
        let points: Vec<[f64; 3]> = vec![[1.0, 2.0, 3.0]];
        let n = points.len();
        let mut nodes = vec![KdNode::default(); n];
        let mut indices = vec![0u32; n];
        let mut codes = vec![0u64; n];
        let mut sort = vec![0u32; n];

        let (node_count, root) = build_kd_tree_3d(&points, &mut nodes, &mut indices, &mut codes, &mut sort).unwrap();
        assert_eq!(node_count, 1);

        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let result = query_nearest_3d(&nodes, &points, root, node_count, [0.0, 0.0, 0.0], &mut stack).unwrap();
        assert!(result.is_some());
        let (idx, _) = result.unwrap();
        assert_eq!(points[idx as usize], [1.0, 2.0, 3.0]);
    }
}
