//! Zero-Heap Topological PGA [`Motor`] Hierarchy Propagation (`transform_sync.rs`).
//!
//! Computes a flat breadth-first topological order over active [`SceneNodeRecord`]s in zero heap
//! (using the caller's output slice itself as the traversal queue) and propagates hierarchical
//! PGA [`Motor`] transforms:
//! 1. Composes `base_motor = parent.world_motor.motor_mul(&node.local_motor)`,
//! 2. Multiplies by `crate::render::pga::motor_v_band` when `node.tensor_v >= 1`,
//! 3. Applies deontic bilateral pull (`crate::render::pga::bilateral_pull_active` + `pull_vector`)
//!    toward `observer_pos` when `node.deontic_lane != 0`,
//! 4. Emits dirty [`SyncedInstanceTransform`] and [`SyncedHolographicNode`] GPU records into
//!    caller-provided slices.

use bytemuck::{Pod, Zeroable};

use crate::render::pga::{
    bilateral_pull_active, motor_mul, motor_translate, motor_v_band, pull_vector, sandwich_point,
    Motor,
};
use crate::render::scene_graph::pool::{NodeHandle, NodeKind, SceneNodeRecord};
use crate::render::telemetry::{STANDPOINT_DID, STANDPOINT_SPECTATOR};

/// Dirty GPU instance transform emitted during [`propagate_transforms_zero_heap`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SyncedInstanceTransform {
    /// Source node slot index in the scene pool.
    pub node_index: u32,
    /// Asset / mesh handle (`u32::MAX` for non-mesh nodes).
    pub mesh_handle: u32,
    /// Composed world-space PGA [`Motor`] rotor `r` `[s, e12, e13, e23]`.
    pub world_motor_r: [f32; 4],
    /// Composed world-space PGA [`Motor`] dual `d` `[e0123, e03, e02, e01]`.
    pub world_motor_d: [f32; 4],
    /// Composed world-space scale `[sx, sy, sz]` and perceptual salience `alpha`.
    pub world_scale_alpha: [f32; 4],
    /// Semantic entity token (`u64` split into `[lo, hi]`), `material_id`, and `proc_header_handle`.
    pub metadata_packed: [u32; 4],
}

/// Platform-agnostic 96-byte GPU record matching `HolographicNodeGpu` byte-for-byte.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SyncedHolographicNode {
    pub position_scale: [f32; 4],
    pub half_extents_primitive: [f32; 4],
    pub frequency_amplitude: [f32; 4],
    pub manifold_tensor: [f32; 4],
    pub connection_endpoint: [f32; 4],
    pub emissive_color: [f32; 4],
}

const _: [(); 96] = [(); core::mem::size_of::<SyncedHolographicNode>()];

/// Checks whether bilateral deontic pull is active for a given scene node and observer DID.
#[inline]
pub fn bilateral_pull_active_for_node(
    deontic_lane: u8,
    observer_did_hash: u64,
    mu_carrier: u64,
) -> bool {
    let standpoint_class = if observer_did_hash != 0 && (mu_carrier == 0 || mu_carrier == observer_did_hash) {
        STANDPOINT_DID
    } else {
        STANDPOINT_SPECTATOR
    };
    bilateral_pull_active(deontic_lane as f32, standpoint_class)
}

/// Computes a parent-before-child topological ordering of active nodes into `out_order`.
///
/// Uses `out_order` itself as an in-place FIFO breadth-first queue — **0 bytes** of heap or
/// auxiliary stack array allocation regardless of arena size.
///
/// Returns the number of node indices written to `out_order`.
pub fn compute_topological_order(
    pool_nodes: &[SceneNodeRecord],
    active_mask: &[bool],
    out_order: &mut [u32],
) -> usize {
    let count = pool_nodes.len().min(active_mask.len());
    let max_out = out_order.len();
    if count == 0 || max_out == 0 {
        return 0;
    }

    let mut write_cursor = 0usize;

    // Pass 1: Enqueue all active root nodes (nodes with no active parent).
    let mut i = 0usize;
    while i < count && write_cursor < max_out {
        if active_mask[i] {
            let p = pool_nodes[i].parent;
            let has_active_parent =
                p.is_some() && (p.index as usize) < count && active_mask[p.index as usize];
            if !has_active_parent {
                out_order[write_cursor] = i as u32;
                write_cursor += 1;
            }
        }
        i += 1;
    }

    // Pass 2: Breadth-first expansion using `out_order[read_cursor..write_cursor]` as the queue.
    let mut read_cursor = 0usize;
    while read_cursor < write_cursor && write_cursor < max_out {
        let parent_idx = out_order[read_cursor] as usize;
        read_cursor += 1;

        let mut child: NodeHandle = pool_nodes[parent_idx].first_child;
        let mut sibling_steps = 0usize;
        while child.is_some() && sibling_steps < count && write_cursor < max_out {
            sibling_steps += 1;
            let c_idx = child.index as usize;
            if c_idx >= count {
                break;
            }
            if active_mask[c_idx] {
                out_order[write_cursor] = c_idx as u32;
                write_cursor += 1;
            }
            child = pool_nodes[c_idx].next_sibling;
        }
    }

    write_cursor
}

