//! Allocation-free upload of native skinning output, including surface frames
//! and compact GPU PGA `Motor` dual-quaternion palettes.
use super::{materials, PortalGpu};
use crate::render::physics::Aabb;
use crate::render::skeletal::gpu_skin::{
    GpuJointMotorPair, MAX_GPU_RIG_JOINTS, MAX_GPU_SKELETON_INSTANCES,
};

/// Uploads a compact per-instance PGA `Motor` skinning palette (`<= 128` joints, 64 bytes/joint)
/// directly to a GPU storage buffer at `instance_rig_offset` (in joint slots) with zero heap
/// allocations (`HotZeroHeap`).
///
/// Replaces the legacy 3× full-vertex-buffer upload (`positions`, `normals`, `tangents`)
/// when hardware vertex-shader dual-quaternion skinning is active.
pub fn upload_rig_motor_palette_gpu(
    queue: &wgpu::Queue,
    palette_buffer: &wgpu::Buffer,
    instance_rig_offset: u32,
    palette: &[GpuJointMotorPair],
) -> bool {
    let count = palette.len().min(MAX_GPU_RIG_JOINTS);
    if count == 0 {
        return false;
    }
    let max_total_joints = MAX_GPU_RIG_JOINTS * MAX_GPU_SKELETON_INSTANCES;
    let first_joint = instance_rig_offset as usize;
    let Some(end_joint) = first_joint.checked_add(count) else {
        return false;
    };
    if end_joint > max_total_joints {
        return false;
    }
    if palette[..count]
        .iter()
        .any(|m| {
            m.current_r
                .iter()
                .chain(m.current_d.iter())
                .chain(m.prev_r.iter())
                .chain(m.prev_d.iter())
                .any(|v| !v.is_finite())
        })
    {
        return false;
    }
    let byte_offset = (first_joint * core::mem::size_of::<GpuJointMotorPair>()) as u64;
    queue.write_buffer(
        palette_buffer,
        byte_offset,
        bytemuck::cast_slice(&palette[..count]),
    );
    true
}

impl PortalGpu {
    /// Uploads a compact GPU PGA `Motor` palette into `palette_buffer` using this device's queue.
    pub fn upload_rig_motor_palette(
        &self,
        palette_buffer: &wgpu::Buffer,
        instance_rig_offset: u32,
        palette: &[GpuJointMotorPair],
    ) -> bool {
        upload_rig_motor_palette_gpu(&self.queue, palette_buffer, instance_rig_offset, palette)
    }

    /// Writes `mesh-animated-frames`.
    pub fn write_mesh_animated_frames(
        &mut self,
        start: u32,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        tangents: &[[f32; 4]],
    ) -> bool {
        let Some(mesh) = self.mesh.as_mut() else {
            return false;
        };
        let first = start as usize;
        let Some(end) = first.checked_add(positions.len()) else {
            return false;
        };
        if positions.is_empty()
            || end > mesh.vertex_count as usize
            || normals.len() != positions.len()
            || tangents.len() != positions.len()
            || positions
                .iter()
                .flatten()
                .chain(normals.iter().flatten())
                .chain(tangents.iter().flatten())
                .any(|v| !v.is_finite())
        {
            return false;
        }
        mesh.cpu_positions[first..end].copy_from_slice(positions);
        // Maintain the legacy edit workspace so later topology-based edits
        // start from the actual posed geometry, without replacing skin normals.
        if mesh
            .normal_workspace
            .update_positions(&mesh.cpu_positions, first, end)
            .is_err()
        {
            return false;
        }
        self.queue.write_buffer(
            &mesh.vertex_buf,
            (first * 12) as u64,
            bytemuck::cast_slice(positions),
        );
        self.queue.write_buffer(
            &mesh.normal_buf,
            (first * 12) as u64,
            bytemuck::cast_slice(normals),
        );
        self.queue.write_buffer(
            &mesh.tangent_buf,
            (first * 16) as u64,
            bytemuck::cast_slice(tangents),
        );
        materials::update_draw_bounds(
            &mut mesh.material_gpu.draws,
            &mesh.cpu_positions,
            &mesh.cpu_indices,
        );
        materials::update_draw_bounds(
            &mut mesh.shadow_draws,
            &mesh.cpu_positions,
            &mesh.cpu_indices,
        );
        self.mesh_base_aabb = Aabb::from_points(&mesh.cpu_positions);
        self.last_visibility_key = None;
        self.shadow_map_dirty = true;
        true
    }
}
