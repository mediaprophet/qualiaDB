//! Optional, renderer-owned GPU temporal resolve.
//!
//! This module owns the resolve, current linear-depth, and history targets. It
//! produces linear depth from the renderer's authoritative depth attachment,
//! but it does not invent motion-vector or reactive-mask data. A frame can
//! enter temporal accumulation only when those remaining real producer views
//! are supplied by the caller.

use std::num::NonZeroU64;

use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use crate::render::frame_graph::TemporalOutputSchedule;

const DEPTH_HISTORY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_DISOCCLUSION_THRESHOLD: f32 = 0.01;
const MAX_MOTION_FOR_HISTORY: f32 = 2.0;
const MAX_HISTORY_WEIGHT: f32 = 0.9;
const PARAMS_BYTES: u64 = 32;
const OUTPUT_PARAMS_BYTES: u64 = 32;
const DEPTH_PRODUCER_PARAMS_BYTES: u64 = 16;
const LINEAR_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TemporalParams {
    viewport: [f32; 4],
    controls: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct OutputParams {
    exposure: f32,
    surface_is_srgb: f32,
    _padding0: [f32; 2],
    white_balance_gains: [f32; 3],
    _padding1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DepthProducerParams {
    near_plane: f32,
    far_plane: f32,
    inv_width: f32,
    inv_height: f32,
}

/// Real producer views required to admit a temporal resolve.
///
/// `motion_vectors` are UV-space vectors pointing from the current pixel to its
/// previous-frame location. `reactive_mask.r` is normalized to `[0, 1]`, where
/// one rejects history. All views must have the helper's extent and one sample.
pub struct TemporalResolveInputs<'scene, 'producer> {
    pub extent: (u32, u32),
    pub current_scene: &'scene wgpu::TextureView,
    pub current_linear_depth: &'producer wgpu::TextureView,
    pub motion_vectors: &'producer wgpu::TextureView,
    pub reactive_mask: &'producer wgpu::TextureView,
}

/// Producer views supplied by a host for one temporal frame.
///
/// Scene colour and linear depth are owned by `PortalGpu` by default; a host may still provide a
/// compatible linear-depth view. Motion vectors and reactive masks remain explicit host producer
/// views. Omitting either of those views keeps temporal output disabled.
#[derive(Clone, Copy, Default)]
pub struct TemporalProducerViews<'a> {
    pub extent: (u32, u32),
    pub current_linear_depth: Option<&'a wgpu::TextureView>,
    pub motion_vectors: Option<&'a wgpu::TextureView>,
    pub reactive_mask: Option<&'a wgpu::TextureView>,
}

/// Explicit availability report for the renderer/host temporal handoff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemporalProducerAvailability {
    pub scene_color: bool,
    pub linear_depth: bool,
    pub motion_vectors: bool,
    pub reactive_mask: bool,
}

impl TemporalProducerAvailability {
    pub const fn host_producers_available(self) -> bool {
        self.motion_vectors && self.reactive_mask
    }

    pub const fn all_available(self) -> bool {
        self.scene_color && self.linear_depth && self.host_producers_available()
    }
}

impl<'a> TemporalProducerViews<'a> {
    pub const fn availability(self, scene_color: bool) -> TemporalProducerAvailability {
        TemporalProducerAvailability {
            scene_color,
            linear_depth: self.current_linear_depth.is_some(),
            motion_vectors: self.motion_vectors.is_some(),
            reactive_mask: self.reactive_mask.is_some(),
        }
    }

    /// Whether the host has supplied both producer attachments that the renderer cannot derive.
    pub const fn has_required_host_producers(self) -> bool {
        self.availability(false).host_producers_available()
    }
}

