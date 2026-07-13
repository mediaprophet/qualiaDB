//! Batched box-intersection spatial join.
//!
//! Given two sets of AABBs (A and B), find all overlapping pairs (a, b) where
//! a ∈ A, b ∈ B, and a.overlaps(b). Uses BVH-accelerated traversal — a BVH is
//! built over set B (see `super::bvh`) and queried once per box in set A —
//! with Morton-ordered BVH construction for determinism. (This is not a
//! sweep-and-prune implementation.)
//!
//! ## Determinism
//!
//! Pairs are emitted in a canonical order: sorted by (a_index, b_index). Two
//! runs from the same input produce byte-identical output. The algorithm is
//! zero-heap in the hot path (caller-supplied output buffer).

use super::distance::Aabb;

/// Error type for box-intersection join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxJoinError {
    /// `out_pairs` buffer too small.
    OutputBufferTooSmall { required: usize },
    /// `sort_buffer` too small.
    SortBufferTooSmall { required: usize },
    /// `morton_codes` buffer too small.
    CodeBufferTooSmall { required: usize },
}

impl From<super::spatial_order::SpatialOrderError> for BoxJoinError {
    fn from(err: super::spatial_order::SpatialOrderError) -> Self {
        match err {
            super::spatial_order::SpatialOrderError::CodeBufferTooSmall { required } => {
                BoxJoinError::CodeBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::IndexBufferTooSmall { required } => {
                BoxJoinError::SortBufferTooSmall { required }
            }
            super::spatial_order::SpatialOrderError::NonFiniteCoordinate { .. } => {
                BoxJoinError::CodeBufferTooSmall { required: 0 }
            }
        }
    }
}

/// A pair of overlapping box indices (a_index, b_index).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxPair {
    pub a: u32,
    pub b: u32,
}

