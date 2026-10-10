//! Viewport state settings, camera transforms, lighting presets, and temporal resolve configuration.

use super::super::{
    ao, atmosphere, output_pass, temporal_resolve_gpu, PortalGpu,
    TemporalProducerAvailability, TemporalSubmissionState,
};
use crate::render::atmosphere::AtmospherePreset;
use crate::render::camera::CameraState;
use crate::render::pga::Motor;
use crate::render::physics::{Aabb, Joint};
use crate::render::telemetry::ObserverStandpoint;

impl PortalGpu {
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Enable/disable the ambient particle field draw. Off by default and opt-in: a plain
    /// mesh/anatomy view (or a game scene) has no use for the particle cloud; a Tensor10D
    /// upload keeps it off and the mixer's "ambient" channel (or an explicit call) enables it.
    pub fn set_ambient_enabled(&mut self, on: bool) {
        self.ambient_enabled = on;
    }

    /// Show or hide semantic tensor sprites without releasing their pick data.
    pub fn set_tensor_projection_enabled(&mut self, on: bool) {
        self.tensor_projection_enabled = on;
    }

    /// Enable the budgeted half-resolution depth/normal ambient-visibility pass when available.
    pub fn set_screen_space_ao_enabled(&mut self, enabled: bool) {
        self.ao_enabled = enabled;
    }

    /// Configure the indirect-visibility radius and strength in world units and normalized range.
    pub fn set_screen_space_ao(&mut self, radius: f32, strength: f32, bias: f32) {
        self.ao_radius = radius.clamp(0.05, 2.0);
        self.ao_strength = strength.clamp(0.0, 1.0);
        self.ao_bias = bias.clamp(0.0, 0.2);
    }

    /// Set the bounded SSAO tap count; 4 is the low-cost tier, 8 default, 12 quality tier.
    pub fn set_screen_space_ao_sample_count(&mut self, samples: u32) {
        self.ao_sample_count = if samples <= 4 {
            4
        } else if samples <= 8 {
            8
        } else {
            12
        };
    }

    pub fn screen_space_ao_available(&self) -> bool {
        self.ao_targets.is_some()
    }

    pub fn screen_space_ao_enabled(&self) -> bool {
        self.ao_enabled
    }

    pub fn screen_space_ao_resolution(&self) -> Option<(u32, u32)> {
        self.ao_targets.as_ref().map(ao::AoTargets::extent)
    }


    /// Drive the loaded mesh by a kinematic joint (Phase 2). `None` freezes it at identity.
    pub fn set_artefact_joint(&mut self, joint: Option<Joint>) {
        self.artefact_joint = joint;
        self.artefact_t0 = None; // re-engage from rest: the slide/spin starts at elapsed t = 0
        self.last_admitted = Motor::identity();
        self.last_refused = false;
        self.shadow_map_dirty = true;
    }

    /// Constrain the artefact to a world bound; a joint pose that would leave it is refused
    /// (the artefact holds at the last admitted pose). `None` = unconstrained.
    pub fn set_artefact_world(&mut self, world: Option<Aabb>) {
        self.artefact_world = world;
    }

    /// Whether last frame's proposed joint pose was deterministically refused (clamped at the bound).
    pub fn artefact_refused(&self) -> bool {
        self.last_refused
    }

