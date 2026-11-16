use super::instance_groups::compact_group_visibility;
use super::PortalGpu;

impl PortalGpu {
    pub(super) fn update_group_visibility(&mut self, model: [[f32; 4]; 4], view: [[f32; 4]; 4]) {
        let Ok(cursor) = compact_group_visibility(
            &mut self.mesh_instance_groups,
            &self.mesh_instance_source,
            model,
            view,
            &mut self.visible_instance_indices,
            &mut self.visible_instance_scratch,
        ) else {
            for group in &mut self.mesh_instance_groups {
                group.visible = 0..0;
            }
            self.last_visibility_key = None;
            return;
        };
        let packed = bytemuck::cast_slice(&self.visible_instance_scratch[..cursor]);
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
            self.last_visibility_key = Some((view, model));
        } else {
            // Never submit new ranges against an older GPU stream after a failed upload.
            for group in &mut self.mesh_instance_groups {
                group.visible = 0..0;
            }
            self.last_visibility_key = None;
        }
    }
}