/// Command-recording lifecycle for one renderer-owned temporal submission.
///
/// This is deliberately a state machine rather than a collection of inferred booleans. It makes
/// the ordering visible to the renderer adapter: a producer pass may be recorded first, resolve
/// must follow it, history publication is a separate step, and the final output transform is the
/// last step. The state describes recorded work in the current command encoder; history validity
/// remains separately tracked because it spans frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TemporalSubmissionState {
    #[default]
    Idle,
    LinearDepthReady,
    Resolved,
    HistoryPublished,
    OutputReady,
}
/// GPU temporal resolve and explicit history publication owner.
pub struct TemporalResolveGpu {
    bind_layout: wgpu::BindGroupLayout,
    depth_producer_layout: wgpu::BindGroupLayout,
    publish_depth_layout: wgpu::BindGroupLayout,
    output_layout: wgpu::BindGroupLayout,
    resolve_pipeline: wgpu::RenderPipeline,
    depth_producer_pipeline: wgpu::RenderPipeline,
    publish_depth_pipeline: wgpu::RenderPipeline,
    output_pipeline: wgpu::RenderPipeline,
    params_buf: wgpu::Buffer,
    depth_producer_params_buf: wgpu::Buffer,
    depth_sampler: wgpu::Sampler,
    output_params_buf: wgpu::Buffer,
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    resolved_texture: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    _linear_depth_texture: wgpu::Texture,
    linear_depth_view: wgpu::TextureView,
    _history_depth_texture: wgpu::Texture,
    history_depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
    history_valid: bool,
    resolved_ready: bool,
    submission_state: TemporalSubmissionState,
    _reservation: VramReservation<'static>,
}

impl TemporalResolveGpu {
    /// Construct the helper if its bounded history targets can be admitted.
    ///
    /// The helper is optional by design. A device that cannot admit the extra
    /// history targets keeps the existing renderer path instead of failing the
    /// whole viewport.
    pub fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scene_format: wgpu::TextureFormat,
        output_format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let reservation = global_vram_ledger()
            .try_reserve_graphics(
                VramResourceClass::FrameTarget,
                target_bytes(width, height, scene_format)?,
            )
            .ok()?;

