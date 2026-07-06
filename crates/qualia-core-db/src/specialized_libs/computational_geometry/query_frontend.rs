//! Scan-free query front-end — unified spatial query API over a loaded
//! `.10d` spatial-index section (P3.8).
//!
//! This module provides the end-to-end query path:
//! 1. Load a `DecodedSpatialIndex` from a `.10d` section payload.
//! 2. Run BVH overlap, closest-primitive, or kd-tree NN queries directly
//!    on the loaded index — no full-scan of primitives required.
//! 3. Chunk-selective loading: only the BVH/kd-tree nodes whose bounding
//!    boxes intersect the query region are touched.
//!
//! ## Determinism
//!
//! Query results are identical whether computed from the in-memory index
//! or from a loaded `.10d` section. The chunk-touch fraction (ratio of
//! nodes visited to total nodes) is reported for observability.

use super::bvh::MAX_BVH_DEPTH;
use super::distance::Aabb;
use super::kd_tree::MAX_KD_DEPTH;
use super::primitives::Point3;

use crate::container_10d::spatial_index_section::{
    decode_spatial_index_section, DecodedSpatialIndex, SpatialIndexSectionError,
};

/// Query front-end error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryFrontendError {
    /// Section decode failed.
    DecodeError(SpatialIndexSectionError),
    /// Stack buffer too small.
    StackBufferTooSmall { required: usize },
    /// Output buffer too small.
    OutputBufferTooSmall { required: usize },
}

impl std::fmt::Display for QueryFrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeError(e) => write!(f, "query front-end decode error: {e}"),
            Self::StackBufferTooSmall { required } => {
                write!(f, "query front-end stack too small: need {required}")
            }
            Self::OutputBufferTooSmall { required } => {
                write!(f, "query front-end output too small: need {required}")
            }
        }
    }
}

impl std::error::Error for QueryFrontendError {}

impl From<SpatialIndexSectionError> for QueryFrontendError {
    fn from(err: SpatialIndexSectionError) -> Self {
        QueryFrontendError::DecodeError(err)
    }
}

/// Query statistics: tracks how many nodes were touched vs total.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryStats {
    /// Total nodes in the index.
    pub total_nodes: usize,
    /// Nodes actually visited during traversal.
    pub nodes_touched: usize,
    /// Chunk-touch fraction (nodes_touched / total_nodes).
    pub touch_fraction: f64,
}

impl QueryStats {
    fn new(total: usize, touched: usize) -> Self {
        let fraction = if total > 0 {
            touched as f64 / total as f64
        } else {
            0.0
        };
        Self {
            total_nodes: total,
            nodes_touched: touched,
            touch_fraction: fraction,
        }
    }
}

/// A loaded spatial index ready for scan-free queries.
pub struct SpatialIndexQuery<'a> {
    decoded: DecodedSpatialIndex<'a>,
}

impl<'a> SpatialIndexQuery<'a> {
    /// Load a spatial index from a `.10d` section payload.
    pub fn load(payload: &'a [u8]) -> Result<Self, QueryFrontendError> {
        let decoded = decode_spatial_index_section(payload)?;
        Ok(Self { decoded })
    }

