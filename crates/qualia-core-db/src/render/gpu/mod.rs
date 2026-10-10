//! Cross-platform WebGPU viewport for the Qualia renderer SDK.
//!
//! Phenomenal viewport: projector (depth write) → ambient → optional T2 Kawase bloom.

mod ao;
mod atmosphere;
mod instance_stream;
mod instance_visibility;
mod material_draws;
mod material_textures;
mod materials;
mod picking;
pub use instance_stream::MeshInstanceUploadError;
mod texture_precomputed;
mod texture_residency;
pub use texture_mips::AlphaCoverageDiagnostics;
pub use texture_mips::TextureMipSemantic;
pub use texture_residency::{TextureColorSpace, TextureUploadError};
pub use temporal_resolve_gpu::{
    TemporalProducerAvailability, TemporalProducerViews, TemporalResolveGpu, TemporalResolveInputs,
    TemporalSubmissionState,
};
mod mesh_normals;
mod mesh_upload;
mod output_pass;
mod shadows;
mod sky;
mod scene_depth;
mod temporal_resolve_gpu;
mod texture_mips;
mod water;

use crate::gpu_context::{
    ambient_draw_instances, global_vram_ledger, universe_orchestrator, ComputeUniverse,
    OperationalMode,
};
use crate::render::camera::CameraState;
use crate::render::pga::{motor_to_mat4_col, Motor};
use crate::render::physics::{Aabb, Joint};
use crate::render::telemetry::{
    AmbientUniforms, CameraUniform, ObserverStandpoint, ParticleInstance, SystemTelemetry,
};
use crate::tensor::buffer_export::{
    read_tensor_at, tensor_node_count, TENSOR_STRIDE,
};

use std::sync::Arc;
use wgpu::util::DeviceExt;

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
thread_local! {
    static PORTAL_GPU_INIT_ABORTED: std::cell::Cell<bool> = std::cell::Cell::new(false);
    static PORTAL_GPU_CANVAS_CLAIMED: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn reset_portal_gpu_init_flags() {
    PORTAL_GPU_INIT_ABORTED.with(|c| c.set(false));
    PORTAL_GPU_CANVAS_CLAIMED.with(|c| c.set(false));
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn abort_portal_gpu_init() {
    PORTAL_GPU_INIT_ABORTED.with(|c| c.set(true));
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn portal_gpu_init_aborted() -> bool {
    PORTAL_GPU_INIT_ABORTED.with(|c| c.get())
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn portal_gpu_canvas_claimed() -> bool {
    PORTAL_GPU_CANVAS_CLAIMED.with(|c| c.get())
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
fn mark_portal_gpu_canvas_claimed() {
    PORTAL_GPU_CANVAS_CLAIMED.with(|c| c.set(true));
}

/// Static ambient SSBO capacity — draw count is throttled per `VramLedger` mode.
const MAX_AMBIENT_INSTANCES: usize = 50_000;

pub(super) fn create_ao_prepass_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("qualia-ao-prepass-camera-layout"),
        entries: &[
            uniform_128_bind_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
            uniform_128_bind_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ao::AoUniform>() as u64,
                    ),
                },
                count: None,
            },
        ],
    })
}

pub(super) fn create_ao_prepass_camera_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    observer: &wgpu::Buffer,
    ao: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("qualia-ao-prepass-camera-and-uniform"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: observer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: ao.as_entire_binding(),
            },
        ],
    })
}

pub(super) fn create_mesh_frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let uniform_entry = |binding, visibility, size| wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(size),
        },
        count: None,
    };
    let depth_texture_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let float_texture_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let entries = [
        uniform_128_bind_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
        uniform_128_bind_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
        depth_texture_entry(2),
        depth_texture_entry(3),
        wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        },
        uniform_entry(
            5,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            std::mem::size_of::<shadows::ShadowUniform>() as u64,
        ),
        float_texture_entry(6),
        float_texture_entry(7),
        uniform_entry(
            8,
            wgpu::ShaderStages::FRAGMENT,
            std::mem::size_of::<ao::AoUniform>() as u64,
        ),
        uniform_entry(
            9,
            wgpu::ShaderStages::FRAGMENT,
            std::mem::size_of::<atmosphere::AtmosphereUniform>() as u64,
        ),
    ];
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("portal-mesh-frame-lighting-layout"),
        entries: &entries,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_mesh_frame_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    observer: &wgpu::Buffer,
    shadow_views: [&wgpu::TextureView; 2],
    shadow_sampler: &wgpu::Sampler,
    shadow_uniform: &wgpu::Buffer,
    ao_view: &wgpu::TextureView,
    ao_surface_view: &wgpu::TextureView,
    ao_uniform: &wgpu::Buffer,
    atmosphere_uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-mesh-frame-lighting-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: observer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(shadow_views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(shadow_views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: shadow_uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(ao_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(ao_surface_view),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: ao_uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: atmosphere_uniform.as_entire_binding(),
            },
        ],
    })
}

