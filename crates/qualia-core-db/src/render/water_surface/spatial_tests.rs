use super::*;
use crate::specialized_libs::computational_geometry::bvh::BvhNode;
use crate::specialized_libs::computational_geometry::{Aabb, Point3};

fn accelerated(p: &[[f32; 3]], indices: &[u32]) -> (Vec<f32>, Vec<f32>, ShoreReceipt) {
    let mut edges = vec![0; indices.len() + p.len()];
    let topology = shore_topology_into(p, indices, &mut edges).unwrap();
    let n = topology.boundary_edges();
    let nodes = 2 * n - 1;
    let mut bounds = vec![Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)); n];
    let mut tree = vec![BvhNode::default(); nodes];
    let mut order = vec![0; n];
    let mut morton = vec![0; n];
    let mut sorting = vec![0; n];
    let mut components = vec![0; nodes];
    let mut distance = vec![9.0; p.len()];
    let mut scales = vec![9.0; p.len()];
    let receipt = shore_distance_accelerated_into(
        p,
        &topology,
        &mut distance,
        &mut scales,
        ShoreSpatialScratch {
            bounds: &mut bounds,
            nodes: &mut tree,
            primitive_order: &mut order,
            morton: &mut morton,
            sort_order: &mut sorting,
            node_components: &mut components,
        },
    )
    .unwrap();
    (distance, scales, receipt)
}

#[test]
fn native_bvh_matches_reference_on_curved_bank_and_prunes_segment_work() {
    let side = 33usize;
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for z in 0..side {
        for x in 0..side {
            let xf = x as f32;
            let zf = z as f32;
            positions.push([
                xf + 0.8 * (zf * 0.22).sin(),
                0.0,
                zf + 0.5 * (xf * 0.17).sin(),
            ]);
        }
    }
    for z in 0..side - 1 {
        for x in 0..side - 1 {
            let a = (z * side + x) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
    let mut distance = vec![0.0; positions.len()];
    let mut scale = vec![0.0; positions.len()];
    let mut scratch = vec![0; indices.len() + positions.len()];
    let reference = shore_distance_into(
        &positions,
        &indices,
        &mut distance,
        &mut scale,
        &mut scratch,
    )
    .unwrap();
    let (actual, actual_scale, receipt) = accelerated(&positions, &indices);
    assert_eq!(receipt.boundary_edges, reference.boundary_edges);
    assert_eq!(actual, distance);
    assert_eq!(actual_scale, scale);
    assert!(
        receipt.segment_evaluations < reference.segment_evaluations / 2,
        "BVH {} versus reference {}",
        receipt.segment_evaluations,
        reference.segment_evaluations
    );
    let (again, again_scale, again_receipt) = accelerated(&positions, &indices);
    assert_eq!(actual, again);
    assert_eq!(actual_scale, again_scale);
    assert_eq!(receipt, again_receipt);
}

#[test]
fn native_bvh_keeps_overlapping_components_independent() {
    let p = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.1, 2.0, 0.0],
        [0.0, 2.0, 0.1],
    ];
    let indices = [0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4, 5, 6, 7];
    let (distance, scales, _) = accelerated(&p, &indices);
    assert_eq!(distance[4], 1.0);
    assert_eq!(&scales[..5], &[1.0; 5]);
    assert_eq!(&scales[5..], &[0.0; 3]);
}

#[test]
fn short_index_scratch_preserves_caller_outputs() {
    let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let mut edges = [0; 6];
    let topology = shore_topology_into(&p, &[0, 1, 2], &mut edges).unwrap();
    let mut distance = [9.0; 3];
    let mut scale = [8.0; 3];
    let mut bounds = [Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)); 3];
    let mut tree = [BvhNode::default(); 5];
    let mut order = [0; 2];
    let mut morton = [0; 3];
    let mut sorting = [0; 3];
    let mut components = [0; 5];
    assert_eq!(
        shore_distance_accelerated_into(
            &p,
            &topology,
            &mut distance,
            &mut scale,
            ShoreSpatialScratch {
                bounds: &mut bounds,
                nodes: &mut tree,
                primitive_order: &mut order,
                morton: &mut morton,
                sort_order: &mut sorting,
                node_components: &mut components
            }
        ),
        Err(ShoreError::ScratchTooSmall)
    );
    assert_eq!(distance, [9.0; 3]);
    assert_eq!(scale, [8.0; 3]);
}