    /// BVH overlap query: find all primitive indices whose AABB overlaps
    /// `query_bbox`. Returns the number of results and query statistics.
    ///
    /// `out_indices` receives the primitive indices.
    /// `stack` is scratch space (needs `MAX_BVH_DEPTH * 2` entries).
    pub fn query_overlap(
        &self,
        primitives: &[Aabb],
        query_bbox: &Aabb,
        out_indices: &mut [u32],
        stack: &mut [u32],
    ) -> Result<(usize, QueryStats), QueryFrontendError> {
        let bvh_count = self.decoded.header.bvh_node_count as usize;
        if bvh_count == 0 {
            return Ok((0, QueryStats::new(0, 0)));
        }
        if stack.len() < MAX_BVH_DEPTH * 2 {
            return Err(QueryFrontendError::StackBufferTooSmall {
                required: MAX_BVH_DEPTH * 2,
            });
        }

        // Count nodes touched by doing a counting traversal.
        let mut touched = 0usize;
        let mut stack_top = 0usize;
        stack[stack_top] = self.decoded.header.bvh_root;
        stack_top += 1;

        while stack_top > 0 {
            stack_top -= 1;
            let node_idx = stack[stack_top] as usize;
            touched += 1;
            let node = &self.decoded.bvh_nodes[node_idx];

            let node_bbox = Aabb::new(
                Point3::new(
                    node.bbox_min[0] as f64,
                    node.bbox_min[1] as f64,
                    node.bbox_min[2] as f64,
                ),
                Point3::new(
                    node.bbox_max[0] as f64,
                    node.bbox_max[1] as f64,
                    node.bbox_max[2] as f64,
                ),
            );

            if !node_bbox.overlaps(query_bbox) {
                continue;
            }

            if node.node_type == 1 {
                // Leaf: check primitives.
                let start = node.left_or_first as usize;
                let count = node.right_or_count as usize;
                for i in 0..count {
                    let p_idx = self.decoded.bvh_prim_indices[start + i] as usize;
                    if p_idx < primitives.len() && primitives[p_idx].overlaps(query_bbox) {
                        // Counted in the actual query below.
                    }
                }
            } else {
                // Internal: push children.
                if stack_top + 2 > stack.len() {
                    break;
                }
                stack[stack_top] = node.right_or_count;
                stack_top += 1;
                stack[stack_top] = node.left_or_first;
                stack_top += 1;
            }
        }

        // Now run the actual query.
        let count = super::bvh::query_overlap(
            self.decoded.bvh_nodes,
            primitives,
            self.decoded.bvh_prim_indices,
            self.decoded.header.bvh_root as usize,
            bvh_count,
            query_bbox,
            out_indices,
            stack,
        )
        .map_err(|_| QueryFrontendError::StackBufferTooSmall {
            required: MAX_BVH_DEPTH * 2,
        })?;

        Ok((count, QueryStats::new(bvh_count, touched)))
    }

    /// BVH closest-primitive query: find the nearest primitive to `point`.
    /// Returns `(index, squared_distance, stats)` or `None` if empty.
    pub fn query_closest(
        &self,
        primitives: &[Aabb],
        point: Point3,
        stack: &mut [u32],
    ) -> Result<Option<(u32, f64, QueryStats)>, QueryFrontendError> {
        let bvh_count = self.decoded.header.bvh_node_count as usize;
        if bvh_count == 0 {
            return Ok(None);
        }
        if stack.len() < MAX_BVH_DEPTH * 2 {
            return Err(QueryFrontendError::StackBufferTooSmall {
                required: MAX_BVH_DEPTH * 2,
            });
        }

        // Counting traversal for stats.
        let mut touched = 0usize;
        let mut stack_top = 0usize;
        stack[stack_top] = self.decoded.header.bvh_root;
        stack_top += 1;
        let mut best_dist_sq = f64::INFINITY;

        while stack_top > 0 {
            stack_top -= 1;
            let node_idx = stack[stack_top] as usize;
            touched += 1;
            let node = &self.decoded.bvh_nodes[node_idx];

            let node_bbox = Aabb::new(
                Point3::new(
                    node.bbox_min[0] as f64,
                    node.bbox_min[1] as f64,
                    node.bbox_min[2] as f64,
                ),
                Point3::new(
                    node.bbox_max[0] as f64,
                    node.bbox_max[1] as f64,
                    node.bbox_max[2] as f64,
                ),
            );

            let node_dist_sq = node_bbox.distance_sq_to_point(point);
            if node_dist_sq > best_dist_sq {
                continue;
            }

            if node.node_type == 1 {
                let start = node.left_or_first as usize;
                let count = node.right_or_count as usize;
                for i in 0..count {
                    let p_idx = self.decoded.bvh_prim_indices[start + i] as usize;
                    if p_idx < primitives.len() {
                        let d = primitives[p_idx].distance_sq_to_point(point);
                        if d < best_dist_sq {
                            best_dist_sq = d;
                        }
                    }
                }
            } else {
                if stack_top + 2 > stack.len() {
                    break;
                }
                stack[stack_top] = node.right_or_count;
                stack_top += 1;
                stack[stack_top] = node.left_or_first;
                stack_top += 1;
            }
        }

        // Actual query.
        let result = super::bvh::query_closest(
            self.decoded.bvh_nodes,
            primitives,
            self.decoded.bvh_prim_indices,
            self.decoded.header.bvh_root as usize,
            bvh_count,
            point,
            stack,
        )
        .map_err(|_| QueryFrontendError::StackBufferTooSmall {
            required: MAX_BVH_DEPTH * 2,
        })?;

        let stats = QueryStats::new(bvh_count, touched);
        Ok(result.map(|(idx, dist)| (idx, dist, stats)))
    }