/// Summary counts returned by [`propagate_transforms_zero_heap`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransformSyncSummary {
    /// Total nodes visited in topological order.
    pub visited_count: usize,
    /// Number of dirty [`SyncedInstanceTransform`] records written.
    pub synced_instances: usize,
    /// Number of dirty [`SyncedHolographicNode`] records written.
    pub synced_holographic_nodes: usize,
}

/// Propagates world PGA [`Motor`] transforms in topological order with zero heap allocation.
///
/// Recomputes a node's world motor whenever:
/// `parent.changed_tick >= last_sync_tick || node.changed_tick >= last_sync_tick || node.tensor_v >= 1 || node.deontic_lane != 0`.
#[allow(clippy::too_many_arguments)]
pub fn propagate_transforms_zero_heap(
    pool_nodes: &mut [SceneNodeRecord],
    active_mask: &[bool],
    topological_order: &[u32],
    last_sync_tick: u32,
    current_tick: u32,
    time_sec: f32,
    observer_pos: [f32; 3],
    observer_did_hash: u64,
    out_instances: &mut [SyncedInstanceTransform],
    out_holographic: &mut [SyncedHolographicNode],
) -> TransformSyncSummary {
    let node_cap = pool_nodes.len().min(active_mask.len());
    let mut summary = TransformSyncSummary::default();

    let mut order_idx = 0usize;
    while order_idx < topological_order.len() {
        let idx = topological_order[order_idx] as usize;
        order_idx += 1;
        if idx >= node_cap || !active_mask[idx] {
            continue;
        }
        summary.visited_count += 1;

        let parent_handle = pool_nodes[idx].parent;
        let has_parent = parent_handle.is_some()
            && (parent_handle.index as usize) < node_cap
            && active_mask[parent_handle.index as usize];

        let parent_dirty = if has_parent {
            pool_nodes[parent_handle.index as usize].changed_tick >= last_sync_tick
        } else {
            false
        };

        let node_dirty = pool_nodes[idx].changed_tick >= last_sync_tick
            || pool_nodes[idx].tensor_v >= 1
            || pool_nodes[idx].deontic_lane != 0;

        if !parent_dirty && !node_dirty {
            continue;
        }

        let (parent_motor, parent_scale) = if has_parent {
            let p = &pool_nodes[parent_handle.index as usize];
            (p.world_motor, p.world_scale)
        } else {
            (Motor::identity(), [1.0, 1.0, 1.0])
        };

        let node = &mut pool_nodes[idx];
        let mut world_motor = parent_motor.motor_mul(&node.local_motor);
        let world_scale = [
            parent_scale[0] * node.local_scale[0],
            parent_scale[1] * node.local_scale[1],
            parent_scale[2] * node.local_scale[2],
        ];

        // 1. Apply 10D `v`-band topological motor when `tensor_v >= 1`.
        if node.tensor_v >= 1 {
            let pos = sandwich_point(world_motor, [0.0, 0.0, 0.0]);
            let v_motor = motor_v_band(
                node.tensor_v as f32,
                pos,
                node.sigma_spectral,
                time_sec,
                node.alpha_salience,
            );
            world_motor = motor_mul(world_motor, v_motor);
        }

        // 2. Apply deontic bilateral pull toward `observer_pos` when permitted.
        if bilateral_pull_active_for_node(
            node.deontic_lane,
            observer_did_hash,
            node.mu_carrier as u64,
        ) {
            let pos = sandwich_point(world_motor, [0.0, 0.0, 0.0]);
            let pv = pull_vector(pos, observer_pos, node.alpha_salience, node.epistemic_q);
            let t_pull = motor_translate(pv);
            world_motor = motor_mul(t_pull, world_motor);
        }

        node.world_motor = world_motor;
        node.world_scale = world_scale;
        node.changed_tick = current_tick;

        // 3. Emit dirty GPU instance record.
        if summary.synced_instances < out_instances.len() {
            out_instances[summary.synced_instances] = SyncedInstanceTransform {
                node_index: idx as u32,
                mesh_handle: node.mesh_handle,
                world_motor_r: world_motor.r,
                world_motor_d: world_motor.d,
                world_scale_alpha: [
                    world_scale[0],
                    world_scale[1],
                    world_scale[2],
                    node.alpha_salience,
                ],
                metadata_packed: [
                    (node.quin_token & 0xFFFF_FFFF) as u32,
                    (node.quin_token >> 32) as u32,
                    (node.material_id as u32) | ((node.proc_header_handle as u32) << 16),
                    (node.eont_handle as u32) | ((node.rig_handle as u32) << 16),
                ],
            };
            summary.synced_instances += 1;
        }

        // 4. Emit Holographic SDF proxy-lens record if applicable.
        if node.node_kind == NodeKind::HolographicSdf
            && summary.synced_holographic_nodes < out_holographic.len()
        {
            let world_pos = sandwich_point(world_motor, [0.0, 0.0, 0.0]);
            let max_scale = world_scale[0].max(world_scale[1]).max(world_scale[2]);
            out_holographic[summary.synced_holographic_nodes] = SyncedHolographicNode {
                position_scale: [world_pos[0], world_pos[1], world_pos[2], max_scale],
                half_extents_primitive: [world_scale[0], world_scale[1], world_scale[2], 0.0],
                frequency_amplitude: [1.0 + node.sigma_spectral * 4.0, node.alpha_salience, time_sec, 1.0],
                manifold_tensor: [node.epistemic_q, node.tensor_v as f32, 0.0, 0.5],
                connection_endpoint: [0.0, 0.0, 0.0, 1.0],
                emissive_color: [0.1, 0.85, 1.0, 1.0],
            };
            summary.synced_holographic_nodes += 1;
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::scene_graph::pool::NodePool;
    use crate::render::telemetry::DEONTIC_LANE_BILATERAL;

    #[test]
    fn topological_order_and_pga_motor_propagation() {
        let mut pool = NodePool::<16>::new();

        let mut root_rec = SceneNodeRecord::new(1, NodeKind::Empty);
        root_rec.local_motor = motor_translate([10.0, 0.0, 0.0]);
        root_rec.local_scale = [2.0, 2.0, 2.0];
        let root = pool.spawn(root_rec).unwrap();

        let mut child_rec = SceneNodeRecord::new(2, NodeKind::HolographicSdf);
        child_rec.local_motor = motor_translate([0.0, 5.0, 0.0]);
        child_rec.local_scale = [1.5, 1.5, 1.5];
        let child = pool.spawn(child_rec).unwrap();

        assert!(pool.attach_child(root, child));

        let mut order = [0u32; 16];
        let order_len = compute_topological_order(&pool.records, &pool.active, &mut order);
        assert_eq!(order_len, 2);
        assert_eq!(order[0], root.index);
        assert_eq!(order[1], child.index);

        let mut instances = [SyncedInstanceTransform::zeroed(); 16];
        let mut holo = [SyncedHolographicNode::zeroed(); 8];

        let summary = propagate_transforms_zero_heap(
            &mut pool.records,
            &pool.active,
            &order[..order_len],
            1,
            2,
            0.0,
            [0.0, 0.0, 0.0],
            0,
            &mut instances,
            &mut holo,
        );

        assert_eq!(summary.visited_count, 2);
        assert_eq!(summary.synced_instances, 2);
        assert_eq!(summary.synced_holographic_nodes, 1);

        let child_world_pos = pool.get(child).unwrap().world_motor.translation();
        assert!((child_world_pos[0] - 10.0).abs() < 1e-5);
        assert!((child_world_pos[1] - 5.0).abs() < 1e-5);
        assert_eq!(pool.get(child).unwrap().world_scale, [3.0, 3.0, 3.0]);
    }

    #[test]
    fn v_band_and_bilateral_pull_modulate_world_motor() {
        let mut pool = NodePool::<8>::new();
        let mut node_rec = SceneNodeRecord::new(99, NodeKind::MeshBRep);
        node_rec.local_motor = motor_translate([1.0, 0.0, 0.0]);
        node_rec.tensor_v = 2; // Hyperbolic radial spread
        node_rec.deontic_lane = DEONTIC_LANE_BILATERAL as u8;
        node_rec.mu_carrier = 0x7777;
        node_rec.alpha_salience = 1.0;
        node_rec.epistemic_q = 1.0;
        let h = pool.spawn(node_rec).unwrap();

        let mut order = [0u32; 8];
        let n = compute_topological_order(&pool.records, &pool.active, &mut order);
        let mut instances = [SyncedInstanceTransform::zeroed(); 8];
        let mut holo = [SyncedHolographicNode::zeroed(); 8];

        let _ = propagate_transforms_zero_heap(
            &mut pool.records,
            &pool.active,
            &order[..n],
            1,
            2,
            1.0,
            [10.0, 0.0, 0.0],
            0x7777,
            &mut instances,
            &mut holo,
        );

        // Hyperbolic v-band + bilateral pull toward x=10.0 must shift x > 1.0.
        let pos = pool.get(h).unwrap().world_motor.translation();
        assert!(pos[0] > 1.05, "expected x > 1.05, got {}", pos[0]);
    }
}