pub(super) fn portal_fixed_buffer_bytes() -> Option<u64> {
    [
        std::mem::size_of::<AmbientUniforms>(),
        std::mem::size_of::<SystemTelemetry>(),
        std::mem::size_of::<CameraUniform>(),
        std::mem::size_of::<ObserverStandpoint>(),
        std::mem::size_of::<[[f32; 4]; 4]>(),
        std::mem::size_of::<shadows::ShadowUniform>(),
        std::mem::size_of::<ao::AoUniform>(),
    ]
    .into_iter()
    .try_fold(0u64, |total, size| total.checked_add(size as u64))
}
const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const BLOOM_THRESHOLD: f32 = 1.0;
const BLOOM_INTENSITY: f32 = 1.15;
const BLOOM_STRENGTH: f32 = 0.85;
const BLOOM_EXPOSURE: f32 = 1.05;
const KAWASE_OFFSETS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomParamsGpu {
    threshold: f32,
    intensity: f32,
    offset: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeParamsGpu {
    exposure: f32,
    bloom_strength: f32,
    surface_is_srgb: f32,
    _pad1: f32,
    white_balance_gains: [f32; 3],
    _pad2: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomUniformBlock {
    bloom: BloomParamsGpu,
    composite: CompositeParamsGpu,
}

/// HDR scene target + half-res Kawase ping-pong (allocated only in `OperationalMode::Full`).
struct BloomChain {
    hdr_texture: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    blur_a: wgpu::Texture,
    blur_a_view: wgpu::TextureView,
    blur_b: wgpu::Texture,
    blur_b_view: wgpu::TextureView,
    dummy_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    bind_layout: wgpu::BindGroupLayout,
    extract_pipeline: wgpu::RenderPipeline,
    kawase_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    half_width: u32,
    half_height: u32,
    exposure_scale: f32,
    surface_is_srgb: bool,
    white_balance_gains: [f32; 3],
    _frame_target_reservation: crate::gpu_context::VramReservation<'static>,
}

impl BloomChain {
    /// HDR scene target extent in pixels.
    pub(super) fn hdr_extent(&self) -> (u32, u32) {
        (self.hdr_texture.width(), self.hdr_texture.height())
    }

    /// Half-resolution Kawase ping-pong extent.
    pub(super) fn blur_extent(&self) -> (u32, u32) {
        (self.half_width, self.half_height)
    }

    /// Keep texture handles alive for resize validation / VRAM accounting.
    pub(super) fn texture_handles(&self) -> (&wgpu::Texture, &wgpu::Texture, &wgpu::Texture) {
        (&self.hdr_texture, &self.blur_a, &self.blur_b)
    }
}

/// WebGPU phenomenal viewport — tensor projector + ambient particles.
/// GPU buffers for an imported triangle mesh (Phase 1.2). Positions are model-space `f32x3`
/// (already centred + scaled to the orbit frame by the caller); `index_count` is `triangles * 3`.
struct MeshGpu {
    vertex_buf: wgpu::Buffer,
    color_buf: wgpu::Buffer,
    normal_buf: wgpu::Buffer,
    tangent_buf: wgpu::Buffer,
    uv_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    vertex_count: u32,
    material_gpu: materials::MaterialGpu,
    shadow_draws: Vec<materials::MaterialDraw>,
    receives_shadows: bool,
    // Cold-path source geometry retained so pose updates can refresh normals
    // without allocating or reading back the GPU buffers.
    cpu_positions: Vec<[f32; 3]>,
    cpu_indices: Vec<u32>,
    normal_workspace: mesh_normals::MeshNormalWorkspace,
}

fn default_gpu_tangent(normal: [f32; 3]) -> [f32; 4] {
    let axis = if normal[0].abs() < 0.8 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let cross = [
        normal[1] * axis[2] - normal[2] * axis[1],
        normal[2] * axis[0] - normal[0] * axis[2],
        normal[0] * axis[1] - normal[1] * axis[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length > f32::EPSILON {
        [cross[0] / length, cross[1] / length, cross[2] / length, 1.0]
    } else {
        [1.0, 0.0, 0.0, 1.0]
    }
}

pub struct PortalGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    device_lost: Arc<std::sync::atomic::AtomicBool>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    offscreen_texture: Option<wgpu::Texture>,
    /// Cached view of the offscreen texture — avoids `create_view` per frame.
    /// Invalidated (set to `None`) when the texture is recreated (resize).
    offscreen_view: Option<wgpu::TextureView>,
    readback_buf: Option<wgpu::Buffer>,
    readback_bytes_per_row: u32,
    _readback_staging_reservation: Option<crate::gpu_context::VramReservation<'static>>,
    color_format: wgpu::TextureFormat,
    scene_depth: scene_depth::SceneDepthOwner,
    temporal_resolve: Option<temporal_resolve_gpu::TemporalResolveGpu>,
    temporal_reset_pending: bool,
    previous_camera_view_projection: Option<crate::render::temporal_producers::Mat4ColumnMajor>,
    picking_texture: wgpu::Texture,
    picking_view: wgpu::TextureView,
    _frame_target_reservation: crate::gpu_context::VramReservation<'static>,
    _fixed_buffer_reservation: crate::gpu_context::VramReservation<'static>,
    picking_pipeline: wgpu::RenderPipeline,
    mesh_picking_pipeline: wgpu::RenderPipeline,
    pick_staging_buf: wgpu::Buffer,
    _pick_staging_reservation: crate::gpu_context::VramReservation<'static>,
    pending_pick: Option<(u32, u32)>,
    pick_copy_submitted: bool,
    pick_map_rx: Option<std::sync::mpsc::Receiver<bool>>,
    pick_result: Option<u32>,
    pick_semantic_result: Option<u64>,
    pick_semantic_count: usize,
    pick_instance_semantics: Vec<u64>,
    ambient_pipeline: wgpu::RenderPipeline,
    projector_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    ambient_pipeline_hdr: Option<wgpu::RenderPipeline>,
    projector_pipeline_hdr: Option<wgpu::RenderPipeline>,
    sky_pipeline_hdr: Option<wgpu::RenderPipeline>,
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_pipeline_blend: wgpu::RenderPipeline,
    mesh_pipeline_hdr: Option<wgpu::RenderPipeline>,
    mesh_pipeline_hdr_blend: Option<wgpu::RenderPipeline>,
    mesh_material_layout: wgpu::BindGroupLayout,
    mesh_texture_layout: wgpu::BindGroupLayout,
    mesh_frame_layout: wgpu::BindGroupLayout,
    mesh_frame_bind: wgpu::BindGroup,
    ao_prepass_camera_layout: wgpu::BindGroupLayout,
    ao_prepass_camera_bind: wgpu::BindGroup,
    shadow_target: Option<shadows::ShadowTarget>,
    shadow_enabled: bool,
    shadow_map_dirty: bool,
    shadow_uniform_buf: wgpu::Buffer,
    shadow_uniform_bind: wgpu::BindGroup,
    shadow_sampler: wgpu::Sampler,
    shadow_uniform_cpu: shadows::ShadowUniform,
    shadow_pipelines: [wgpu::RenderPipeline; shadows::SHADOW_CASCADE_COUNT],
    mesh_model_layout: wgpu::BindGroupLayout,
    mesh_instance_layout: wgpu::BindGroupLayout,
    /// Complete source stream retained for shadow-caster correctness.
    mesh_instances: instance_stream::GpuInstanceStream,
    /// Camera-visible compact stream used by the forward and AO passes.
    visible_mesh_instances: instance_stream::GpuInstanceStream,
    /// Bounded CPU-side source/scratch storage. These vectors are allocated at renderer creation;
    /// camera-driven culling and compaction reuse them without frame-time heap growth.
    mesh_instance_source: Vec<crate::render::instance_culling::GpuInstanceRecord>,
    visible_instance_indices: Vec<u32>,
    visible_instance_scratch: Vec<crate::render::instance_culling::GpuInstanceRecord>,
    last_visibility_key: Option<([[f32; 4]; 4], [[f32; 4]; 4])>,
    ao_targets: Option<ao::AoTargets>,
    ao_uniform_buf: wgpu::Buffer,
    atmosphere_uniform_buf: wgpu::Buffer,
    ao_enabled: bool,
    ao_radius: f32,
    ao_strength: f32,
    ao_bias: f32,
    ao_sample_count: u32,
    material_texture_defaults: material_textures::MaterialTextureDefaults,
    mip_generator: texture_mips::MipGenerator,
    resident_textures: texture_residency::ResidentTextureMap,
    mesh: Option<MeshGpu>,
    mesh_reservation: Option<crate::gpu_context::VramReservation<'static>>,
    water: water::WaterGpu,
    model_buf: wgpu::Buffer,
    mesh_model_bind: wgpu::BindGroup,
    artefact_joint: Option<Joint>,
    /// Sim-time at which the current joint was engaged; the joint is driven by *elapsed* time
    /// (`time − artefact_t0`), not absolute sim-time, so a slide/spin always starts from rest when
    /// armed (set lazily on the first frame after `set_artefact_joint`).
    artefact_t0: Option<f32>,
    mesh_base_aabb: Option<Aabb>,
    artefact_world: Option<Aabb>,
    last_admitted: Motor,
    last_refused: bool,
    bloom: Option<BloomChain>,
    output_chain: Option<output_pass::OutputChain>,
    bloom_policy_snapshot: bool,
    hdr_exposure_ev: f32,
    white_balance_gains: [f32; 3],
    ambient_bind_group_layout: wgpu::BindGroupLayout,
    ambient_bind_group: wgpu::BindGroup,
    projector_tensor_layout: wgpu::BindGroupLayout,
    projector_camera_bind: wgpu::BindGroup,
    projector_tensor_bind: Option<wgpu::BindGroup>,
    uniform_buf: wgpu::Buffer,
    telemetry_buf: wgpu::Buffer,
    camera_buf: wgpu::Buffer,
    observer_buf: wgpu::Buffer,
    camera: CameraState,
    observer: ObserverStandpoint,
    particle_buf: wgpu::Buffer,
    _particle_reservation: crate::gpu_context::VramReservation<'static>,
    tensor_raw_buf: Option<wgpu::Buffer>,
    _tensor_field_reservation: Option<crate::gpu_context::VramReservation<'static>>,
    tensor_node_count: u32,
    /// Whether semantic Tensor10D nodes are projected as visible sprites.
    /// The tensor remains resident for picking when this is false.
    tensor_projection_enabled: bool,
    particle_count: u32,
    /// Whether the ambient particle field is drawn. **Off by default** and opt-in: the field is
    /// the tensor-node particle cloud (`generate_particles` / uploaded tensor); the mixer's
    /// "ambient" channel (or an explicit `set_ambient_enabled(true)`) turns it on. A Tensor10D
    /// upload never forces it — hosts that want the particle view enable it explicitly.
    ambient_enabled: bool,
    /// WebGPU compute dispatch state: pipeline cache + pending readback slot
    /// (plan §7.3 W6 — `Render.gpu_compute_dispatch` / `Render.gpu_compute_readback`).
    compute: compute::ComputeState,
    /// EMF 5D volumetric visualizer state (plan §7.3 W4).
    emf: emf_pipeline::EmfState,
    /// Pre-allocated uniform belt for zero-alloc buffer writes (VC3).
    /// Replaces per-frame `queue.write_buffer` calls (which each allocate
    /// a temporary staging buffer) with writes into a pre-mapped ring of
    /// staging buffers. Pool size 3 ensures the oldest buffer's copy has
    /// completed by the time we wrap around.
    uniform_belt: uniform_belt::UniformBelt,
    width: u32,
    height: u32,
    clear_color: [f64; 4],
    sky_enabled: bool,
}

impl PortalGpu {

    #[cfg(test)]
    pub(crate) fn uniform_belt_write_and_unmap(&mut self, data: &[u8]) {
        self.uniform_belt.write_and_unmap(data);
    }
    #[cfg(test)]
    pub(crate) fn uniform_belt_record_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: wgpu::BufferAddress,
    ) {
        self.uniform_belt.record_copy(encoder, target, offset);
    }
    #[cfg(test)]
    pub(crate) fn uniform_belt_advance(&mut self) {
        self.uniform_belt.advance(&self.device);
    }
    #[cfg(test)]
    pub(crate) fn uniform_buf_test(&self) -> wgpu::Buffer {
        self.uniform_buf.clone()
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(u32, u32), String> {
        if width == 0 || height == 0 {
            return Ok((self.width, self.height));
        }
        let offscreen = self.surface.is_none();
        let (width, height, frame_target_reservation, readback_staging_reservation) =
            reserve_view_resources(width, height, offscreen)?;
        let mut next_offscreen_texture = None;
        let mut next_offscreen_view = None;
        let mut next_readback_buf = None;
        let mut next_readback_bytes_per_row = self.readback_bytes_per_row;
        if offscreen {
            let texture = create_offscreen_texture(&self.device, self.color_format, width, height);
            next_offscreen_view =
                Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            next_offscreen_texture = Some(texture);
            next_readback_bytes_per_row = checked_padded_bytes_per_row(width)
                .ok_or_else(|| format!("readback row pitch overflow for width {width}"))?;
            next_readback_buf = Some(create_readback_buffer(
                &self.device,
                next_readback_bytes_per_row,
                height,
            ));
        }
        let temporal_was_enabled = self.temporal_resolve.take().is_some();
        self.scene_depth.replace(&self.device, width, height);
        if temporal_was_enabled {
            self.temporal_resolve = temporal_resolve_gpu::TemporalResolveGpu::try_new(
                &self.device,
                width,
                height,
                self.color_format,
                self.color_format,
            );
        }
        let (picking_texture, picking_view) = create_picking_texture(&self.device, width, height);
        if let (Some(surface), Some(config)) = (self.surface.as_ref(), self.config.as_mut()) {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
        self.width = width;
        self.height = height;
        if offscreen {
            self.offscreen_texture = next_offscreen_texture;
            self.offscreen_view = next_offscreen_view;
            self.readback_bytes_per_row = next_readback_bytes_per_row;
            self.readback_buf = next_readback_buf;
            self._readback_staging_reservation = readback_staging_reservation;
        }
        self.picking_texture = picking_texture;
        self.picking_view = picking_view;
        self._frame_target_reservation = frame_target_reservation;
        self.temporal_reset_pending = true;
        self.previous_camera_view_projection = None;
        self.rebuild_ao_resources();
        self.sync_bloom_targets();
        Ok((width, height))
    }

    /// Reconcile the HDR bloom or portable SDR output chain with current budgets and policy.
    pub fn sync_bloom_targets(&mut self) {
        let bloom_wanted = portal_bloom_enabled() && probe_hdr_format(&self.device);
        if bloom_wanted {
            if let Some(ref bloom) = self.bloom {
                if bloom.hdr_extent() == (self.width, self.height) {
                    self.output_chain = None;
                    self.bloom_policy_snapshot = portal_bloom_enabled();
                    return;
                }
            }
        } else if let Some(ref output) = self.output_chain {
            if output.extent() == (self.width, self.height) {
                self.bloom = None;
                self.bloom_policy_snapshot = portal_bloom_enabled();
                return;
            }
        }

        // Release the alternate path before admitting the replacement. If the
        // preferred path does not fit, construct the SDR output path below.
        self.output_chain = None;
        if bloom_wanted {
            let mut bloom = create_bloom_chain(
                &self.device,
                self.width,
                self.height,
                self.color_format,
                self.hdr_exposure_ev,
            );
            if let Some(chain) = bloom.as_mut() {
                chain.white_balance_gains = self.white_balance_gains;
            }
            if let Some(ref chain) = bloom {
                let (hw, hh) = chain.hdr_extent();
                let (bw, bh) = chain.blur_extent();
                debug_assert_eq!(hw, self.width);
                debug_assert_eq!(hh, self.height);
                debug_assert_eq!(bw, (self.width / 2).max(1));
                debug_assert_eq!(bh, (self.height / 2).max(1));
                let _ = chain.texture_handles();
            }
            if bloom.is_some() {
                self.bloom = bloom;
                self.bloom_policy_snapshot = portal_bloom_enabled();
                return;
            }
        }

        self.bloom = None;
        let hdr_scene = self.ambient_pipeline_hdr.is_some()
            && self.projector_pipeline_hdr.is_some()
            && self.sky_pipeline_hdr.is_some()
            && self.mesh_pipeline_hdr.is_some();
        self.output_chain = output_pass::OutputChain::try_new(
            &self.device,
            self.width,
            self.height,
            if hdr_scene {
                HDR_FORMAT
            } else {
                self.color_format
            },
            self.color_format,
            self.hdr_exposure_ev,
            hdr_scene,
            self.white_balance_gains,
        );
        self.bloom_policy_snapshot = portal_bloom_enabled();
    }

}

// Phase 0.2a: render/gpu submodules (bloom post-pass, resource builders, particle field).
mod bloom;
mod compute;
mod emf_pipeline;
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
mod native_device;
mod particles;
mod resources;
mod uniform_belt;
mod portal_gpu;

/// Maximum ordered material draws admitted for one resident mesh in the portable renderer.
pub const MAX_GPU_MATERIAL_DRAWS: usize = 65_536;

use bloom::*;
pub use compute::{ComputeBinding, ComputeBufferKind};
pub use emf_pipeline::{EmfFieldCell, EmfSliceUniform};
pub use particles::particle_cap_for_mode;
use particles::*;
use resources::*;


#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "picking_tests.rs"]
mod picking_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mesh_pixel_tests.rs"]
mod mesh_pixel_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "material_pixel_tests.rs"]
mod material_pixel_tests;