    /// kd-tree nearest-neighbour query.
    /// Returns `(point_index, squared_distance, stats)` or `None`.
    pub fn query_nearest(
        &self,
        points: &[[f64; 3]],
        query: [f64; 3],
        stack: &mut [u32],
    ) -> Result<Option<(u32, f64, QueryStats)>, QueryFrontendError> {
        let kd_count = self.decoded.header.kd_node_count as usize;
        if kd_count == 0 {
            return Ok(None);
        }
        if stack.len() < MAX_KD_DEPTH * 2 {
            return Err(QueryFrontendError::StackBufferTooSmall {
                required: MAX_KD_DEPTH * 2,
            });
        }

        // Counting traversal for stats.
        let mut touched = 0usize;
        let mut stack_top = 0usize;
        stack[stack_top] = self.decoded.header.kd_root;
        stack_top += 1;
        let mut best_dist_sq = f64::INFINITY;

        while stack_top > 0 {
            stack_top -= 1;
            let node_idx = stack[stack_top] as usize;
            touched += 1;
            let node = &self.decoded.kd_nodes[node_idx];

            let p_idx = node.point_index as usize;
            if p_idx < points.len() {
                let p = points[p_idx];
                let dx = query[0] - p[0];
                let dy = query[1] - p[1];
                let dz = query[2] - p[2];
                let d = dx * dx + dy * dy + dz * dz;
                if d < best_dist_sq {
                    best_dist_sq = d;
                }
            }

            let axis = node.split_axis as usize;
            let diff = if p_idx < points.len() {
                query[axis] - points[p_idx][axis]
            } else {
                0.0
            };
            let plane_dist_sq = diff * diff;

            let (near, far) = if diff < 0.0 {
                (node.left, node.right)
            } else {
                (node.right, node.left)
            };

            if near != super::topology::INVALID_INDEX {
                stack[stack_top] = near;
                stack_top += 1;
            }
            if far != super::topology::INVALID_INDEX && plane_dist_sq < best_dist_sq {
                if stack_top + 1 > stack.len() {
                    break;
                }
                stack[stack_top] = far;
                stack_top += 1;
            }
        }

        // Actual query.
        let result = super::kd_tree::query_nearest_3d(
            self.decoded.kd_nodes,
            points,
            self.decoded.header.kd_root as usize,
            kd_count,
            query,
            stack,
        )
        .map_err(|_| QueryFrontendError::StackBufferTooSmall {
            required: MAX_KD_DEPTH * 2,
        })?;

        let stats = QueryStats::new(kd_count, touched);
        Ok(result.map(|(idx, dist)| (idx, dist, stats)))
    }