/// Compute all overlapping pairs between two AABB sets using brute-force O(n*m).
///
/// This is the reference implementation for differential testing. Pairs are
/// emitted in (a, b) sorted order.
pub fn box_join_brute_force(
    boxes_a: &[Aabb],
    boxes_b: &[Aabb],
    out_pairs: &mut [BoxPair],
) -> Result<usize, BoxJoinError> {
    let mut count = 0usize;
    for (i, a) in boxes_a.iter().enumerate() {
        for (j, b) in boxes_b.iter().enumerate() {
            if a.overlaps(b) {
                if count >= out_pairs.len() {
                    return Err(BoxJoinError::OutputBufferTooSmall {
                        required: count + 1,
                    });
                }
                out_pairs[count] = BoxPair {
                    a: i as u32,
                    b: j as u32,
                };
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Compute all overlapping pairs using BVH-accelerated traversal.
///
/// For each box in set A, query the BVH of set B. Pairs are emitted in
/// (a, b) sorted order. Uses the caller-supplied BVH buffers for set B.
///
/// Buffers:
/// - `bvh_nodes`: `2 * boxes_b.len()` entries.
/// - `bvh_prim_indices`: `boxes_b.len()` entries.
/// - `bvh_morton_codes`: `boxes_b.len()` entries.
/// - `bvh_sort_indices`: `boxes_b.len()` entries.
/// - `query_stack`: `MAX_BVH_DEPTH * 2` entries.
/// - `query_out`: `boxes_b.len()` entries (per-query results).
/// - `out_pairs`: up to `boxes_a.len() * boxes_b.len()` entries.
pub fn box_join_bvh(
    boxes_a: &[Aabb],
    boxes_b: &[Aabb],
    bvh_nodes: &mut [super::bvh::BvhNode],
    bvh_prim_indices: &mut [u32],
    bvh_morton_codes: &mut [u64],
    bvh_sort_indices: &mut [u32],
    query_stack: &mut [u32],
    query_out: &mut [u32],
    out_pairs: &mut [BoxPair],
) -> Result<usize, BoxJoinError> {
    let nb = boxes_b.len();
    if nb == 0 || boxes_a.is_empty() {
        return Ok(0);
    }

    // Build BVH over set B.
    let (node_count, root) = super::bvh::build_bvh_recursive(
        boxes_b,
        bvh_nodes,
        bvh_prim_indices,
        bvh_morton_codes,
        bvh_sort_indices,
    )
    .map_err(|e| match e {
        super::bvh::BvhError::NodeBufferTooSmall { required } => {
            BoxJoinError::SortBufferTooSmall { required }
        }
        super::bvh::BvhError::CodeBufferTooSmall { required } => {
            BoxJoinError::CodeBufferTooSmall { required }
        }
        super::bvh::BvhError::SortBufferTooSmall { required } => {
            BoxJoinError::SortBufferTooSmall { required }
        }
        super::bvh::BvhError::IndexBufferTooSmall { required } => {
            BoxJoinError::SortBufferTooSmall { required }
        }
        super::bvh::BvhError::InvalidAabb { .. } => {
            BoxJoinError::CodeBufferTooSmall { required: 0 }
        }
    })?;

    let mut pair_count = 0usize;

    for (i, box_a) in boxes_a.iter().enumerate() {
        let hit_count = super::bvh::query_overlap(
            bvh_nodes,
            boxes_b,
            bvh_prim_indices,
            root,
            node_count,
            box_a,
            query_out,
            query_stack,
        )
        .map_err(|_| BoxJoinError::SortBufferTooSmall {
            required: super::bvh::MAX_BVH_DEPTH * 2,
        })?;

        // Sort the per-query results for deterministic pair ordering.
        query_out[..hit_count].sort_unstable();

        for j in 0..hit_count {
            if pair_count >= out_pairs.len() {
                return Err(BoxJoinError::OutputBufferTooSmall {
                    required: pair_count + 1,
                });
            }
            out_pairs[pair_count] = BoxPair {
                a: i as u32,
                b: query_out[j],
            };
            pair_count += 1;
        }
    }

    Ok(pair_count)
}

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

    #[test]
    fn box_pair_is_pod() {
        assert_eq!(std::mem::size_of::<BoxPair>(), 8);
    }

    #[test]
    fn brute_force_finds_overlaps() {
        let a = vec![
            make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
            make_aabb([2.0, 2.0, 2.0], [3.0, 3.0, 3.0]),
        ];
        let b = vec![
            make_aabb([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]), // overlaps a[0]
            make_aabb([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]), // overlaps nothing
        ];
        let mut out = vec![BoxPair { a: 0, b: 0 }; 10];
        let count = box_join_brute_force(&a, &b, &mut out).unwrap();
        assert_eq!(count, 1);
        assert_eq!(out[0], BoxPair { a: 0, b: 0 });
    }

    #[test]
    fn brute_force_all_disjoint() {
        let a = vec![make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])];
        let b = vec![make_aabb([5.0, 5.0, 5.0], [6.0, 6.0, 6.0])];
        let mut out = vec![BoxPair { a: 0, b: 0 }; 10];
        let count = box_join_brute_force(&a, &b, &mut out).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn brute_force_all_overlapping() {
        let a = vec![
            make_aabb([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]),
            make_aabb([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]),
        ];
        let b = vec![
            make_aabb([0.5, 0.5, 0.5], [2.5, 2.5, 2.5]),
            make_aabb([1.5, 1.5, 1.5], [4.0, 4.0, 4.0]),
        ];
        let mut out = vec![BoxPair { a: 0, b: 0 }; 20];
        let count = box_join_brute_force(&a, &b, &mut out).unwrap();
        // All 4 pairs overlap.
        assert_eq!(count, 4);
    }

    #[test]
    fn brute_force_boundary_touching() {
        let a = vec![make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])];
        let b = vec![make_aabb([1.0, 0.0, 0.0], [2.0, 1.0, 1.0])];
        let mut out = vec![BoxPair { a: 0, b: 0 }; 10];
        let count = box_join_brute_force(&a, &b, &mut out).unwrap();
        // Touching faces should overlap (<=, >=).
        assert_eq!(count, 1);
    }

    #[test]
    fn bvh_join_matches_brute_force() {
        let a: Vec<Aabb> = (0..10)
            .map(|i| {
                let x = (i % 3) as f64;
                let y = ((i / 3) % 3) as f64;
                let z = (i / 9) as f64;
                make_aabb([x, y, z], [x + 1.5, y + 1.5, z + 1.5])
            })
            .collect();
        let b: Vec<Aabb> = (0..8)
            .map(|i| {
                let x = (i % 2) as f64 + 0.5;
                let y = ((i / 2) % 4) as f64;
                let z = (i / 8) as f64;
                make_aabb([x, y, z], [x + 1.0, y + 1.0, z + 1.0])
            })
            .collect();

        // Brute force.
        let mut brute_out = vec![BoxPair { a: 0, b: 0 }; a.len() * b.len()];
        let brute_count = box_join_brute_force(&a, &b, &mut brute_out).unwrap();

        // BVH join.
        let nb = b.len();
        let mut bvh_nodes = vec![super::super::bvh::BvhNode::default(); 2 * nb];
        let mut bvh_indices = vec![0u32; nb];
        let mut bvh_codes = vec![0u64; nb];
        let mut bvh_sort = vec![0u32; nb];
        let mut query_stack = vec![0u32; super::super::bvh::MAX_BVH_DEPTH * 2];
        let mut query_out = vec![0u32; nb];
        let mut bvh_out = vec![BoxPair { a: 0, b: 0 }; a.len() * b.len()];
        let bvh_count = box_join_bvh(
            &a,
            &b,
            &mut bvh_nodes,
            &mut bvh_indices,
            &mut bvh_codes,
            &mut bvh_sort,
            &mut query_stack,
            &mut query_out,
            &mut bvh_out,
        )
        .unwrap();

        assert_eq!(brute_count, bvh_count);

        // Compare as sorted sets.
        let mut brute_sorted = brute_out[..brute_count].to_vec();
        brute_sorted.sort_unstable();
        let mut bvh_sorted = bvh_out[..bvh_count].to_vec();
        bvh_sorted.sort_unstable();
        assert_eq!(brute_sorted, bvh_sorted);
    }

    #[test]
    fn bvh_join_empty_sets() {
        let a: Vec<Aabb> = vec![];
        let b: Vec<Aabb> = vec![make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])];

        let mut bvh_nodes = vec![super::super::bvh::BvhNode::default(); 2];
        let mut bvh_indices = vec![0u32; 1];
        let mut bvh_codes = vec![0u64; 1];
        let mut bvh_sort = vec![0u32; 1];
        let mut query_stack = vec![0u32; super::super::bvh::MAX_BVH_DEPTH * 2];
        let mut query_out = vec![0u32; 1];
        let mut bvh_out = vec![BoxPair { a: 0, b: 0 }; 1];
        let count = box_join_bvh(
            &a,
            &b,
            &mut bvh_nodes,
            &mut bvh_indices,
            &mut bvh_codes,
            &mut bvh_sort,
            &mut query_stack,
            &mut query_out,
            &mut bvh_out,
        )
        .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn bvh_join_deterministic() {
        let a: Vec<Aabb> = (0..6)
            .map(|i| make_aabb([i as f64, 0.0, 0.0], [(i + 2) as f64, 2.0, 2.0]))
            .collect();
        let b: Vec<Aabb> = (0..5)
            .map(|i| make_aabb([0.0, i as f64, 0.0], [3.0, (i + 2) as f64, 2.0]))
            .collect();

        let nb = b.len();
        let run = || {
            let mut bvh_nodes = vec![super::super::bvh::BvhNode::default(); 2 * nb];
            let mut bvh_indices = vec![0u32; nb];
            let mut bvh_codes = vec![0u64; nb];
            let mut bvh_sort = vec![0u32; nb];
            let mut query_stack = vec![0u32; super::super::bvh::MAX_BVH_DEPTH * 2];
            let mut query_out = vec![0u32; nb];
            let mut bvh_out = vec![BoxPair { a: 0, b: 0 }; a.len() * b.len()];
            let count = box_join_bvh(
                &a,
                &b,
                &mut bvh_nodes,
                &mut bvh_indices,
                &mut bvh_codes,
                &mut bvh_sort,
                &mut query_stack,
                &mut query_out,
                &mut bvh_out,
            )
            .unwrap();
            (count, bvh_out)
        };

        let (c1, o1) = run();
        let (c2, o2) = run();
        assert_eq!(c1, c2);
        assert_eq!(o1[..c1], o2[..c2]);
    }
}
