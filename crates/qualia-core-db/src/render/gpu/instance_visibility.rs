//! Camera-driven visibility submission for the retained instanced mesh.
//!
//! The complete source stream remains resident for cascaded shadow casters. A compact stable
//! subset is uploaded to a second stream for camera-facing forward and AO passes. All host buffers
//! are preallocated at renderer construction, and an unchanged camera/model/mesh key skips work.

use super::*;
use crate::render::instance_culling::{compact_visible_records, select_visible_mesh_instances};

impl PortalGpu {
    pub(super) fn update_mesh_instance_visibility(&mut self) {
        let view_projection = self
            .camera
            .to_uniform(self.width as f32 / self.height.max(1) as f32, false)
            .view_projection;
        let model_from_mesh = motor_to_mat4_col(self.last_admitted);
        let key = (view_projection, model_from_mesh);
        if self.last_visibility_key == Some(key) {
            return;
        }

        let source_count = self.mesh_instance_source.len();
        let selection = match self.mesh_base_aabb {
            Some(bounds) if self.mesh.is_some() => Some(select_visible_mesh_instances(
                bounds.min,
                bounds.max,
                model_from_mesh,
                view_projection,
                &self.mesh_instance_source,
                &mut self.visible_instance_indices[..source_count],
            )),
            _ => None,
        };
        let visible_count = match selection {
            Some(Ok(count)) => count,
            Some(Err(_)) | None => {
                for (index, output) in self.visible_instance_indices[..source_count]
                    .iter_mut()
                    .enumerate()
                {
                    *output = index as u32;
                }
                source_count
            }
        };

        let compacted = compact_visible_records(
            &self.mesh_instance_source,
            &self.visible_instance_indices[..visible_count],
            &mut self.visible_instance_scratch[..visible_count],
        );
        let compacted_count = if compacted.is_ok() {
            visible_count
        } else {
            self.visible_instance_scratch[..source_count]
                .copy_from_slice(&self.mesh_instance_source);
            source_count
        };
        let packed = bytemuck::cast_slice(&self.visible_instance_scratch[..compacted_count]);
        if self
            .visible_mesh_instances
            .replace_culled_packed(
                &self.device,
                &self.queue,
                &self.mesh_instance_layout,
                packed,
            )
            .is_ok()
        {
            self.last_visibility_key = Some(key);
        } else {
            // The source stream remains the correctness fallback if a device budget changes
            // between scene upload and visibility update. Retry on the next frame.
            self.last_visibility_key = None;
        }
    }
}