    /// Enable or disable the portable two-cascade sun-shadow tier. Disabling it keeps geometry and
    /// material state intact and removes both the shadow pass and receiver sampling work.
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        if enabled && !self.shadow_enabled {
            self.shadow_map_dirty = true;
        }
        self.shadow_enabled = enabled;
    }

    pub fn shadows_enabled(&self) -> bool {
        self.shadow_enabled && self.shadow_target.is_some()
    }

    pub fn shadow_resolution(&self) -> Option<u32> {
        self.shadow_target.as_ref().map(|target| target.resolution)
    }


    pub fn set_camera(&mut self, yaw: f32, pitch: f32, zoom: f32) {
        let current_target = self.camera.target;
        let sun_dir = self.camera.sun_dir;
        let sun_intensity = self.camera.sun_intensity;
        let ambient_intensity = self.camera.ambient_intensity;
        self.camera = CameraState {
            yaw,
            pitch,
            zoom,
            target: current_target,
            sun_dir,
            sun_intensity,
            ambient_intensity,
        }
        .clamped();
        self.temporal_reset_pending = true;
    }

    pub fn set_camera_pan(&mut self, target_x: f32, target_y: f32, target_z: f32) {
        self.camera.target = [target_x, target_y, target_z];
        self.temporal_reset_pending = true;
    }

    pub fn set_camera_target(
        &mut self,
        yaw: f32,
        pitch: f32,
        zoom: f32,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) {
        let sun_dir = self.camera.sun_dir;
        let sun_intensity = self.camera.sun_intensity;
        let ambient_intensity = self.camera.ambient_intensity;
        self.camera = CameraState {
            yaw,
            pitch,
            zoom,
            target: [target_x, target_y, target_z],
            sun_dir,
            sun_intensity,
            ambient_intensity,
        }
        .clamped();
        self.temporal_reset_pending = true;
    }

    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.clear_color = [r, g, b, a];
        self.sky_enabled = false;
    }

    pub fn clear_color(&self) -> [f64; 4] {
        self.clear_color
    }

    pub fn set_lighting(
        &mut self,
        sun_x: f32,
        sun_y: f32,
        sun_z: f32,
        sun_intensity: f32,
        ambient_intensity: f32,
    ) {
        self.camera.sun_dir = [sun_x, sun_y, sun_z];
        self.camera.sun_intensity = sun_intensity;
        self.camera.ambient_intensity = ambient_intensity;
    }

    pub fn set_sky_preset(&mut self, preset: u32) {
        let profile = AtmospherePreset::from_id(preset);
        self.clear_color = profile.clear_rgba.map(f64::from);
        self.sky_enabled = true;
        self.camera.sun_dir = profile.sun_direction;
        self.camera.sun_intensity = profile.sun_radiance;
        self.camera.ambient_intensity = profile.ambient_irradiance;
        self.set_atmosphere_preset(preset);
    }

    /// Apply only the shared atmospheric fog profile, leaving the selected clear/sky mode intact.
    pub fn set_atmosphere_preset(&mut self, preset: u32) {
        let profile = AtmospherePreset::from_id(preset);
        let atmosphere = atmosphere::AtmosphereUniform::from_profile(profile);
        self.queue.write_buffer(
            &self.atmosphere_uniform_buf,
            0,
            bytemuck::bytes_of(&atmosphere),
        );
    }
    pub fn set_standpoint(&mut self, observer: ObserverStandpoint) {
        self.observer = observer;
    }

    pub fn observer_standpoint(&self) -> ObserverStandpoint {
        self.observer
    }

    pub fn camera_state(&self) -> CameraState {
        self.camera
    }

    /// Configured surface/depth size. The swapchain texture follows the canvas backing store,
    /// so callers compare this to `canvas.width()/height()` and `resize()` on divergence —
    /// otherwise color and depth attachments mismatch and the render pass fails validation.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Access the shared GPU device (test/diagnostic helper).
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Access the shared GPU queue (test/diagnostic helper).
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Optional renderer-owned temporal resolve. It is absent when the device
    /// cannot admit the bounded history targets; callers must preserve the
    /// existing output path in that case.
    pub fn temporal_resolve_gpu(&self) -> Option<&temporal_resolve_gpu::TemporalResolveGpu> {
        self.temporal_resolve.as_ref()
    }

    /// Mutable temporal owner for hosts that have real linear-depth,
    /// motion-vector, and reactive-mask producer views to submit.
    pub fn temporal_resolve_gpu_mut(
        &mut self,
    ) -> Option<&mut temporal_resolve_gpu::TemporalResolveGpu> {
        self.temporal_resolve.as_mut()
    }

    /// The renderer-owned scene-colour attachment that can be sampled by temporal resolve.
    ///
    /// The direct surface/offscreen target is not assumed to be texture-bindable. The bloom HDR
    /// target or the SDR output-chain scene target is therefore the only concrete scene producer
    /// exposed here.
    pub fn temporal_scene_color_view(&self) -> Option<&wgpu::TextureView> {
        self.bloom
            .as_ref()
            .map(|bloom| &bloom.hdr_view)
            .or_else(|| self.output_chain.as_ref().map(output_pass::OutputChain::scene_view))
    }

    /// Report producer ownership before a host attempts a temporal submission.
    ///
    /// The temporal owner converts `SceneDepthOwner` into a renderer-owned linear-depth colour
    /// target. Motion vectors and reactive masks remain unavailable until a real producer is
    /// supplied by the host.
    pub fn temporal_producer_availability(&self) -> TemporalProducerAvailability {
        TemporalProducerAvailability {
            scene_color: self.temporal_scene_color_view().is_some(),
            linear_depth: self.temporal_resolve.is_some(),
            motion_vectors: false,
            reactive_mask: false,
        }
    }

    /// Whether the renderer-owned temporal history currently contains a published frame.
    pub fn temporal_history_valid(&self) -> bool {
        self.temporal_resolve
            .as_ref()
            .is_some_and(temporal_resolve_gpu::TemporalResolveGpu::history_valid)
    }

    /// State of the current renderer-owned temporal command handoff.
    pub fn temporal_submission_state(&self) -> TemporalSubmissionState {
        self.temporal_resolve
            .as_ref()
            .map_or(TemporalSubmissionState::Idle, |temporal| {
                temporal.submission_state()
            })
    }

    /// Whether the next admitted temporal frame must start without previous history.
    pub fn temporal_reset_pending(&self) -> bool {
        self.temporal_reset_pending
    }

    /// Drop temporal history after a camera cut, seek, or host-owned discontinuity.
    pub fn invalidate_temporal_history(&mut self) {
        self.temporal_reset_pending = true;
        self.previous_camera_view_projection = None;
        if let Some(temporal) = self.temporal_resolve.as_mut() {
            temporal.invalidate_history();
        }
    }

    /// Current camera view-projection matrix (column-major).
    pub fn current_camera_view_projection(&self) -> crate::render::temporal_producers::Mat4ColumnMajor {
        let aspect = self.width as f32 / self.height.max(1) as f32;
        let raw = crate::render::camera::orbit_view_projection_target(
            self.camera.yaw,
            self.camera.pitch,
            self.camera.zoom,
            self.camera.target,
            aspect,
        );
        crate::render::temporal_producers::Mat4ColumnMajor::from_cols(raw)
    }

    /// Previous frame's camera view-projection matrix, if history is valid.
    pub fn previous_camera_view_projection(&self) -> Option<crate::render::temporal_producers::Mat4ColumnMajor> {
        self.previous_camera_view_projection
    }

    pub(super) fn effective_temporal_schedule(
        &self,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
    ) -> crate::render::frame_graph::TemporalOutputSchedule {
        if self.temporal_reset_pending && schedule.enabled {
            schedule.with_reset_history()
        } else {
            schedule
        }
    }

    /// Record a scheduled temporal output using real host producer views.
    ///
    /// `PortalGpu::render` intentionally does not fabricate motion vectors, reactive masks, or
    /// linear-depth views. Hosts that own those attachments call this seam with their command
    /// encoder before submission. It records resolve → history publication → final output and
    /// returns `false` without recording when the optional owner is unavailable or the schedule
    /// is not a complete temporal handoff.
    pub fn record_scheduled_temporal_output(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        inputs: temporal_resolve_gpu::TemporalResolveInputs<'_, '_>,
        target: &wgpu::TextureView,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
    ) -> bool {
        let schedule = self.effective_temporal_schedule(schedule);
        let Some(temporal) = self.temporal_resolve.as_mut() else {
            return false;
        };
        let recorded = temporal.record_scheduled_output(
            &self.device,
            &self.queue,
            encoder,
            inputs,
            target,
            schedule,
            self.hdr_exposure_ev,
            self.white_balance_gains,
            self.color_format.is_srgb(),
        );
        if recorded {
            self.temporal_reset_pending = false;
        }
        recorded
    }

}
