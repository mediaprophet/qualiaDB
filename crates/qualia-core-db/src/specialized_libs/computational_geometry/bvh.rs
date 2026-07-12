//! Static BVH (Bounding Volume Hierarchy) / AABB-tree builder and traversal.
//!
//! Provides a deterministic, caller-buffered BVH over 3D AABBs built by
//! Morton-code spatial sorting followed by a median split. (This is not a
//! Surface Area Heuristic build — no per-split surface-area cost is evaluated;
//! see `build_recursive`, which splits each range at its Morton-sorted median.
//! A true SAH build remains a possible future improvement.)
//! The tree is built once and queried many times — suitable for static scenes.
//!
//! ## Layout
//!
//! The BVH is stored as a flat array of nodes. Internal nodes store two child
//! indices; leaf nodes store a range of primitive indices. The node array is
//! `repr(C)` and POD for direct GPU staging.
//!
//! ## Determinism
//!
//! The builder sorts primitives by Morton code, then recursively partitions
//! each range at its median. Two builds from the same input produce
//! byte-identical node arrays.

use bytemuck::{Pod, Zeroable};

use super::distance::Aabb;
use super::spatial_order::sort_by_morton_3d;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the BVH builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvhError {
    /// `nodes` buffer too small.
    NodeBufferTooSmall { required: usize },
    /// `prim_indices` buffer too small (needs `primitive_count` entries).
    IndexBufferTooSmall { required: usize },
    /// `morton_codes` buffer too small.
    CodeBufferTooSmall { required: usize },
    /// `sort_indices` buffer too small.
    SortBufferTooSmall { required: usize },
    /// A bounding box is invalid (min > max).
    InvalidAabb { index: usize },
}

// ---------------------------------------------------------------------------
// POD node
// ---------------------------------------------------------------------------

/// BVH node: 48 bytes, `repr(C)`, naturally aligned.
///
/// ```text
/// offset  size  field
/// 0       12    bbox_min:[f32;3]
/// 12      12    bbox_max:[f32;3]
/// 24      4     left_or_first:u32    (internal: left child index; leaf: first prim index)
/// 28      4     right_or_count:u32  (internal: right child index; leaf: prim count)
/// 32      4     parent:u32          (parent node index, or INVALID_INDEX)
/// 36      4     node_type:u32       (0 = internal, 1 = leaf)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct BvhNode {
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub left_or_first: u32,
    pub right_or_count: u32,
    pub parent: u32,
    pub node_type: u32,
}

impl Default for BvhNode {
    fn default() -> Self {
        Self {
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            left_or_first: 0,
            right_or_count: 0,
            parent: super::topology::INVALID_INDEX,
            node_type: 0,
        }
    }
}

/// Node size in bytes.
pub const BVH_NODE_SIZE: usize = 40;

/// Maximum BVH depth (bounded by 2^21 primitives → 21 levels).
pub const MAX_BVH_DEPTH: usize = 32;

/// Leaf size: maximum primitives per leaf before splitting.
const LEAF_SIZE: usize = 4;