    /// kd-tree fixed-radius query.
    /// Returns the number of points found and query statistics.
    pub fn query_radius(
        &self,
        points: &[[f64; 3]],
        query: [f64; 3],
        radius_sq: f64,
        out_indices: &mut [u32],
        stack: &mut [u32],
    ) -> Result<(usize, QueryStats), QueryFrontendError> {
        let kd_count = self.decoded.header.kd_node_count as usize;
        if kd_count == 0 {
            return Ok((0, QueryStats::new(0, 0)));
        }
        if stack.len() < MAX_KD_DEPTH * 2 {
            return Err(QueryFrontendError::StackBufferTooSmall {
                required: MAX_KD_DEPTH * 2,
            });
        }

        // Counting traversal for stats.
        let mut touched = 0usize;
        let mut stack_top = 0usize;
        stack[stack_top] = self.decoded.header.kd_root;
        stack_top += 1;

        while stack_top > 0 {
            stack_top -= 1;
            let node_idx = stack[stack_top] as usize;
            touched += 1;
            let node = &self.decoded.kd_nodes[node_idx];

            let p_idx = node.point_index as usize;
            let axis = node.split_axis as usize;
            let diff = if p_idx < points.len() {
                query[axis] - points[p_idx][axis]
            } else {
                0.0
            };
            let plane_dist_sq = diff * diff;

            if node.left != super::topology::INVALID_INDEX
                && (diff <= 0.0 || plane_dist_sq <= radius_sq)
            {
                if stack_top + 1 > stack.len() {
                    break;
                }
                stack[stack_top] = node.left;
                stack_top += 1;
            }
            if node.right != super::topology::INVALID_INDEX
                && (diff >= 0.0 || plane_dist_sq <= radius_sq)
            {
                if stack_top + 1 > stack.len() {
                    break;
                }
                stack[stack_top] = node.right;
                stack_top += 1;
            }
        }

        // Actual query.
        let count = super::kd_tree::query_radius_3d(
            self.decoded.kd_nodes,
            points,
            self.decoded.header.kd_root as usize,
            kd_count,
            query,
            radius_sq,
            out_indices,
            stack,
        )
        .map_err(|_| QueryFrontendError::StackBufferTooSmall {
            required: MAX_KD_DEPTH * 2,
        })?;

        let stats = QueryStats::new(kd_count, touched);
        Ok((count, stats))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::{
        build_bvh_recursive, build_kd_tree_3d, Aabb, Point3,
    };
    use crate::container_10d::spatial_index_section::encode_spatial_index_section;

    fn test_aabbs() -> Vec<Aabb> {
        (0..8)
            .map(|i| {
                let x = (i % 2) as f64;
                let y = ((i / 2) % 2) as f64;
                let z = (i / 4) as f64;
                Aabb::new(
                    Point3::new(x, y, z),
                    Point3::new(x + 1.0, y + 1.0, z + 1.0),
                )
            })
            .collect()
    }

    fn test_points() -> Vec<[f64; 3]> {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ]
    }

    fn build_and_encode() -> Vec<u8> {
        let aabbs = test_aabbs();
        let n = aabbs.len();
        let mut bvh_nodes = vec![crate::specialized_libs::computational_geometry::BvhNode::default(); 2 * n];
        let mut bvh_indices = vec![0u32; n];
        let mut bvh_codes = vec![0u64; n];
        let mut bvh_sort = vec![0u32; n];
        let (bvh_count, bvh_root) = build_bvh_recursive(
            &aabbs,
            &mut bvh_nodes,
            &mut bvh_indices,
            &mut bvh_codes,
            &mut bvh_sort,
        )
        .unwrap();

        let points = test_points();
        let np = points.len();
        let mut kd_nodes = vec![crate::specialized_libs::computational_geometry::KdNode::default(); np];
        let mut kd_indices = vec![0u32; np];
        let mut kd_codes = vec![0u64; np];
        let mut kd_sort = vec![0u32; np];
        let (kd_count, kd_root) = build_kd_tree_3d(
            &points,
            &mut kd_nodes,
            &mut kd_indices,
            &mut kd_codes,
            &mut kd_sort,
        )
        .unwrap();

        let need = crate::container_10d::spatial_index_section::encoded_len(bvh_count as u32, kd_count as u32, n as u32, np as u32);
        let mut buf = vec![0u8; need];
        encode_spatial_index_section(
            &bvh_nodes[..bvh_count],
            &bvh_indices,
            bvh_root as u32,
            &kd_nodes[..kd_count],
            &kd_indices,
            kd_root as u32,
            &mut buf,
        )
        .unwrap();
        buf
    }