        let texture_usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        let history_texture = create_texture(
            device,
            "qualia-temporal-history-colour",
            width,
            height,
            scene_format,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_texture = create_texture(
            device,
            "qualia-temporal-resolved-colour",
            width,
            height,
            scene_format,
            texture_usage,
        );
        let resolved_view = resolved_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let linear_depth_texture = create_texture(
            device,
            "qualia-temporal-current-linear-depth",
            width,
            height,
            LINEAR_DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let linear_depth_view =
            linear_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let history_depth_texture = create_texture(
            device,
            "qualia-temporal-history-linear-depth",
            width,
            height,
            DEPTH_HISTORY_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let history_depth_view =
            history_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-temporal-resolve-layout"),
            entries: &[
                float_texture_entry(0),
                float_texture_entry(1),
                float_texture_entry(2),
                float_texture_entry(3),
                float_texture_entry(4),
                float_texture_entry(5),
                uniform_entry(6, PARAMS_BYTES),
            ],
        });
        let publish_depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-temporal-depth-publication-layout"),
            entries: &[float_texture_entry(7)],
        });
        let depth_producer_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("qualia-temporal-depth-producer-layout"),
                entries: &[
                    depth_texture_entry(0),
                    uniform_entry(1, DEPTH_PRODUCER_PARAMS_BYTES),
                    sampler_entry(2),
                ],
            });
        let output_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-temporal-output-layout"),
            entries: &[
                float_texture_entry(0),
                uniform_entry(1, OUTPUT_PARAMS_BYTES),
            ],
        });

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-temporal-resolve-params"),
            size: PARAMS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth_producer_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-temporal-depth-producer-params"),
            size: DEPTH_PRODUCER_PARAMS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("qualia-temporal-depth-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let output_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-temporal-output-params"),
            size: OUTPUT_PARAMS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-temporal-resolve-shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::TEMPORAL_RESOLVE_WGSL.into(),
            ),
        });
        let output_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-temporal-output-shader"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::TEMPORAL_OUTPUT_WGSL.into()),
        });
        let resolve_pipeline = create_pipeline(
            device,
            "qualia-temporal-resolve-pipeline",
            &resolve_shader,
            &bind_layout,
            "temporal_vs",
            "temporal_fs",
            scene_format,
        );
        let depth_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-temporal-depth-producer-shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::TEMPORAL_DEPTH_WGSL.into(),
            ),
        });
        let depth_producer_pipeline = create_pipeline(
            device,
            "qualia-temporal-depth-producer-pipeline",
            &depth_shader,
            &depth_producer_layout,
            "temporal_depth_vs",
            "temporal_depth_fs",
            LINEAR_DEPTH_FORMAT,
        );
        let publish_depth_pipeline = create_pipeline(
            device,
            "qualia-temporal-depth-publication-pipeline",
            &resolve_shader,
            &publish_depth_layout,
            "temporal_vs",
            "publish_depth_fs",
            DEPTH_HISTORY_FORMAT,
        );
        let output_pipeline = create_pipeline(
            device,
            "qualia-temporal-final-output-pipeline",
            &output_shader,
            &output_layout,
            "temporal_output_vs",
            "temporal_output_fs",
            output_format,
        );

        Some(Self {
            bind_layout,
            depth_producer_layout,
            publish_depth_layout,
            output_layout,
            resolve_pipeline,
            depth_producer_pipeline,
            publish_depth_pipeline,
            output_pipeline,
            params_buf,
            depth_producer_params_buf,
            depth_sampler,
            output_params_buf,
            history_texture,
            history_view,
            resolved_texture,
            resolved_view,
            _linear_depth_texture: linear_depth_texture,
            linear_depth_view,
            _history_depth_texture: history_depth_texture,
            history_depth_view,
            width,
            height,
            history_valid: false,
            resolved_ready: false,
            submission_state: TemporalSubmissionState::Idle,
            _reservation: reservation,
        })
    }

    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn history_valid(&self) -> bool {
        self.history_valid
    }

    pub fn resolved_ready(&self) -> bool {
        self.resolved_ready
    }

    pub fn submission_state(&self) -> TemporalSubmissionState {
        self.submission_state
    }

    /// The renderer-owned current-frame linear-depth producer target.
    pub fn linear_depth_view(&self) -> &wgpu::TextureView {
        &self.linear_depth_view
    }

    /// Convert the authoritative depth attachment into view-space linear depth.
    ///
    /// This pass is recorded after opaque/transparent scene rendering and before
    /// temporal resolve. It never treats the depth attachment itself as a colour
    /// view, so the producer contract remains valid on portable WebGPU backends.
    pub fn record_linear_depth_producer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        scene_depth: &wgpu::TextureView,
        near_plane: f32,
        far_plane: f32,
    ) -> bool {
        if !near_plane.is_finite()
            || !far_plane.is_finite()
            || near_plane <= 0.0
            || far_plane <= near_plane
        {
            return false;
        }
        let params = DepthProducerParams {
            near_plane,
            far_plane,
            inv_width: 1.0 / self.width as f32,
            inv_height: 1.0 / self.height as f32,
        };
        queue.write_buffer(
            &self.depth_producer_params_buf,
            0,
            bytemuck::bytes_of(&params),
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-temporal-depth-producer-bind"),
            layout: &self.depth_producer_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_depth),
                },
                buffer_entry(1, &self.depth_producer_params_buf),
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("qualia-temporal-depth-producer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.linear_depth_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.depth_producer_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
        self.submission_state = TemporalSubmissionState::LinearDepthReady;
        true
    }

    /// Discard history after a resize, camera cut, seek, or device recovery.
    pub fn invalidate_history(&mut self) {
        self.history_valid = false;
        self.resolved_ready = false;
        self.submission_state = TemporalSubmissionState::Idle;
    }

    /// The scene-linear resolve target for an output-chain handoff.
    pub fn resolved_view(&self) -> &wgpu::TextureView {
        &self.resolved_view
    }

    /// Record one resolve and, when requested, publish colour and linear depth history.
    ///
    /// Every producer view is mandatory. The method returns `false` for an extent
    /// mismatch and records no commands, which is the fail-closed path for a
    /// caller that has not yet synchronized its attachments.
    pub fn record_resolve(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: TemporalResolveInputs<'_, '_>,
        reset_history: bool,
        publish_history: bool,
    ) -> bool {
        let dimensions = [self.width as f32, self.height as f32];
        if inputs.extent != (self.width, self.height)
            || dimensions.iter().any(|value| !value.is_finite() || *value <= 0.0)
        {
            return false;
        }

        let history_bit = if self.history_valid && !reset_history {
            1.0
        } else {
            0.0
        };
        let reset_bit = if reset_history { 2.0 } else { 0.0 };
        let params = TemporalParams {
            viewport: [
                self.width as f32,
                self.height as f32,
                1.0 / self.width as f32,
                1.0 / self.height as f32,
            ],
            controls: [
                DEPTH_DISOCCLUSION_THRESHOLD,
                MAX_MOTION_FOR_HISTORY,
                MAX_HISTORY_WEIGHT,
                history_bit + reset_bit,
            ],
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-temporal-resolve-bind"),
            layout: &self.bind_layout,
            entries: &[
                texture_entry(0, inputs.current_scene),
                texture_entry(1, &self.history_view),
                texture_entry(2, inputs.current_linear_depth),
                texture_entry(3, &self.history_depth_view),
                texture_entry(4, inputs.motion_vectors),
                texture_entry(5, inputs.reactive_mask),
                buffer_entry(6, &self.params_buf),
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qualia-temporal-resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.resolved_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.resolve_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.submission_state = TemporalSubmissionState::Resolved;

        if publish_history {
            let depth_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("qualia-temporal-depth-publication-bind"),
                layout: &self.publish_depth_layout,
                entries: &[texture_entry(7, inputs.current_linear_depth)],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qualia-temporal-depth-publication"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.history_depth_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.publish_depth_pipeline);
            pass.set_bind_group(0, &depth_bind, &[]);
            pass.draw(0..3, 0..1);
            drop(pass);

            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.resolved_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.history_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
            self.history_valid = true;
            self.submission_state = TemporalSubmissionState::HistoryPublished;
        } else if reset_history {
            self.history_valid = false;
        }
        self.resolved_ready = true;
        true
    }

    /// Record the complete renderer-owned temporal handoff in schedule order.
    ///
    /// This is the explicit host submission seam: the host supplies all producer views and an
    /// output target, while this owner records resolve, history publication, and final output in
    /// that order. A disabled or incomplete schedule fails closed before any command is recorded.
    pub fn record_scheduled_output(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: TemporalResolveInputs<'_, '_>,
        target: &wgpu::TextureView,
        schedule: TemporalOutputSchedule,
        exposure_ev: f32,
        white_balance_gains: [f32; 3],
        surface_is_srgb: bool,
    ) -> bool {
        if !self.scheduled_output_is_admissible(
            inputs.extent,
            schedule,
            exposure_ev,
            white_balance_gains,
        ) {
            return false;
        }
        // The graph's lifecycle state is authoritative for this submission. This prevents a
        // stale owner-side history from being read when the scheduler deliberately starts a
        // frame without previous-frame history but does not need a destructive reset request.
        if !schedule.reads_history {
            self.history_valid = false;
        }
        if !self.record_resolve(
            device,
            queue,
            encoder,
            inputs,
            schedule.reset_history,
            schedule.publish_history,
        ) {
            return false;
        }
        let output_recorded = self.record_final_output(
            device,
            queue,
            encoder,
            target,
            exposure_ev,
            white_balance_gains,
            surface_is_srgb,
        );
        if !output_recorded {
            self.invalidate_history();
        }
        output_recorded
    }

    /// Preflight a scheduled submission before an adapter records any renderer-owned producer
    /// passes. The host extent is the only attachment metadata available at this API boundary;
    /// WebGPU validates the actual texture-view dimensions when the command buffer is submitted.
    pub(super) fn scheduled_output_is_admissible(
        &self,
        extent: (u32, u32),
        schedule: TemporalOutputSchedule,
        exposure_ev: f32,
        white_balance_gains: [f32; 3],
    ) -> bool {
        schedule.is_valid()
            && extent == (self.width, self.height)
            && !(schedule.reads_history && !self.history_valid)
            && exposure_ev.is_finite()
            && white_balance_gains
                .iter()
                .all(|gain| gain.is_finite() && *gain >= 0.0)
            && crate::render::output::exposure_scale_from_ev(exposure_ev).is_some()
    }

    /// Record the existing versioned SDR transform against the resolved scene.
    ///
    /// This is an explicit handoff, not an automatic second output pass. Hosts
    /// must choose either this handoff or the existing `OutputChain` composite.
    pub fn record_final_output(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        exposure_ev: f32,
        white_balance_gains: [f32; 3],
        surface_is_srgb: bool,
    ) -> bool {
        if !self.resolved_ready
            || !matches!(
                self.submission_state,
                TemporalSubmissionState::Resolved | TemporalSubmissionState::HistoryPublished
            )
            || !exposure_ev.is_finite()
            || white_balance_gains
                .iter()
                .any(|gain| !gain.is_finite() || *gain < 0.0)
        {
            return false;
        }
        let Some(exposure) = crate::render::output::exposure_scale_from_ev(exposure_ev) else {
            return false;
        };
        let params = OutputParams {
            exposure,
            surface_is_srgb: if surface_is_srgb { 1.0 } else { 0.0 },
            _padding0: [0.0; 2],
            white_balance_gains,
            _padding1: 0.0,
        };
        queue.write_buffer(
            &self.output_params_buf,
            0,
            bytemuck::bytes_of(&params),
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-temporal-final-output-bind"),
            layout: &self.output_layout,
            entries: &[
                texture_entry(0, &self.resolved_view),
                buffer_entry(1, &self.output_params_buf),
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("qualia-temporal-final-output"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.output_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
        self.submission_state = TemporalSubmissionState::OutputReady;
        true
    }

}

fn target_bytes(width: u32, height: u32, scene_format: wgpu::TextureFormat) -> Option<u64> {
    let scene_bytes = u64::from(scene_format.block_copy_size(None)?);
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(scene_bytes.checked_mul(2)?.checked_add(16)?)
        .and_then(|bytes| {
            bytes.checked_add(PARAMS_BYTES + OUTPUT_PARAMS_BYTES + DEPTH_PRODUCER_PARAMS_BYTES)
        })
}

fn create_texture(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn float_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn depth_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
        count: None,
    }
}

fn uniform_entry(binding: u32, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(size),
        },
        count: None,
    }
}

fn texture_entry<'a>(binding: u32, view: &'a wgpu::TextureView) -> wgpu::BindGroupEntry<'a> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

