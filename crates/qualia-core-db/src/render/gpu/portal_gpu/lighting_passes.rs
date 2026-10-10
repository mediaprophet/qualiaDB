//! Ambient occlusion, cascaded sun shadows, and transparent draw sorting for PortalGpu.

use super::super::{
    ao, create_mesh_frame_bind, motor_to_mat4_col, shadows, PortalGpu,
};

impl PortalGpu {
    pub(super) fn sort_transparent_draws(&mut self) {
        let Some(mesh) = self.mesh.as_mut() else {
            return;
        };
        let eye = crate::render::camera::orbit_eye_position_target(
            self.camera.yaw,
            self.camera.pitch,
            self.camera.zoom,
            self.camera.target,
        );
        let forward = crate::render::camera::orbit_forward(self.camera.yaw, self.camera.pitch);
        let model = motor_to_mat4_col(self.last_admitted);
        let draws = &mut mesh.material_gpu.draws;
        for &index in &mesh.material_gpu.transparent_draw_order {
            let draw = &mut draws[index];
            let point = draw.center;
            let world = [
                model[0][0] * point[0]
                    + model[1][0] * point[1]
                    + model[2][0] * point[2]
                    + model[3][0],
                model[0][1] * point[0]
                    + model[1][1] * point[1]
                    + model[2][1] * point[2]
                    + model[3][1],
                model[0][2] * point[0]
                    + model[1][2] * point[1]
                    + model[2][2] * point[2]
                    + model[3][2],
            ];
            let to_center = [world[0] - eye[0], world[1] - eye[1], world[2] - eye[2]];
            draw.sort_depth =
                to_center[0] * forward[0] + to_center[1] * forward[1] + to_center[2] * forward[2];
        }
        mesh.material_gpu
            .transparent_draw_order
            .sort_by(|left, right| {
                draws[*right]
                    .sort_depth
                    .total_cmp(&draws[*left].sort_depth)
                    .then_with(|| left.cmp(right))
            });
    }

    pub(super) fn record_sun_shadow_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(target), Some(mesh)) = (self.shadow_target.as_ref(), self.mesh.as_ref()) else {
            return;
        };
        if !self.shadow_enabled
            || !mesh.receives_shadows
            || mesh.shadow_draws.is_empty()
            || self.camera.sun_intensity <= 0.0
            || !self.shadow_map_dirty
        {
            return;
        }
        for (cascade, view) in target.views.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if cascade == 0 {
                    "portal-sun-shadow-near-depth-pass"
                } else {
                    "portal-sun-shadow-far-depth-pass"
                }),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.shadow_pipelines[cascade]);
            pass.set_bind_group(0, &self.shadow_uniform_bind, &[]);
            pass.set_bind_group(1, &self.mesh_model_bind, &[]);
            pass.set_bind_group(3, self.mesh_instances.bind_group(), &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
            pass.set_vertex_buffer(1, mesh.color_buf.slice(..));
            pass.set_vertex_buffer(2, mesh.uv_buf.slice(..));
            pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            let cascade_matrix = self.shadow_uniform_cpu.light_view_projection[cascade];
            let model_matrix = motor_to_mat4_col(self.last_admitted);
            for draw in &mesh.shadow_draws {
                if self.mesh_instances.count() == 1
                    && shadows::aabb_outside_shadow_clip(
                        draw.bounds_min,
                        draw.bounds_max,
                        cascade_matrix,
                        model_matrix,
                    )
                {
                    continue;
                }
                pass.set_bind_group(
                    2,
                    &mesh.material_gpu.material_bind_groups[draw.material_index],
                    &[draw.material_offset],
                );
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    0,
                    0..self.mesh_instances.count(),
                );
            }
        }
        self.shadow_map_dirty = false;
    }

    pub(in crate::render::gpu) fn rebuild_ao_resources(&mut self) {
        // Release old extents before admission so a resize can reuse their reserved bytes.
        self.ao_targets = None;
        let targets = ao::AoTargets::try_new(
            &self.device,
            self.width,
            self.height,
            &self.ao_prepass_camera_layout,
            &self.mesh_model_layout,
            &self.mesh_instance_layout,
            &self.mesh_material_layout,
            &self.mesh_texture_layout,
            &self.ao_uniform_buf,
        );
        let (ao_view, packed_surface_view) = targets
            .as_ref()
            .map(|resources| (resources.ao_view(), resources.normal_view()))
            .unwrap_or((
                self.material_texture_defaults.view(3),
                self.material_texture_defaults.view(1),
            ));
        let shadow_views = self
            .shadow_target
            .as_ref()
            .map(|target| [&target.views[0], &target.views[1]])
            .unwrap_or([self.scene_depth.view(), self.scene_depth.view()]);
        self.mesh_frame_bind = create_mesh_frame_bind(
            &self.device,
            &self.mesh_frame_layout,
            &self.camera_buf,
            &self.observer_buf,
            shadow_views,
            &self.shadow_sampler,
            &self.shadow_uniform_buf,
            ao_view,
            packed_surface_view,
            &self.ao_uniform_buf,
            &self.atmosphere_uniform_buf,
        );
        self.ao_targets = targets;
    }

    pub(super) fn write_ao_uniform(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let camera = self
            .camera
            .to_uniform(self.width as f32 / self.height.max(1) as f32, false);
        let enabled = self.ao_enabled && self.ao_targets.is_some() && self.mesh.is_some();
        let uniform = ao::make_uniform(
            self.width,
            self.height,
            [camera._padding[1], camera._padding[2], camera._padding[3]],
            crate::render::camera::orbit_forward(camera.yaw, camera.pitch),
            self.ao_radius,
            self.ao_strength,
            self.ao_bias,
            self.ao_sample_count,
            enabled,
        );
        self.uniform_belt
            .write_and_unmap(bytemuck::bytes_of(&uniform));
        self.uniform_belt
            .record_copy(encoder, &self.ao_uniform_buf, 0);
        self.uniform_belt.advance(&self.device);
    }

    pub(super) fn record_ao_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.ao_enabled {
            if let (Some(targets), Some(mesh)) = (self.ao_targets.as_ref(), self.mesh.as_ref()) {
                targets.record(
                    encoder,
                    mesh,
                    &self.ao_prepass_camera_bind,
                    &self.mesh_model_bind,
                    self.visible_mesh_instances.bind_group(),
                    self.visible_mesh_instances.count(),
                );
            }
        }
    }
}
