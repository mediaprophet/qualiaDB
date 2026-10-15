//! Allocation-free upload of native skinning output, including surface frames.
use super::{materials, PortalGpu};
use crate::render::physics::Aabb;

impl PortalGpu {
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