impl From<super::spatial_order::SpatialOrderError> for BvhError {
    fn from(err: super::spatial_order::SpatialOrderError) -> Self {
        match err {
            super::spatial_order::SpatialOrderError::CodeBufferTooSmall { required } => {
                BvhError::CodeBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::IndexBufferTooSmall { required } => {
                BvhError::SortBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::NonFiniteCoordinate { index } => {
                BvhError::InvalidAabb { index }
            }
        }
    }
}

/// Recursive BVH build (simpler and correct).
///
/// Returns the node index and the number of nodes used.
fn build_recursive(
    primitives: &[Aabb],
    prim_indices: &mut [u32],
    start: usize,
    count: usize,
    nodes: &mut [BvhNode],
    next_node: &mut usize,
    parent: u32,
) -> usize {
    let node_idx = *next_node;
    *next_node += 1;

    // Compute bounding box.
    let mut bbox = Aabb::new(
        primitives[prim_indices[start] as usize].min,
        primitives[prim_indices[start] as usize].max,
    );
    for i in 1..count {
        let p_idx = prim_indices[start + i] as usize;
        bbox = bbox.union(&primitives[p_idx]);
    }

    if count <= LEAF_SIZE {
        nodes[node_idx] = BvhNode {
            bbox_min: [bbox.min.x as f32, bbox.min.y as f32, bbox.min.z as f32],
            bbox_max: [bbox.max.x as f32, bbox.max.y as f32, bbox.max.z as f32],
            left_or_first: start as u32,
            right_or_count: count as u32,
            parent,
            node_type: 1,
        };
        return 1;
    }

    // Internal node: split at median (Morton-sorted).
    let mid = start + count / 2;

    // Build left subtree first (gets node_idx + 1).
    let left_child = *next_node;
    let left_size = build_recursive(
        primitives,
        prim_indices,
        start,
        count / 2,
        nodes,
        next_node,
        node_idx as u32,
    );

    // Build right subtree.
    let right_child = *next_node;
    let right_size = build_recursive(
        primitives,
        prim_indices,
        mid,
        count - count / 2,
        nodes,
        next_node,
        node_idx as u32,
    );

    nodes[node_idx] = BvhNode {
        bbox_min: [bbox.min.x as f32, bbox.min.y as f32, bbox.min.z as f32],
        bbox_max: [bbox.max.x as f32, bbox.max.y as f32, bbox.max.z as f32],
        left_or_first: left_child as u32,
        right_or_count: right_child as u32,
        parent,
        node_type: 0,
    };

    1 + left_size + right_size
}

/// Build a BVH using the recursive builder (correct, deterministic).
///
/// Caller-supplied buffers:
/// - `nodes`: needs up to `2 * primitives.len()` entries.
/// - `prim_indices`: needs `primitives.len()` entries (reordered by Morton code).
/// - `morton_codes`: needs `primitives.len()` entries (scratch).
/// - `sort_indices`: needs `primitives.len()` entries (scratch).
///
/// Returns (node_count, root_index). Root is always 0.
pub fn build_bvh_recursive(
    primitives: &[Aabb],
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    morton_codes: &mut [u64],
    sort_indices: &mut [u32],
) -> Result<(usize, usize), BvhError> {
    let n = primitives.len();
    if n == 0 {
        return Ok((0, 0));
    }

    // Validate AABBs.
    for (i, aabb) in primitives.iter().enumerate() {
        if aabb.min.x > aabb.max.x || aabb.min.y > aabb.max.y || aabb.min.z > aabb.max.z {
            return Err(BvhError::InvalidAabb { index: i });
        }
    }

    if morton_codes.len() < n {
        return Err(BvhError::CodeBufferTooSmall { required: n });
    }
    if sort_indices.len() < n {
        return Err(BvhError::SortBufferTooSmall { required: n });
    }
    if prim_indices.len() < n {
        return Err(BvhError::IndexBufferTooSmall { required: n });
    }

    let max_nodes = 2 * n - 1;
    if nodes.len() < max_nodes {
        return Err(BvhError::NodeBufferTooSmall {
            required: max_nodes,
        });
    }

    // Compute centroids for Morton sorting.
    let centroids: Vec<[f64; 3]> = primitives
        .iter()
        .map(|aabb| {
            [
                0.5 * (aabb.min.x + aabb.max.x) as f64,
                0.5 * (aabb.min.y + aabb.max.y) as f64,
                0.5 * (aabb.min.z + aabb.max.z) as f64,
            ]
        })
        .collect();

    // Sort by Morton code.
    sort_by_morton_3d(&centroids, morton_codes, sort_indices)?;
    prim_indices[..n].copy_from_slice(&sort_indices[..n]);

    // Build recursively.
    let mut next_node = 0usize;
    let node_count = build_recursive(
        primitives,
        prim_indices,
        0,
        n,
        nodes,
        &mut next_node,
        super::topology::INVALID_INDEX,
    );

    Ok((node_count, 0))
}

// ---------------------------------------------------------------------------
// Traversal: query AABB overlap
// ---------------------------------------------------------------------------

/// Query the BVH for all primitives whose AABB overlaps `query_bbox`.
///
/// `out_indices` receives the primitive indices. Returns the count found.
/// `stack` is scratch space (needs `MAX_BVH_DEPTH` entries).
///
/// Zero-heap. Deterministic (traversal order is left-to-right).
pub fn query_overlap(
    nodes: &[BvhNode],
    primitives: &[Aabb],
    prim_indices: &[u32],
    root: usize,
    node_count: usize,
    query_bbox: &Aabb,
    out_indices: &mut [u32],
    stack: &mut [u32],
) -> Result<usize, BvhError> {
    if node_count == 0 {
        return Ok(0);
    }
    if stack.len() < MAX_BVH_DEPTH * 2 {
        return Err(BvhError::SortBufferTooSmall {
            required: MAX_BVH_DEPTH * 2,
        });
    }

    let mut out_count = 0;
    let mut stack_top = 0usize;
    stack[stack_top] = root as u32;
    stack_top += 1;

    while stack_top > 0 {
        stack_top -= 1;
        let node_idx = stack[stack_top] as usize;

        let node = &nodes[node_idx];
        let node_bbox = Aabb::new(
            crate::specialized_libs::computational_geometry::Point3::new(
                node.bbox_min[0] as f64,
                node.bbox_min[1] as f64,
                node.bbox_min[2] as f64,
            ),
            crate::specialized_libs::computational_geometry::Point3::new(
                node.bbox_max[0] as f64,
                node.bbox_max[1] as f64,
                node.bbox_max[2] as f64,
            ),
        );

        if !node_bbox.overlaps(query_bbox) {
            continue;
        }

        // Leaf: node_type == 1. Internal: node_type == 0.
        if node.node_type == 1 {
            // Leaf node: check each primitive's AABB.
            let start = node.left_or_first as usize;
            let count = node.right_or_count as usize;
            for i in 0..count {
                if out_count >= out_indices.len() {
                    return Ok(out_count);
                }
                let p_idx = prim_indices[start + i] as usize;
                if primitives[p_idx].overlaps(query_bbox) {
                    out_indices[out_count] = prim_indices[start + i];
                    out_count += 1;
                }
            }
        } else {
            // Internal node: push children.
            if stack_top + 2 > stack.len() {
                break;
            }
            stack[stack_top] = node.right_or_count; // right child
            stack_top += 1;
            stack[stack_top] = node.left_or_first; // left child
            stack_top += 1;
        }
    }

    Ok(out_count)
}

/// Query the BVH for the closest primitive to a point.
///
/// Uses distance pruning. Returns the primitive index and squared distance,
/// or `None` if the tree is empty.
///
/// `stack` needs `MAX_BVH_DEPTH * 2` entries.
pub fn query_closest(
    nodes: &[BvhNode],
    primitives: &[Aabb],
    prim_indices: &[u32],
    root: usize,
    node_count: usize,
    point: super::primitives::Point3,
    stack: &mut [u32],
) -> Result<Option<(u32, f64)>, BvhError> {
    if node_count == 0 {
        return Ok(None);
    }
    if stack.len() < MAX_BVH_DEPTH * 2 {
        return Err(BvhError::SortBufferTooSmall {
            required: MAX_BVH_DEPTH * 2,
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

        let node_bbox = Aabb::new(
            super::primitives::Point3::new(
                node.bbox_min[0] as f64,
                node.bbox_min[1] as f64,
                node.bbox_min[2] as f64,
            ),
            super::primitives::Point3::new(
                node.bbox_max[0] as f64,
                node.bbox_max[1] as f64,
                node.bbox_max[2] as f64,
            ),
        );

        // Prune: if node bbox is farther than best, skip.
        let node_dist_sq = node_bbox.distance_sq_to_point(point);
        if node_dist_sq > best_dist_sq {
            continue;
        }

        if node.node_type == 1 {
            // Leaf node.
            let start = node.left_or_first as usize;
            let count = node.right_or_count as usize;
            for i in 0..count {
                let p_idx = prim_indices[start + i] as usize;
                let dist_sq = primitives[p_idx].distance_sq_to_point(point);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_idx = Some(prim_indices[start + i]);
                }
            }
        } else {
            // Internal: push both children.
            if stack_top + 2 > stack.len() {
                break;
            }
            stack[stack_top] = node.right_or_count;
            stack_top += 1;
            stack[stack_top] = node.left_or_first;
            stack_top += 1;
        }
    }

    Ok(best_idx.map(|idx| (idx, best_dist_sq)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::primitives::Point3;
    use super::*;

    fn make_aabb(min: [f64; 3], max: [f64; 3]) -> Aabb {
        Aabb::new(
            Point3::new(min[0], min[1], min[2]),
            Point3::new(max[0], max[1], max[2]),
        )
    }

    fn unit_cubes() -> Vec<Aabb> {
        // 8 unit cubes in a 2×2×2 grid.
        let mut prims = Vec::new();
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    let min = [x as f64, y as f64, z as f64];
                    let max = [(x + 1) as f64, (y + 1) as f64, (z + 1) as f64];
                    prims.push(make_aabb(min, max));
                }
            }
        }
        prims
    }

    #[test]
    fn bvh_node_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<BvhNode>(), BVH_NODE_SIZE);
        assert_eq!(std::mem::align_of::<BvhNode>(), 4);
    }