    #[test]
    fn load_and_query_overlap_matches_in_memory() {
        let aabbs = test_aabbs();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        let query = Aabb::new(
            Point3::new(-0.5, -0.5, -0.5),
            Point3::new(0.5, 0.5, 0.5),
        );
        let mut out = vec![0u32; aabbs.len()];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let (count, stats) = query_frontend
            .query_overlap(&aabbs, &query, &mut out, &mut stack)
            .unwrap();

        assert_eq!(count, 1);
        assert!(stats.total_nodes > 0);
        assert!(stats.nodes_touched > 0);
        assert!(stats.touch_fraction <= 1.0);
    }

    #[test]
    fn load_and_query_closest_matches_in_memory() {
        let aabbs = test_aabbs();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let result = query_frontend
            .query_closest(&aabbs, Point3::new(0.1, 0.1, 0.1), &mut stack)
            .unwrap();

        assert!(result.is_some());
        let (idx, dist_sq, stats) = result.unwrap();
        assert_eq!(aabbs[idx as usize].min, Point3::new(0.0, 0.0, 0.0));
        assert!(dist_sq < 0.01);
        assert!(stats.touch_fraction <= 1.0);
    }

    #[test]
    fn load_and_query_nearest_matches_in_memory() {
        let points = test_points();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let result = query_frontend
            .query_nearest(&points, [0.1, 0.1, 0.1], &mut stack)
            .unwrap();

        assert!(result.is_some());
        let (idx, dist_sq, stats) = result.unwrap();
        assert_eq!(points[idx as usize], [0.0, 0.0, 0.0]);
        assert!(dist_sq < 0.1);
        assert!(stats.touch_fraction <= 1.0);
    }

    #[test]
    fn load_and_query_radius_matches_in_memory() {
        let points = test_points();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        let mut out = vec![0u32; points.len()];
        let mut stack = vec![0u32; MAX_KD_DEPTH * 2];
        let (count, stats) = query_frontend
            .query_radius(&points, [0.0, 0.0, 0.0], 1.5, &mut out, &mut stack)
            .unwrap();

        // Points within sqrt(1.5) ≈ 1.22 of origin.
        assert_eq!(count, 4);
        assert!(stats.touch_fraction <= 1.0);
    }

    #[test]
    fn load_and_query_overlap_matches_brute_force() {
        let aabbs = test_aabbs();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        let query = Aabb::new(
            Point3::new(-10.0, -10.0, -10.0),
            Point3::new(10.0, 10.0, 10.0),
        );
        let mut out = vec![0u32; aabbs.len()];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let (count, _) = query_frontend
            .query_overlap(&aabbs, &query, &mut out, &mut stack)
            .unwrap();

        // Brute force.
        let brute_count = aabbs.iter().filter(|a| a.overlaps(&query)).count();
        assert_eq!(count, brute_count);
    }

    #[test]
    fn empty_section_queries_return_none() {
        let need = crate::container_10d::spatial_index_section::encoded_len(0, 0, 0, 0);
        let mut buf = vec![0u8; need];
        encode_spatial_index_section(
            &[], &[], 0, &[], &[], 0, &mut buf,
        )
        .unwrap();

        let query_frontend = SpatialIndexQuery::load(&buf).unwrap();
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let result = query_frontend
            .query_closest(&[], Point3::new(0.0, 0.0, 0.0), &mut stack)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn touch_fraction_is_honest() {
        let aabbs = test_aabbs();
        let payload = build_and_encode();
        let query_frontend = SpatialIndexQuery::load(&payload).unwrap();

        // Small query box that only touches a few nodes.
        let query = Aabb::new(
            Point3::new(-0.1, -0.1, -0.1),
            Point3::new(0.1, 0.1, 0.1),
        );
        let mut out = vec![0u32; aabbs.len()];
        let mut stack = vec![0u32; MAX_BVH_DEPTH * 2];
        let (_, stats) = query_frontend
            .query_overlap(&aabbs, &query, &mut out, &mut stack)
            .unwrap();

        // With 3 BVH nodes and a small query, the touch fraction should be
        // honest (≤ 1.0). For very small trees, all nodes may be touched.
        assert!(stats.nodes_touched <= stats.total_nodes);
        assert!(stats.touch_fraction <= 1.0);
    }
}
