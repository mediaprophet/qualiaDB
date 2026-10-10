//! Per-frame uniform buffer writing and model pose updates for PortalGpu.

use super::super::{motor_to_mat4_col, shadows, PortalGpu};
use crate::render::pga::Motor;
use crate::render::physics::Admission;

impl PortalGpu {
    pub(super) fn write_camera_uniform(&mut self, encoder: &mut wgpu::CommandEncoder, time: f32) {
        let aspect = self.width as f32 / self.height.max(1) as f32;
        let mut uniform = self
            .camera
            .to_uniform(aspect, self.tensor_raw_buf.is_some());
        uniform._padding[0] = time;
        // VC3: Use UniformBelt for zero-alloc buffer writes.
        let bytes = bytemuck::bytes_of(&uniform);
        self.uniform_belt.write_and_unmap(bytes);
        self.uniform_belt.record_copy(encoder, &self.camera_buf, 0);
        self.uniform_belt.advance(&self.device);
    }

    pub(super) fn write_observer_uniform(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // VC3: Use UniformBelt for zero-alloc buffer writes.
        let bytes = bytemuck::bytes_of(&self.observer);
        self.uniform_belt.write_and_unmap(bytes);
        self.uniform_belt
            .record_copy(encoder, &self.observer_buf, 0);
        self.uniform_belt.advance(&self.device);
    }

    /// Resolve this frame's per-artefact model transform: the joint pose at `time`, gated through
    /// the admission policy (refuse out-of-world → hold the last admitted pose), then write it.
    pub(super) fn update_model(&mut self, encoder: &mut wgpu::CommandEncoder, time: f32) {
        let previous_motor = self.last_admitted;
        let proposed = match self.artefact_joint {
            // Drive by *elapsed* time since the joint was engaged, not absolute sim-time, so a slide
            // always starts from rest when armed (the t0 is latched on this first post-arm frame).
            Some(j) => {
                let t0 = *self.artefact_t0.get_or_insert(time);
                j.motor_at(time - t0)
            }
            None => Motor::identity(),
        };
        let motor = match (self.mesh_base_aabb, self.artefact_world) {
            (Some(base), Some(world)) => {
                match Admission::new(0.0, Some(world)).admit(&base, proposed, [1.0, 1.0, 1.0]) {
                    Ok(_) => {
                        self.last_refused = false;
                        self.last_admitted = proposed;
                        proposed
                    }
                    Err(_) => {
                        self.last_refused = true;
                        self.last_admitted // deterministic refusal: hold at the boundary
                    }
                }
            }
            _ => {
                self.last_refused = false;
                self.last_admitted = proposed;
                proposed
            }
        };
        if motor != previous_motor {
            self.shadow_map_dirty = true;
        }
        let model = motor_to_mat4_col(motor);
        // VC3: Use UniformBelt for zero-alloc buffer writes.
        let bytes = bytemuck::cast_slice(&model);
        self.uniform_belt.write_and_unmap(bytes);
        self.uniform_belt.record_copy(encoder, &self.model_buf, 0);
        self.uniform_belt.advance(&self.device);
    }

    pub(super) fn write_shadow_uniform(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let active = self.shadow_enabled
            && self.shadow_target.is_some()
            && self.camera.sun_intensity > 0.0
            && self
                .mesh
                .as_ref()
                .is_some_and(|mesh| mesh.receives_shadows && !mesh.shadow_draws.is_empty());
        let resolution = self
            .shadow_target
            .as_ref()
            .map_or(1, |target| target.resolution);
        let world_bounds = self
            .mesh_base_aabb
            .map(|bounds| bounds.transformed(self.last_admitted, [1.0; 3]));
        let scene_center = world_bounds.map_or([0.0; 3], |bounds| bounds.center());
        let scene_extent = world_bounds.map_or([0.0; 3], |bounds| bounds.extent());
        let eye = crate::render::camera::orbit_eye_position_target(
            self.camera.yaw,
            self.camera.pitch,
            self.camera.zoom,
            self.camera.target,
        );
        let split = (self.camera.zoom * 4.0).clamp(8.0, 48.0);
        let light_view_projection = shadows::camera_range_sun_view_projections(
            self.camera.sun_dir,
            eye,
            crate::render::camera::orbit_forward(self.camera.yaw, self.camera.pitch),
            scene_center,
            scene_extent,
            self.width as f32 / self.height.max(1) as f32,
            split,
            200.0,
            resolution,
        );
        let uniform = shadows::ShadowUniform {
            light_view_projection,
            params: [
                if active { 1.0 } else { 0.0 },
                1.0 / resolution as f32,
                0.0015,
                split,
            ],
        };
        if uniform.light_view_projection != self.shadow_uniform_cpu.light_view_projection {
            self.shadow_map_dirty = true;
        }
        if uniform != self.shadow_uniform_cpu {
            self.uniform_belt
                .write_and_unmap(bytemuck::bytes_of(&uniform));
            self.uniform_belt
                .record_copy(encoder, &self.shadow_uniform_buf, 0);
            self.uniform_belt.advance(&self.device);
            self.shadow_uniform_cpu = uniform;
        }
    }
}