    #[test]
    fn build_bvh_8_cubes() {
        let prims = unit_cubes();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        assert!(node_count > 0);
        assert_eq!(root, 0);
        // 8 prims with LEAF_SIZE=4: root + 2 leaves = 3 nodes.
        assert_eq!(node_count, 3);
    }

    #[test]
    fn build_bvh_deterministic() {
        let prims = unit_cubes();
        let n = prims.len();

        let mut nodes_a = vec![BvhNode::default(); 2 * n];
        let mut indices_a = vec![0u32; n];
        let mut codes_a = vec![0u64; n];
        let mut sort_a = vec![0u32; n];
        let (count_a, _) = build_bvh_recursive(
            &prims,
            &mut nodes_a,
            &mut indices_a,
            &mut codes_a,
            &mut sort_a,
        )
        .unwrap();

        let mut nodes_b = vec![BvhNode::default(); 2 * n];
        let mut indices_b = vec![0u32; n];
        let mut codes_b = vec![0u64; n];
        let mut sort_b = vec![0u32; n];
        let (count_b, _) = build_bvh_recursive(
            &prims,
            &mut nodes_b,
            &mut indices_b,
            &mut codes_b,
            &mut sort_b,
        )
        .unwrap();

        assert_eq!(count_a, count_b);
        assert_eq!(nodes_a[..count_a], nodes_b[..count_b]);
        assert_eq!(indices_a, indices_b);
    }

