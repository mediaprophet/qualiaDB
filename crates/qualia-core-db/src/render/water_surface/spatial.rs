//! Cold caller-buffered native BVH construction and allocation-free nearest
//! shoreline queries. Exact segment distances remain the acceptance criterion.
use super::{ShoreError, ShoreReceipt, ShoreTopology};
use crate::specialized_libs::computational_geometry::{
    build_bvh_recursive, point_segment_distance_sq_2d, Aabb, BvhNode, Point2, Point3, MAX_BVH_DEPTH,
};
const MIXED: u32 = u32::MAX;
pub struct ShoreSpatialScratch<'a> {
    pub bounds: &'a mut [Aabb],
    pub nodes: &'a mut [BvhNode],
    pub primitive_order: &'a mut [u32],
    pub morton: &'a mut [u64],
    pub sort_order: &'a mut [u32],
    pub node_components: &'a mut [u32],
}
/// Includes caller arrays and the native BVH builder's bounded centroid scratch.
pub fn spatial_working_bytes(edges: usize) -> Option<usize> {
    let nodes = edges.checked_mul(2)?.checked_sub(1)?;
    edges
        .checked_mul(std::mem::size_of::<Aabb>() + 4 + 8 + 4 + 24)?
        .checked_add(nodes.checked_mul(std::mem::size_of::<BvhNode>() + 4)?)
}
fn bbox(node: &BvhNode) -> Aabb {
    Aabb::new(
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
    )
}
fn nearest(
    positions: &[[f32; 3]],
    topology: &ShoreTopology<'_>,
    nodes: &[BvhNode],
    order: &[u32],
    components: &[u32],
    point: Point2,
    component: u32,
) -> Result<(f64, u64), ShoreError> {
    let query = Point3::new(point.x, 0.0, point.y);
    let mut best = f64::INFINITY;
    let mut evaluated = 0;
    let mut stack = [0u32; MAX_BVH_DEPTH * 2];
    let mut pending = 1;
    while pending > 0 {
        pending -= 1;
        let index = stack[pending] as usize;
        let node = &nodes[index];
        if components[index] != MIXED && components[index] != component {
            continue;
        }
        if bbox(node).distance_sq_to_point(query) > best {
            continue;
        }
        if node.node_type == 1 {
            for primitive in &order
                [node.left_or_first as usize..(node.left_or_first + node.right_or_count) as usize]
            {
                let edge = topology.edges[*primitive as usize];
                let a = (edge >> 32) as usize;
                let b = edge as u32 as usize;
                if topology.component(a) != component {
                    continue;
                }
                let p = positions[a];
                let q = positions[b];
                evaluated += 1;
                best = best.min(point_segment_distance_sq_2d(
                    point,
                    Point2::new(p[0] as f64, p[2] as f64),
                    Point2::new(q[0] as f64, q[2] as f64),
                ));
            }
        } else {
            if pending + 2 > stack.len() {
                return Err(ShoreError::SpatialIndex);
            }
            let a = node.left_or_first;
            let b = node.right_or_count;
            let ad = bbox(&nodes[a as usize]).distance_sq_to_point(query);
            let bd = bbox(&nodes[b as usize]).distance_sq_to_point(query);
            let (near, far) = if ad <= bd { (a, b) } else { (b, a) };
            stack[pending] = far;
            stack[pending + 1] = near;
            pending += 2;
        }
    }
    if !best.is_finite() {
        return Err(ShoreError::SpatialIndex);
    }
    Ok((best.sqrt(), evaluated))
}
pub fn shore_distance_accelerated_into(
    positions: &[[f32; 3]],
    topology: &ShoreTopology<'_>,
    distances: &mut [f32],
    scales: &mut [f32],
    scratch: ShoreSpatialScratch<'_>,
) -> Result<ShoreReceipt, ShoreError> {
    let n = topology.edges.len();
    let node_capacity = n
        .checked_mul(2)
        .and_then(|n| n.checked_sub(1))
        .ok_or(ShoreError::InvalidInput)?;
    if positions.len() != topology.vertices()
        || positions
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
    {
        return Err(ShoreError::InvalidInput);
    }
    if distances.len() < positions.len() || scales.len() < positions.len() {
        return Err(ShoreError::OutputTooSmall);
    }
    if scratch.bounds.len() < n
        || scratch.nodes.len() < node_capacity
        || scratch.primitive_order.len() < n
        || scratch.morton.len() < n
        || scratch.sort_order.len() < n
        || scratch.node_components.len() < node_capacity
    {
        return Err(ShoreError::ScratchTooSmall);
    }
    for (index, edge) in topology.edges.iter().enumerate() {
        let a = positions[(*edge >> 32) as usize];
        let b = positions[*edge as u32 as usize];
        scratch.bounds[index] = Aabb::new(
            Point3::new(a[0].min(b[0]) as f64, 0.0, a[2].min(b[2]) as f64),
            Point3::new(a[0].max(b[0]) as f64, 0.0, a[2].max(b[2]) as f64),
        );
    }
    let (node_count, _) = build_bvh_recursive(
        &scratch.bounds[..n],
        scratch.nodes,
        scratch.primitive_order,
        scratch.morton,
        scratch.sort_order,
    )
    .map_err(|_| ShoreError::SpatialIndex)?;
    for index in (0..node_count).rev() {
        let node = &scratch.nodes[index];
        scratch.node_components[index] = if node.node_type == 1 {
            let mut component = MIXED;
            for (i, primitive) in scratch.primitive_order
                [node.left_or_first as usize..(node.left_or_first + node.right_or_count) as usize]
                .iter()
                .enumerate()
            {
                let candidate =
                    topology.component((topology.edges[*primitive as usize] >> 32) as usize);
                if i == 0 {
                    component = candidate;
                } else if component != candidate {
                    component = MIXED;
                    break;
                }
            }
            component
        } else {
            let a = scratch.node_components[node.left_or_first as usize];
            let b = scratch.node_components[node.right_or_count as usize];
            if a == b {
                a
            } else {
                MIXED
            }
        };
    }
    scales[..positions.len()].fill(0.0);
    let mut max_distance = 0.0_f32;
    let mut segment_evaluations = 0;
    for index in 0..positions.len() {
        if !topology.referenced(index) {
            distances[index] = 0.0;
            continue;
        }
        let p = positions[index];
        let component = topology.component(index);
        let (distance, evaluations) = nearest(
            positions,
            topology,
            &scratch.nodes[..node_count],
            scratch.primitive_order,
            &scratch.node_components[..node_count],
            Point2::new(p[0] as f64, p[2] as f64),
            component,
        )?;
        let distance = distance as f32;
        distances[index] = distance;
        segment_evaluations += evaluations;
        scales[component as usize] = scales[component as usize].max(distance);
        max_distance = max_distance.max(distance);
    }
    for index in 0..positions.len() {
        scales[index] = scales[topology.component(index) as usize];
    }
    Ok(ShoreReceipt {
        boundary_edges: n,
        max_distance,
        segment_evaluations,
    })
}