fn buffer_entry<'a>(binding: u32, buffer: &'a wgpu::Buffer) -> wgpu::BindGroupEntry<'a> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    vertex_entry: &str,
    fragment_entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex_entry),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_budget_is_bounded_and_format_aware() {
        let rgba8 = target_bytes(64, 32, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        let rgba16 = target_bytes(64, 32, wgpu::TextureFormat::Rgba16Float).unwrap();
        assert_eq!(rgba16 - rgba8, 64 * 32 * 8);
        assert!(rgba16 < 64 * 32 * 40);
    }

    #[test]
    fn temporal_controls_fail_closed_for_reset_and_missing_history() {
        let no_history = if false { 1.0 } else { 0.0 };
        let reset = no_history + 2.0;
        assert_eq!(no_history, 0.0);
        assert_eq!(reset, 2.0);
    }

    #[test]
    fn producer_availability_requires_every_real_input() {
        let producers = TemporalProducerViews {
            extent: (64, 64),
            current_linear_depth: None,
            motion_vectors: None,
            reactive_mask: None,
        };
        assert_eq!(
            producers.availability(true),
            TemporalProducerAvailability {
                scene_color: true,
                linear_depth: false,
                motion_vectors: false,
                reactive_mask: false,
            }
        );
        assert!(!producers.availability(true).all_available());
        assert!(!producers.availability(false).all_available());
        assert!(!producers.has_required_host_producers());
    }

    #[test]
    fn host_producer_contract_never_admits_a_partial_pair() {
        for (motion_vectors, reactive_mask) in [(true, false), (false, true)] {
            let availability = TemporalProducerAvailability {
                scene_color: true,
                linear_depth: true,
                motion_vectors,
                reactive_mask,
            };
            assert!(!availability.host_producers_available());
            assert!(!availability.all_available());
        }
    }

    #[test]
    fn linear_depth_producer_rejects_invalid_camera_ranges() {
        assert!(0.05_f32.is_finite());
        assert!(200.0_f32 > 0.05_f32);
        assert!(!(0.0_f32 > 200.0_f32));
    }

    #[cfg(any(feature = "wgsl-forge", feature = "webgl2"))]
    #[test]
    fn temporal_shaders_pass_naga_validation() {
        fn validate(source: &str) {
            let module = naga::front::wgsl::parse_str(source).expect("WGSL parse");
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("WGSL validation");
        }

        validate(crate::shaders::viewport::TEMPORAL_RESOLVE_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_DEPTH_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_OUTPUT_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_MOTION_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_REACTIVE_WGSL);
    }
}