    #[test]
    fn query_overlap_finds_correct_primitives() {
        let prims = unit_cubes();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        // Query: a box that covers only the (0,0,0) cube.
        let query = make_aabb([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
        let mut out = vec![0u32; n];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let count = query_overlap(
            &nodes,
            &prims,
            &prim_indices,
            root,
            node_count,
            &query,
            &mut out,
            &mut stack,
        )
        .unwrap();

        assert_eq!(count, 1);
        // The found primitive should be the one at (0,0,0).
        let found_prim = prims[out[0] as usize];
        assert_eq!(found_prim.min, Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn query_overlap_all() {
        let prims = unit_cubes();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        // Query: a box that covers everything.
        let query = make_aabb([-10.0, -10.0, -10.0], [10.0, 10.0, 10.0]);
        let mut out = vec![0u32; n];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let count = query_overlap(
            &nodes,
            &prims,
            &prim_indices,
            root,
            node_count,
            &query,
            &mut out,
            &mut stack,
        )
        .unwrap();

        assert_eq!(count, n);
    }

    #[test]
    fn query_closest_finds_nearest() {
        let prims = unit_cubes();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        let point = Point3::new(0.1, 0.1, 0.1);
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let result = query_closest(
            &nodes,
            &prims,
            &prim_indices,
            root,
            node_count,
            point,
            &mut stack,
        )
        .unwrap();

        assert!(result.is_some());
        let (idx, dist_sq) = result.unwrap();
        // The closest cube should be at (0,0,0).
        let prim = &prims[idx as usize];
        assert_eq!(prim.min, Point3::new(0.0, 0.0, 0.0));
        // Point is inside the cube, so distance is 0.
        assert!(dist_sq < 1e-12);
    }

    #[test]
    fn query_closest_outside() {
        let prims = unit_cubes();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        // Point far from all cubes.
        let point = Point3::new(5.0, 5.0, 5.0);
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let result = query_closest(
            &nodes,
            &prims,
            &prim_indices,
            root,
            node_count,
            point,
            &mut stack,
        )
        .unwrap();

        assert!(result.is_some());
        let (_idx, dist_sq) = result.unwrap();
        // Closest cube is at (1,1,1), distance from (5,5,5) to (2,2,2) corner = sqrt(3*9)=sqrt(27)
        assert!((dist_sq - 27.0).abs() < 1e-10, "dist_sq={dist_sq}");
    }

    #[test]
    fn differential_vs_brute_force_overlap() {
        // Random-ish AABBs.
        let prims: Vec<Aabb> = (0..20)
            .map(|i| {
                let x = (i % 5) as f64;
                let y = ((i / 5) % 4) as f64;
                let z = (i / 20) as f64;
                make_aabb([x, y, z], [x + 0.8, y + 0.8, z + 0.8])
            })
            .collect();
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        let query = make_aabb([1.5, 1.5, 0.0], [3.5, 2.5, 1.0]);
        let mut bvh_out = vec![0u32; n];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let bvh_count = query_overlap(
            &nodes,
            &prims,
            &prim_indices,
            root,
            node_count,
            &query,
            &mut bvh_out,
            &mut stack,
        )
        .unwrap();

        // Brute force.
        let mut brute_out: Vec<u32> = Vec::new();
        for (i, p) in prims.iter().enumerate() {
            if p.overlaps(&query) {
                brute_out.push(i as u32);
            }
        }

        // Compare as sets.
        let mut bvh_sorted: Vec<u32> = bvh_out[..bvh_count].to_vec();
        bvh_sorted.sort_unstable();
        let mut brute_sorted = brute_out.clone();
        brute_sorted.sort_unstable();
        assert_eq!(bvh_sorted, brute_sorted, "BVH and brute force must agree");
    }

    #[test]
    fn empty_bvh_returns_empty() {
        let prims: Vec<Aabb> = vec![];
        let mut nodes = vec![BvhNode::default(); 1];
        let mut prim_indices = vec![0u32; 1];
        let mut morton_codes = vec![0u64; 1];
        let mut sort_indices = vec![0u32; 1];

        let (node_count, _) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        assert_eq!(node_count, 0);
    }

    #[test]
    fn single_primitive_bvh() {
        let prims = vec![make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])];
        let n = prims.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];

        let (node_count, root) = build_bvh_recursive(
            &prims,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )
        .unwrap();

        assert_eq!(node_count, 1);
        let node = &nodes[root];
        assert_eq!(node.right_or_count, 1); // leaf with 1 primitive
    }
}
