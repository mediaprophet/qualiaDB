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
use crate::render::atmosphere::AtmospherePreset;
use crate::render::camera::CameraState;
use crate::render::pga::{motor_to_mat4_col, Motor};
use crate::render::physics::{Aabb, Joint};
use crate::render::standpoint::spectator_default;
use crate::render::telemetry::{
    AmbientUniforms, CameraUniform, ObserverStandpoint, ParticleInstance, SystemTelemetry,
};
use crate::shaders::viewport::{AMBIENT_WGSL, BLOOM_WGSL, MESH_WGSL, PROJECTOR_WGSL};
use crate::tensor::buffer_export::{
    read_tensor_at, tensor_node_count, TENSOR_HEADER_BYTES, TENSOR_STRIDE,
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

fn create_ao_prepass_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

fn create_ao_prepass_camera_bind(
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

fn create_mesh_frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
fn create_mesh_frame_bind(
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

fn portal_fixed_buffer_bytes() -> Option<u64> {
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
    /// Build a native offscreen renderer on QualiaDB's process-wide shared GPU device.
    ///
    /// The output target is linear `Rgba8Unorm`. Call [`Self::render`] and then
    /// [`Self::read_rgba8_into`] to retrieve tightly packed pixels into a caller-owned buffer.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_offscreen(width: u32, height: u32, particle_cap: usize) -> Result<Self, String> {
        let width = width.max(1);
        let height = height.max(1);
        let explicit_renderer_backend = native_device::backend_override()?.is_some();
        let mut shared_failure = None;

        // Keep the zero-copy shared device as the normal path. A renderer-only
        // backend pin bypasses it, so graphics can select GL without changing
        // inference's process-wide backend.
        if !explicit_renderer_backend {
            if let Some(shared) = crate::gpu_context::try_shared_gpu() {
                match pollster::block_on(Self::from_device(
                    Arc::new(shared.device.clone()),
                    Arc::new(shared.queue.clone()),
                    width,
                    height,
                    wgpu::TextureFormat::Rgba8Unorm,
                    None,
                    None,
                    particle_cap,
                )) {
                    Ok(renderer) => return Ok(renderer),
                    Err(error) => shared_failure = Some(error),
                }
            }
        }

        let (device, queue) = native_device::request_offscreen_device()?;
        pollster::block_on(Self::from_device(
            device,
            queue,
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            None,
            None,
            particle_cap,
        ))
        .map_err(|error| match shared_failure {
            Some(shared) => format!(
                "shared renderer initialization failed ({shared}); native renderer fallback failed ({error})"
            ),
            None => error,
        })
    }

    /// Async offscreen WebGPU renderer on the process-wide shared device.
    /// Logic/scientific/LLM WASM packages construct this without a canvas.
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub async fn new_offscreen_async(
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        crate::gpu_context::ensure_shared_gpu().await?;
        let shared = crate::gpu_context::try_shared_gpu()
            .ok_or_else(|| "shared WebGPU device missing after ensure_shared_gpu".to_string())?;
        Self::from_device(
            Arc::new(shared.device.clone()),
            Arc::new(shared.queue.clone()),
            width.max(1),
            height.max(1),
            wgpu::TextureFormat::Rgba8Unorm,
            None,
            None,
            particle_cap,
        )
        .await
    }

    /// Build a native **surface** renderer that draws directly to a window's GPU swapchain.
    ///
    /// This is the native desktop path — no PNG round-trip, no webview `<img>`. The surface
    /// is created from a raw window handle (HWND on Windows) and frames are presented directly
    /// to the OS swapchain.
    ///
    /// The surface format is chosen from the adapter's capabilities (sRGB preferred).
    /// Call [`Self::render`] to draw a frame; the swapchain present is automatic.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn new_surface(
        hwnd: isize,
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        use raw_window_handle::{
            RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
        };

        // Create a DEDICATED instance/adapter/device for the surface renderer.
        // The shared GPU context is optimised for compute (LLM inference) and its
        // adapter may not support presentation to an HWND (e.g. Vulkan without a
        // VkSurfaceKHR, or a compute-only adapter). A dedicated instance ensures
        // the surface, adapter, and device are all from the same wgpu instance and
        // the adapter is picked with surface compatibility.
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = native_device::backend_override()?.unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(desc);

        let win32_handle =
            Win32WindowHandle::new(std::num::NonZeroIsize::new(hwnd).ok_or("invalid HWND (zero)")?);
        let raw_window = RawWindowHandle::Win32(win32_handle);
        let raw_display = RawDisplayHandle::Windows(WindowsDisplayHandle::new());

        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: Some(raw_display),
                    raw_window_handle: raw_window,
                })
                .map_err(|e| format!("create_surface from HWND: {e:?}"))?
        };

        // Request an adapter that supports the surface
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("Failed to find wgpu adapter for surface: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err(format!(
                "Surface supports no formats — adapter backend {:?} may not support presentation to HWND",
                adapter.get_info().backend
            ));
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Webizen GPU Surface"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("Failed to request device for surface: {e}"))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: surfaces declare their colour space; Auto preserves the
            // pre-30 (implicit sRGB/linear-by-format) behaviour.
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        surface.configure(&device, &config);

        pollster::block_on(Self::from_device(
            device,
            queue,
            width.max(1),
            height.max(1),
            format,
            Some(surface),
            Some(config),
            particle_cap,
        ))
    }

    /// Async WebGPU init — awaits `request_adapter` / `request_device` (the browser main thread
    /// cannot block). Native callers use the `try_new` wrapper above.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    /// True when a browser adapter answers. Does not bind a canvas, so a hang
    /// or a miss leaves the 2d tick free to draw.
    pub async fn adapter_responds() -> bool {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(instance_desc);
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .is_ok()
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    pub async fn try_new_async(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(instance_desc);

        // Probe already ran without a canvas. Claim only when we are about
        // to present, and only with an adapter that can target this surface.
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }

        mark_portal_gpu_canvas_claimed();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("surface: {e}"))?;
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        // Phones often refuse low-power or the first preference and then
        // answer the next. One miss must not drop the lit frame.
        let mut adapter = None;
        let mut last_err = String::from("no WebGPU adapter");
        for pref in [
            wgpu::PowerPreference::None,
            wgpu::PowerPreference::LowPower,
            wgpu::PowerPreference::HighPerformance,
        ] {
            if portal_gpu_init_aborted() {
                return Err("aborted".into());
            }
            match instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: pref,
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
            {
                Ok(found) => {
                    adapter = Some(found);
                    break;
                }
                Err(e) => last_err = format!("no WebGPU adapter: {e}"),
            }
        }
        let adapter = adapter.ok_or(last_err)?;

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("qualia-portal-gpu"),
                required_features: wgpu::Features::empty(),
                // Request exactly the adapter's advertised limits. `Limits::default()`
                // under wgpu 30 asks for desktop-tier limits that a browser WebGPU
                // adapter does not grant, so `request_device` would fail device
                // validation. `adapter.limits()` never over-requests and still preserves
                // the non-zero storage-buffer limits the portal pipelines need (unlike
                // `downlevel_webgl2_defaults`, which zeroes them and blacks out the view).
                required_limits: crate::gpu_context::webgpu_minimum_limits(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("device: {e}"))?;

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }

        // This device is uniquely owned by the browser portal. Never install this
        // callback on the shared native/inference device: that device has multiple
        // owners and a device-lost callback is a single-owner notification slot.
        let device_lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lost_signal = Arc::clone(&device_lost);
        device.set_device_lost_callback(move |_reason, _message| {
            lost_signal.store(true, std::sync::atomic::Ordering::Release);
        });

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err("surface has no presentable format".into());
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: Auto preserves pre-30 colour-space behaviour.
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let mut portal_gpu = Self::from_device(
            Arc::new(device),
            Arc::new(queue),
            width,
            height,
            format,
            Some(surface),
            Some(config),
            particle_cap,
        )
        .await?;
        portal_gpu.device_lost = device_lost;
        Ok(portal_gpu)
    }

    async fn from_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        surface: Option<wgpu::Surface<'static>>,
        config: Option<wgpu::SurfaceConfiguration>,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let particle_bytes_per_instance = std::mem::size_of::<ParticleInstance>() as u64;
        let particle_device_limit = u64::from(device.limits().max_storage_buffer_binding_size)
            .min(device.limits().max_buffer_size);
        let device_particle_cap =
            usize::try_from(particle_device_limit / particle_bytes_per_instance)
                .unwrap_or(usize::MAX);
        if device_particle_cap == 0 {
            return Err("device cannot bind even one ambient particle instance".into());
        }
        let mut particle_count = particle_cap
            .clamp(256, MAX_AMBIENT_INSTANCES)
            .min(device_particle_cap);
        let (width, height, frame_target_reservation, readback_staging_reservation) =
            reserve_view_resources(width, height, surface.is_none())?;
        let fixed_buffer_bytes = portal_fixed_buffer_bytes()
            .ok_or_else(|| "portal fixed-buffer size overflow".to_string())?;
        let fixed_buffer_reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FrameTarget,
                fixed_buffer_bytes,
            )
            .map_err(|e| format!("portal fixed-buffer admission failed: {e}"))?;

        // Ambient/Tensor10D instance data is optional presentation residency.
        // Preserve a reduced pool under pressure instead of refusing the whole
        // renderer; fail only when even one instance cannot be admitted.
        let particle_reservation = loop {
            let bytes = (particle_count as u64)
                .checked_mul(std::mem::size_of::<ParticleInstance>() as u64)
                .ok_or_else(|| "particle field byte size overflow".to_string())?;
            match global_vram_ledger()
                .try_reserve_graphics(crate::gpu_context::VramResourceClass::FieldResidency, bytes)
            {
                Ok(reservation) => break reservation,
                Err(_) if particle_count > 1 => particle_count = particle_count.div_ceil(2),
                Err(error) => return Err(format!("particle field admission failed: {error}")),
            }
        };

        // Capture deferred pipeline/shader creation errors on both Dawn and native backends.
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

        let scene_depth = scene_depth::SceneDepthOwner::new(&device, width, height);
        let temporal_resolve = temporal_resolve_gpu::TemporalResolveGpu::try_new(
            &device, width, height, format, format,
        );
        let shadow_target = shadows::ShadowTarget::try_new(&device);
        let (picking_texture, picking_view) = create_picking_texture(&device, width, height);
        let offscreen_texture = if surface.is_none() {
            Some(create_offscreen_texture(&device, format, width, height))
        } else {
            None
        };
        // Pre-create the offscreen texture view so the render loop doesn't
        // call `create_view` every frame (VC3 zero-alloc).
        let offscreen_view = offscreen_texture
            .as_ref()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
        // Uniform belt: pre-allocated pool of mapped staging buffers for
        // zero-alloc uniform writes (VC3). 256 bytes covers all per-frame
        // uniform structs (ambient ~16B, telemetry ~256B, camera ~96B,
        // observer ~64B, model ~64B, shadow matrix 80B, ao 64B, water 48B). Pool size 16 ensures
        // we never wrap around within a frame — the oldest
        // buffer's copy has completed by the time we wrap around.
        let uniform_belt = uniform_belt::UniformBelt::new(&device, queue.clone(), 256, 16)?;
        let readback_bytes_per_row = checked_padded_bytes_per_row(width)
            .ok_or_else(|| format!("readback row pitch overflow for width {width}"))?;
        let readback_buf = if surface.is_none() {
            Some(create_readback_buffer(
                &device,
                readback_bytes_per_row,
                height,
            ))
        } else {
            None
        };

        let particles = generate_particles(particle_count);
        let particle_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-particles"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-ambient-uniforms"),
            contents: bytemuck::bytes_of(&AmbientUniforms {
                time: 0.0,
                view_width: width as f32,
                view_height: height as f32,
                _padding: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let telemetry_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-telemetry"),
            contents: bytemuck::bytes_of(&SystemTelemetry::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera = CameraState::default();
        let aspect = width as f32 / height.max(1) as f32;
        let camera_uniform = camera.to_uniform(aspect, false);
        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-camera"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let session_nonce = crate::render::standpoint::generate_session_nonce();
        let observer = spectator_default(session_nonce);
        let observer_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-observer"),
            contents: bytemuck::bytes_of(&observer),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let ambient_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal-ambient"),
            source: wgpu::ShaderSource::Wgsl(AMBIENT_WGSL.into()),
        });

        let projector_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal-projector"),
            source: wgpu::ShaderSource::Wgsl(PROJECTOR_WGSL.into()),
        });

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal-mesh"),
            source: wgpu::ShaderSource::Wgsl(MESH_WGSL.into()),
        });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal-sun-shadow"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::SUN_SHADOW_WGSL.into()),
        });

        let ambient_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("portal-ambient-layout"),
                entries: &ambient_bind_entries(),
            });

        let ambient_bind_group = make_ambient_bind_group(
            &device,
            &ambient_bind_group_layout,
            &uniform_buf,
            &telemetry_buf,
            &camera_buf,
            &observer_buf,
            &particle_buf,
        );

        let projector_camera_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("portal-projector-camera-layout"),
                entries: &[
                    uniform_128_bind_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                    uniform_128_bind_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
                ],
            });

        let sky_pipeline = sky::create_pipeline(&device, &projector_camera_layout, format);

        let projector_tensor_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("portal-projector-tensor-layout"),
                entries: &[tensor_storage_bind_entry()],
            });

        let projector_camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-projector-camera-bind"),
            layout: &projector_camera_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: observer_buf.as_entire_binding(),
                },
            ],
        });

        let depth_state = depth_stencil_state(wgpu::TextureFormat::Depth32Float);

        let ambient_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("portal-ambient-pipeline-layout"),
                bind_group_layouts: &[Some(&ambient_bind_group_layout)],
                immediate_size: 0,
            });

        let projector_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("portal-projector-pipeline-layout"),
                bind_group_layouts: &[
                    Some(&projector_camera_layout),
                    Some(&projector_tensor_layout),
                ],
                immediate_size: 0,
            });

        let ambient_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-ambient-pipeline"),
            layout: Some(&ambient_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ambient_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ambient_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil_state_read_only()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let projector_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-projector-pipeline"),
            layout: Some(&projector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &projector_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &projector_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Per-artefact model transform (Phase 2 kinematic joint), group 1 of the mesh pipeline.
        const IDENTITY_MAT4: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let model_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-mesh-model"),
            contents: bytemuck::cast_slice(&IDENTITY_MAT4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let mesh_model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("portal-mesh-model-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(64),
                },
                count: None,
            }],
        });
        let mesh_model_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-mesh-model-bind"),
            layout: &mesh_model_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buf.as_entire_binding(),
            }],
        });
        let mesh_instance_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("portal-mesh-instance-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            crate::render::instance_culling::GpuInstanceRecord,
                        >()
                            as u64),
                    },
                    count: None,
                }],
            });
        let mesh_instances =
            instance_stream::GpuInstanceStream::new(&device, &mesh_instance_layout).map_err(
                |error| format!("mesh instance stream initialization failed: {error:?}"),
            )?;
        let visible_mesh_instances =
            instance_stream::GpuInstanceStream::new(&device, &mesh_instance_layout).map_err(
                |error| format!("visible mesh instance stream initialization failed: {error:?}"),
            )?;

        let shadow_sampler = shadows::compare_sampler(&device);
        let shadow_matrix_layout = shadows::matrix_layout(&device);
        let shadow_initial = shadows::ShadowUniform {
            light_view_projection: [IDENTITY_MAT4; shadows::SHADOW_CASCADE_COUNT],
            params: [0.0, 1.0, 0.001, 0.0],
        };
        let shadow_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-sun-shadow-uniform"),
            contents: bytemuck::bytes_of(&shadow_initial),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shadow_uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-sun-shadow-matrix-bind"),
            layout: &shadow_matrix_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_uniform_buf.as_entire_binding(),
            }],
        });
        let ao_uniform_initial = ao::make_uniform(
            width,
            height,
            [
                camera_uniform._padding[1],
                camera_uniform._padding[2],
                camera_uniform._padding[3],
            ],
            crate::render::camera::orbit_forward(camera_uniform.yaw, camera_uniform.pitch),
            0.45,
            0.55,
            0.025,
            8,
            false,
        );
        let ao_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-screen-space-ao-uniform"),
            contents: bytemuck::bytes_of(&ao_uniform_initial),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let atmosphere_profile = AtmospherePreset::from_id(0);
        let atmosphere_uniform = atmosphere::AtmosphereUniform::from_profile(atmosphere_profile);
        let atmosphere_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-atmosphere-uniform"),
            contents: bytemuck::bytes_of(&atmosphere_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let mesh_material_layout = materials::create_layout(&device);
        let mesh_texture_layout = material_textures::create_layout(&device);
        let ao_prepass_camera_layout = create_ao_prepass_camera_layout(&device);
        let ao_prepass_camera_bind = create_ao_prepass_camera_bind(
            &device,
            &ao_prepass_camera_layout,
            &camera_buf,
            &observer_buf,
            &ao_uniform_buf,
        );
        let material_texture_defaults = material_textures::create_defaults(&device, &queue)?;
        let mip_generator = texture_mips::MipGenerator::new(&device);
        let ao_targets = ao::AoTargets::try_new(
            &device,
            width,
            height,
            &ao_prepass_camera_layout,
            &mesh_model_layout,
            &mesh_instance_layout,
            &mesh_material_layout,
            &mesh_texture_layout,
            &ao_uniform_buf,
        );
        let (ao_view, ao_packed_surface_view) = ao_targets
            .as_ref()
            .map(|targets| (targets.ao_view(), targets.normal_view()))
            .unwrap_or((
                material_texture_defaults.view(3),
                material_texture_defaults.view(1),
            ));
        let mesh_frame_layout = create_mesh_frame_layout(&device);
        let shadow_views = shadow_target
            .as_ref()
            .map(|target| [&target.views[0], &target.views[1]])
            .unwrap_or([scene_depth.view(), scene_depth.view()]);
        let mesh_frame_bind = create_mesh_frame_bind(
            &device,
            &mesh_frame_layout,
            &camera_buf,
            &observer_buf,
            shadow_views,
            &shadow_sampler,
            &shadow_uniform_buf,
            ao_view,
            ao_packed_surface_view,
            &ao_uniform_buf,
            &atmosphere_uniform_buf,
        );

        // The mesh frame consolidates camera, shadow, AO and atmosphere resources into group 0;
        // factors and fixed texture slots share group 2. This keeps the whole mesh family within
        // WebGPU's portable four-group baseline without dropping any material or lighting path.
        // Position, colour and generated normal streams remain separate for future authored data.
        // Culling remains disabled until import winding has an explicit canonical contract.
        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portal-mesh-pipeline-layout"),
            bind_group_layouts: &[
                Some(&mesh_frame_layout),
                Some(&mesh_model_layout),
                Some(&mesh_material_layout),
                Some(&mesh_instance_layout),
            ],
            immediate_size: 0,
        });
        let water = water::WaterGpu::new(
            &device,
            &mesh_frame_layout,
            &mesh_model_layout,
            format,
            probe_hdr_format(&device),
        );
        let mesh_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("portal-sun-shadow-pipeline-layout"),
                bind_group_layouts: &[
                    Some(&shadow_matrix_layout),
                    Some(&mesh_model_layout),
                    Some(&mesh_material_layout),
                    Some(&mesh_instance_layout),
                ],
                immediate_size: 0,
            });
        let shadow_color_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 1,
            }],
        };
        let shadow_uv_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 2,
            }],
        };
        let shadow_pipelines = ["shadow_vertex_near", "shadow_vertex_far"].map(|entry_point| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if entry_point == "shadow_vertex_near" {
                    "portal-sun-shadow-near-depth"
                } else {
                    "portal-sun-shadow-far-depth"
                }),
                layout: Some(&shadow_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shadow_shader,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(mesh_vertex_layout.clone()),
                        Some(shadow_color_layout.clone()),
                        Some(shadow_uv_layout.clone()),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shadow_shader,
                    entry_point: Some("shadow_mask_fragment"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 1.5,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        });
        let mesh_color_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 1,
            }],
        };
        let mesh_normal_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
        };
        let mesh_tangent_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 3,
            }],
        };
        let mesh_uv_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 4,
            }],
        };
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-mesh-pipeline"),
            layout: Some(&mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(mesh_vertex_layout.clone()),
                    Some(mesh_color_layout.clone()),
                    Some(mesh_normal_layout.clone()),
                    Some(mesh_tangent_layout.clone()),
                    Some(mesh_uv_layout.clone()),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mut transparent_depth_state = depth_state.clone();
        transparent_depth_state.depth_write_enabled = Some(false);
        transparent_depth_state.depth_compare = Some(wgpu::CompareFunction::LessEqual);
        let mesh_pipeline_blend = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-mesh-transparent-pipeline"),
            layout: Some(&mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(mesh_vertex_layout.clone()),
                    Some(mesh_color_layout.clone()),
                    Some(mesh_normal_layout.clone()),
                    Some(mesh_tangent_layout.clone()),
                    Some(mesh_uv_layout.clone()),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(transparent_depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let picking_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-picking-pipeline"),
            layout: Some(&projector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &projector_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &projector_shader,
                entry_point: Some("picking_fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(picking_color_target_state())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mesh_picking_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-mesh-semantic-picking-pipeline"),
                layout: Some(&mesh_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(mesh_vertex_layout.clone()),
                        Some(mesh_color_layout.clone()),
                        Some(mesh_normal_layout.clone()),
                        Some(mesh_tangent_layout.clone()),
                        Some(mesh_uv_layout.clone()),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: Some("picking_fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(picking_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let pick_staging_size =
            u64::from(checked_padded_bytes_per_row(1).expect("one-pixel pick row always fits"));
        let pick_staging_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::UploadStaging,
                pick_staging_size,
            )
            .map_err(|error| format!("pick staging reservation failed: {error}"))?;
        let pick_staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("portal-pick-staging"),
            size: pick_staging_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let hdr_pipeline_wanted = probe_hdr_format(&device);
        let bloom_wanted = portal_bloom_enabled() && hdr_pipeline_wanted;
        let (
            ambient_pipeline_hdr,
            projector_pipeline_hdr,
            sky_pipeline_hdr,
            mesh_pipeline_hdr,
            mesh_pipeline_hdr_blend,
            bloom,
        ) = if hdr_pipeline_wanted {
            let sky_hdr = sky::create_pipeline(&device, &projector_camera_layout, HDR_FORMAT);
            let ambient_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-ambient-hdr"),
                layout: Some(&ambient_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &ambient_shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ambient_shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(hdr_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(depth_stencil_state_read_only()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
            let projector_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-projector-hdr"),
                layout: Some(&projector_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &projector_shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &projector_shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(hdr_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
            let mesh_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-mesh-hdr"),
                layout: Some(&mesh_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(mesh_vertex_layout.clone()),
                        Some(mesh_color_layout.clone()),
                        Some(mesh_normal_layout.clone()),
                        Some(mesh_tangent_layout.clone()),
                        Some(mesh_uv_layout.clone()),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state(HDR_FORMAT))],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
            let mesh_hdr_blend = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-mesh-hdr-transparent"),
                layout: Some(&mesh_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(mesh_vertex_layout.clone()),
                        Some(mesh_color_layout.clone()),
                        Some(mesh_normal_layout.clone()),
                        Some(mesh_tangent_layout.clone()),
                        Some(mesh_uv_layout.clone()),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target_state(HDR_FORMAT))],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(transparent_depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
            let bloom = if bloom_wanted {
                create_bloom_chain(
                    &device,
                    width,
                    height,
                    format,
                    crate::render::output::DEFAULT_HDR_EXPOSURE_EV,
                )
            } else {
                None
            };
            (
                Some(ambient_hdr),
                Some(projector_hdr),
                Some(sky_hdr),
                Some(mesh_hdr),
                Some(mesh_hdr_blend),
                bloom,
            )
        } else {
            (None, None, None, None, None, None)
        };
        let hdr_scene = ambient_pipeline_hdr.is_some()
            && projector_pipeline_hdr.is_some()
            && sky_pipeline_hdr.is_some()
            && mesh_pipeline_hdr.is_some();
        let output_chain = if bloom.is_none() {
            output_pass::OutputChain::try_new(
                &device,
                width,
                height,
                if hdr_scene { HDR_FORMAT } else { format },
                format,
                crate::render::output::DEFAULT_HDR_EXPOSURE_EV,
                hdr_scene,
                crate::render::output::DEFAULT_WHITE_BALANCE_GAINS,
            )
        } else {
            None
        };

        // Surface otherwise-silent deferred pipeline/shader creation errors. Dawn (WebGPU) is far
        // stricter than the native backends, so a pipeline that builds on desktop can be invalid in
        // the browser and silently render nothing; log it instead of leaving a black viewport.
        //
        // On wasm we CANNOT await this pop under wgpu 30.0.0: `GPUDevice.popErrorScope()` resolves
        // to `GPUError | null`, and on the (common) no-error path it returns JS `null`. wgpu's
        // `future_pop_error_scope` feeds that through `JsOption::into_option()`, which — since
        // wasm-bindgen 0.2.123 — treats `null` as a *present* value (only `undefined` is absent).
        // So a null (no error) becomes `Some(null)`, wgpu calls `Error::from_js(null)`, and
        // webgpu.rs:85 panics `"Unexpected error"`, aborting the module before the canvas ever
        // renders. Dropping the guard still pops the scope (discarding any captured error) but
        // never polls that buggy future, so it cannot panic. Tracked in
        // docs/WGPU_UPSTREAM_TRACKING.md for a proper soft-fork of `future_pop_error_scope`.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let scope_err = error_scope.pop().await;
            if let Some(err) = scope_err {
                return Err(format!("renderer pipeline/shader creation failed: {err}"));
            }
        }
        #[cfg(target_arch = "wasm32")]
        drop(error_scope);

        let emf_state = emf_pipeline::EmfState::new(&device, format)?;

        let identity_instance =
            crate::render::instance_culling::GpuInstanceRecord::new(IDENTITY_MAT4, 0);
        let mut mesh_instance_source =
            Vec::with_capacity(crate::render::instance_culling::MAX_GPU_MESH_INSTANCES);
        mesh_instance_source.push(identity_instance);
        let visible_instance_indices =
            vec![0; crate::render::instance_culling::MAX_GPU_MESH_INSTANCES];
        let visible_instance_scratch =
            vec![identity_instance; crate::render::instance_culling::MAX_GPU_MESH_INSTANCES];
        let pick_instance_semantics =
            vec![0; crate::render::instance_culling::MAX_GPU_MESH_INSTANCES];

        Ok(Self {
            device,
            queue,
            device_lost: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            surface,
            config,
            offscreen_texture,
            offscreen_view,
            readback_buf,
            readback_bytes_per_row,
            _readback_staging_reservation: readback_staging_reservation,
            color_format: format,
            scene_depth,
            temporal_resolve,
            temporal_reset_pending: true,
            previous_camera_view_projection: None,
            picking_texture,
            picking_view,
            _frame_target_reservation: frame_target_reservation,
            _fixed_buffer_reservation: fixed_buffer_reservation,
            picking_pipeline,
            mesh_picking_pipeline,
            pick_staging_buf,
            _pick_staging_reservation: pick_staging_reservation,
            pending_pick: None,
            pick_copy_submitted: false,
            pick_map_rx: None,
            pick_result: None,
            pick_semantic_result: None,
            pick_semantic_count: 0,
            pick_instance_semantics,
            ambient_pipeline,
            projector_pipeline,
            sky_pipeline,
            ambient_pipeline_hdr,
            projector_pipeline_hdr,
            sky_pipeline_hdr,
            mesh_pipeline,
            mesh_pipeline_blend,
            mesh_pipeline_hdr,
            mesh_pipeline_hdr_blend,
            mesh_material_layout,
            mesh_texture_layout,
            mesh_frame_layout,
            mesh_frame_bind,
            ao_prepass_camera_layout,
            ao_prepass_camera_bind,
            shadow_target,
            shadow_enabled: true,
            shadow_map_dirty: true,
            shadow_uniform_buf,
            shadow_uniform_bind,
            shadow_sampler,
            shadow_uniform_cpu: shadow_initial,
            shadow_pipelines,
            mesh_model_layout: mesh_model_layout.clone(),
            mesh_instance_layout,
            mesh_instances,
            visible_mesh_instances,
            mesh_instance_source,
            visible_instance_indices,
            visible_instance_scratch,
            last_visibility_key: None,
            ao_targets,
            ao_uniform_buf,
            atmosphere_uniform_buf,
            ao_enabled: true,
            ao_radius: 0.45,
            ao_strength: 0.55,
            ao_bias: 0.025,
            ao_sample_count: 8,
            material_texture_defaults,
            mip_generator,
            resident_textures: Default::default(),
            mesh: None,
            mesh_reservation: None,
            water,
            model_buf,
            mesh_model_bind,
            artefact_joint: None,
            artefact_t0: None,
            mesh_base_aabb: None,
            artefact_world: None,
            last_admitted: Motor::identity(),
            last_refused: false,
            bloom,
            output_chain,
            bloom_policy_snapshot: portal_bloom_enabled(),
            hdr_exposure_ev: crate::render::output::DEFAULT_HDR_EXPOSURE_EV,
            white_balance_gains: crate::render::output::DEFAULT_WHITE_BALANCE_GAINS,
            ambient_bind_group_layout,
            ambient_bind_group,
            projector_tensor_layout,
            projector_camera_bind,
            projector_tensor_bind: None,
            uniform_buf,
            telemetry_buf,
            camera_buf,
            observer_buf,
            camera,
            observer,
            particle_buf,
            _particle_reservation: particle_reservation,
            tensor_raw_buf: None,
            _tensor_field_reservation: None,
            tensor_node_count: 0,
            tensor_projection_enabled: true,
            particle_count: particle_count as u32,
            ambient_enabled: false,
            compute: compute::ComputeState::new(),
            emf: emf_state,
            uniform_belt,
            width,
            height,
            clear_color: [0.03, 0.05, 0.08, 1.0],
            sky_enabled: false,
        })
    }

    /// Device loss is signalled asynchronously by wgpu; callers poll this flag
    /// at a frame boundary before using any device-owned resource again.
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

    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<u32, String> {
        let (header, _) =
            crate::tensor::buffer_export::parse_header(bytes).map_err(|e| e.to_string())?;
        let count = header.node_count;
        if count == 0 {
            return Ok(0);
        }

        let particles = particles_from_tensor(bytes, MAX_AMBIENT_INSTANCES)?;
        let instance_count = particles.len() as u32;

        // Admit the replacement as a peak allocation while the current field
        // remains resident. Both reservations drop automatically if validation
        // or GPU resource construction fails before the state swap.
        let particle_bytes = (particles.len() as u64)
            .checked_mul(std::mem::size_of::<ParticleInstance>() as u64)
            .ok_or_else(|| "tensor particle field byte size overflow".to_string())?;
        let particle_reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FieldResidency,
                particle_bytes,
            )
            .map_err(|e| format!("tensor particle field admission failed: {e}"))?;
        let body = bytes
            .get(TENSOR_HEADER_BYTES..)
            .ok_or_else(|| "tensor buffer shorter than header".to_string())?;
        let tensor_bytes = body.len() as u64;
        let max_binding_bytes = u64::from(self.device.limits().max_storage_buffer_binding_size);
        let max_buffer_bytes = self.device.limits().max_buffer_size;
        if tensor_bytes == 0 || tensor_bytes > max_binding_bytes || tensor_bytes > max_buffer_bytes
        {
            return Err(format!(
                "tensor buffer body size {tensor_bytes} exceeds device storage limits (binding {max_binding_bytes}, buffer {max_buffer_bytes})"
            ));
        }
        let tensor_field_reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FieldResidency,
                tensor_bytes,
            )
            .map_err(|e| format!("Tensor10D field admission failed: {e}"))?;

        let particle_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-tensor-particles"),
                contents: bytemuck::cast_slice(&particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Upload the SOA *body* only (skip the 32-byte header). WebGPU requires storage-buffer
        // binding offsets to be a multiple of minStorageBufferOffsetAlignment (256), so we cannot
        // bind at offset 32 the way native backends allow — start the buffer at the first record
        // and bind at offset 0.
        let tensor_raw_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-tensor-raw-soa"),
                contents: body,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let ambient_bind_group = make_ambient_bind_group(
            &self.device,
            &self.ambient_bind_group_layout,
            &self.uniform_buf,
            &self.telemetry_buf,
            &self.camera_buf,
            &self.observer_buf,
            &particle_buf,
        );

        let projector_tensor_bind = make_projector_tensor_bind_group(
            &self.device,
            &self.projector_tensor_layout,
            &tensor_raw_buf,
            count,
        )?;

        self.ambient_bind_group = ambient_bind_group;
        self.projector_tensor_bind = Some(projector_tensor_bind);
        self.particle_buf = particle_buf;
        self.tensor_raw_buf = Some(tensor_raw_buf);
        self._particle_reservation = particle_reservation;
        self._tensor_field_reservation = Some(tensor_field_reservation);
        self.tensor_node_count = count;
        self.particle_count = instance_count.max(1);
        // The particle field now carries real tensor nodes (not the
        // decorative random cloud), but it stays opt-in: showing it is an
        // explicit presentation choice (`set_ambient_enabled`), never forced
        // by an upload. Forcing it here resurrected the debug field after a
        // GPU re-init even when the host had disabled it.

        Ok(count)
    }

    pub fn tensor_node_count(&self) -> u32 {
        self.tensor_node_count
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

    pub fn has_tensor_buffer(&self) -> bool {
        self.tensor_raw_buf.is_some()
    }

    /// Replace a span of mesh positions. The buffer is `COPY_DST`. Used so a
    /// part can change over time without uploading the whole scene again.
    pub fn write_mesh_vertices(&mut self, start: u32, positions: &[[f32; 3]]) {
        let Some(mesh) = self.mesh.as_mut() else {
            return;
        };
        let start_us = start as usize;
        let Some(end) = start_us.checked_add(positions.len()) else {
            return;
        };
        if positions.is_empty()
            || end > mesh.vertex_count as usize
            || positions.iter().flatten().any(|value| !value.is_finite())
        {
            return;
        }
        mesh.cpu_positions[start_us..end].copy_from_slice(positions);
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
        if mesh
            .normal_workspace
            .update_positions(&mesh.cpu_positions, start_us, end)
            .is_err()
        {
            return;
        }
        self.queue.write_buffer(
            &mesh.vertex_buf,
            (start_us * 12) as u64,
            bytemuck::cast_slice(positions),
        );
        let (dirty, normals) = mesh.normal_workspace.sorted_dirty_vertices_and_normals();
        if let Some(&first) = dirty.first() {
            let mut run_start = first as usize;
            let mut run_end = run_start + 1;
            for &vertex in &dirty[1..] {
                let vertex = vertex as usize;
                if vertex == run_end {
                    run_end += 1;
                    continue;
                }
                self.queue.write_buffer(
                    &mesh.normal_buf,
                    (run_start * 12) as u64,
                    bytemuck::cast_slice(&normals[run_start..run_end]),
                );
                run_start = vertex;
                run_end = vertex + 1;
            }
            self.queue.write_buffer(
                &mesh.normal_buf,
                (run_start * 12) as u64,
                bytemuck::cast_slice(&normals[run_start..run_end]),
            );
        }
    }

    /// Whether a mesh surface is resident.
    pub fn has_mesh(&self) -> bool {
        self.mesh.is_some()
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

    fn effective_temporal_schedule(
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

    /// Record temporal output using the renderer-owned scene colour attachment.
    ///
    // VC3 test helpers — expose uniform belt internals for allocation measurement.
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
