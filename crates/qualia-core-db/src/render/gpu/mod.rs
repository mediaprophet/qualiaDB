//! U2 WebGPU viewport for the Qualia WASM portal (wasm32).
//!
//! Phenomenal viewport: projector (depth write) → ambient → optional T2 Kawase bloom.

use crate::gpu_context::{
    ambient_draw_instances, global_vram_ledger, universe_orchestrator, ComputeUniverse,
    OperationalMode,
};
use crate::render::camera::CameraState;
use crate::render::navigation::PICK_SENTINEL;
use crate::render::pga::{motor_to_mat4_col, Motor};
use crate::render::physics::{Aabb, Admission, Joint};
use crate::render::standpoint::spectator_default;
use crate::render::telemetry::{AmbientUniforms, ObserverStandpoint, ParticleInstance, SystemTelemetry};
use crate::shaders::viewport::{AMBIENT_WGSL, BLOOM_WGSL, MESH_WGSL, PROJECTOR_WGSL};
use crate::tensor::buffer_export::{
    read_tensor_at, tensor_node_count, TENSOR_HEADER_BYTES, TENSOR_STRIDE,
};

use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Static ambient SSBO capacity — draw count is throttled per `VramLedger` mode.
const MAX_AMBIENT_INSTANCES: usize = 50_000;
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
    _pad0: f32,
    _pad1: f32,
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
    vram_bytes: u64,
}

/// WebGPU phenomenal viewport — tensor projector + ambient particles.
/// GPU buffers for an imported triangle mesh (Phase 1.2). Positions are model-space `f32x3`
/// (already centred + scaled to the orbit frame by the caller); `index_count` is `triangles * 3`.
struct MeshGpu {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
}

pub struct PortalGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    picking_texture: wgpu::Texture,
    picking_view: wgpu::TextureView,
    picking_pipeline: wgpu::RenderPipeline,
    pick_staging_buf: wgpu::Buffer,
    pending_pick: Option<(u32, u32)>,
    pick_copy_submitted: bool,
    pick_result: Option<u32>,
    ambient_pipeline: wgpu::RenderPipeline,
    projector_pipeline: wgpu::RenderPipeline,
    ambient_pipeline_hdr: Option<wgpu::RenderPipeline>,
    projector_pipeline_hdr: Option<wgpu::RenderPipeline>,
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_pipeline_hdr: Option<wgpu::RenderPipeline>,
    mesh: Option<MeshGpu>,
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
    ambient_bind_group_layout: wgpu::BindGroupLayout,
    ambient_bind_group: wgpu::BindGroup,
    projector_camera_layout: wgpu::BindGroupLayout,
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
    tensor_raw_buf: Option<wgpu::Buffer>,
    tensor_node_count: u32,
    particle_count: u32,
    width: u32,
    height: u32,
}

impl PortalGpu {
    /// Native sync wrapper around the async initialiser (`block_on` traps in browser WASM, so the
    /// browser path must call `try_new_async` and await it instead).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_new(canvas: &web_sys::HtmlCanvasElement, particle_cap: usize) -> Result<Self, String> {
        pollster::block_on(Self::try_new_async(canvas, particle_cap))
    }

    /// Async WebGPU init — awaits `request_adapter` / `request_device` (the browser main thread
    /// cannot block). Native callers use the `try_new` wrapper above.
    pub async fn try_new_async(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let _ = particle_cap;
        let particle_count = MAX_AMBIENT_INSTANCES.max(256);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "no WebGPU adapter".to_string())?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("qualia-portal-gpu"),
                    required_features: wgpu::Features::empty(),
                    // WebGPU baseline limits — NOT downlevel_webgl2_defaults, which set
                    // max_storage_buffers_per_shader_stage = 0 and silently invalidate both portal
                    // pipelines (their vertex shaders read the tensor SOA / particle storage
                    // buffers), turning the viewport black. Any BROWSER_WEBGPU adapter supports the
                    // baseline. The limits-shim strips fields Chrome doesn't recognise.
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("device: {e}"))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // DIAG: capture deferred pipeline/shader creation validation errors.
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let caps = surface.get_capabilities(&adapter);
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
        };
        surface.configure(&device, &config);

        let (depth_texture, depth_view) = create_depth_texture(&device, width, height);
        let (picking_texture, picking_view) = create_picking_texture(&device, width, height);

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
                    uniform_128_bind_entry(0, wgpu::ShaderStages::VERTEX),
                    uniform_128_bind_entry(1, wgpu::ShaderStages::VERTEX_FRAGMENT),
                ],
            });

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
                bind_group_layouts: &[&ambient_bind_group_layout],
                push_constant_ranges: &[],
            });

        let projector_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("portal-projector-pipeline-layout"),
                bind_group_layouts: &[&projector_camera_layout, &projector_tensor_layout],
                push_constant_ranges: &[],
            });

        let ambient_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-ambient-pipeline"),
            layout: Some(&ambient_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ambient_shader,
                entry_point: "vertex_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ambient_shader,
                entry_point: "fragment_main",
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil_state_read_only()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let projector_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-projector-pipeline"),
            layout: Some(&projector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &projector_shader,
                entry_point: "vertex_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &projector_shader,
                entry_point: "fragment_main",
                compilation_options: Default::default(),
                targets: &[Some(color_target_state(format))],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
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

        // Triangle-mesh pipeline (Phase 1.2). Reuses the projector camera bind layout (mesh shader
        // declares only camera@0, a valid subset); one f32x3 vertex buffer at slot 0; cull disabled
        // (imported meshes carry inconsistent winding). HDR variant built in the bloom block below.
        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portal-mesh-pipeline-layout"),
            bind_group_layouts: &[&projector_camera_layout, &mesh_model_layout],
            push_constant_ranges: &[],
        });
        let mesh_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-mesh-pipeline"),
            layout: Some(&mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: "vertex_main",
                compilation_options: Default::default(),
                buffers: &[mesh_vertex_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: "fragment_main",
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
            multiview: None,
        });

        let picking_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-picking-pipeline"),
            layout: Some(&projector_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &projector_shader,
                entry_point: "vertex_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &projector_shader,
                entry_point: "picking_fragment_main",
                compilation_options: Default::default(),
                targets: &[Some(picking_color_target_state())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(depth_state.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pick_staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("portal-pick-staging"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bloom_wanted = portal_bloom_enabled() && probe_hdr_format(&device);
        let (ambient_pipeline_hdr, projector_pipeline_hdr, mesh_pipeline_hdr, bloom) = if bloom_wanted {
            let ambient_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-ambient-hdr"),
                layout: Some(&ambient_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &ambient_shader,
                    entry_point: "vertex_main",
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ambient_shader,
                    entry_point: "fragment_main",
                    compilation_options: Default::default(),
                    targets: &[Some(hdr_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(depth_stencil_state_read_only()),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            let projector_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-projector-hdr"),
                layout: Some(&projector_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &projector_shader,
                    entry_point: "vertex_main",
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &projector_shader,
                    entry_point: "fragment_main",
                    compilation_options: Default::default(),
                    targets: &[Some(hdr_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            let mesh_hdr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("portal-mesh-hdr"),
                layout: Some(&mesh_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: "vertex_main",
                    compilation_options: Default::default(),
                    buffers: &[mesh_vertex_layout.clone()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: "fragment_main",
                    compilation_options: Default::default(),
                    targets: &[Some(hdr_color_target_state())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state.clone()),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            let bloom = create_bloom_chain(&device, width, height, format);
            (Some(ambient_hdr), Some(projector_hdr), Some(mesh_hdr), bloom)
        } else {
            (None, None, None, None)
        };

        let mut render_bytes = (particle_count * std::mem::size_of::<ParticleInstance>()) as u64;
        if let Some(ref chain) = bloom {
            render_bytes += chain.vram_bytes;
        }
        global_vram_ledger().record_render(render_bytes);

        // Surface otherwise-silent deferred pipeline/shader creation errors. Dawn (WebGPU) is far
        // stricter than the native backends, so a pipeline that builds on desktop can be invalid in
        // the browser and silently render nothing; log it instead of leaving a black viewport.
        let scope_err = device.pop_error_scope().await;
        #[cfg(target_arch = "wasm32")]
        if let Some(err) = &scope_err {
            web_sys::console::error_1(
                &format!("[portal_gpu] pipeline/shader creation error: {err}").into(),
            );
        }
        let _ = &scope_err;

        Ok(Self {
            device,
            queue,
            surface,
            config,
            depth_texture,
            depth_view,
            picking_texture,
            picking_view,
            picking_pipeline,
            pick_staging_buf,
            pending_pick: None,
            pick_copy_submitted: false,
            pick_result: None,
            ambient_pipeline,
            projector_pipeline,
            ambient_pipeline_hdr,
            projector_pipeline_hdr,
            mesh_pipeline,
            mesh_pipeline_hdr,
            mesh: None,
            model_buf,
            mesh_model_bind,
            artefact_joint: None,
            artefact_t0: None,
            mesh_base_aabb: None,
            artefact_world: None,
            last_admitted: Motor::identity(),
            last_refused: false,
            bloom,
            ambient_bind_group_layout,
            ambient_bind_group,
            projector_camera_layout,
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
            tensor_raw_buf: None,
            tensor_node_count: 0,
            particle_count: particle_count as u32,
            width,
            height,
        })
    }

    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<u32, String> {
        let (header, _) = crate::tensor::buffer_export::parse_header(bytes)
            .map_err(|e| e.to_string())?;
        let count = header.node_count;
        if count == 0 {
            return Ok(0);
        }

        let particles = particles_from_tensor(bytes, MAX_AMBIENT_INSTANCES)?;
        let instance_count = particles.len() as u32;

        let particle_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-tensor-particles"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Upload the SOA *body* only (skip the 32-byte header). WebGPU requires storage-buffer
        // binding offsets to be a multiple of minStorageBufferOffsetAlignment (256), so we cannot
        // bind at offset 32 the way native backends allow — start the buffer at the first record
        // and bind at offset 0.
        let body = bytes
            .get(TENSOR_HEADER_BYTES..)
            .ok_or_else(|| "tensor buffer shorter than header".to_string())?;
        let tensor_raw_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-tensor-raw-soa"),
            contents: body,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        self.ambient_bind_group = make_ambient_bind_group(
            &self.device,
            &self.ambient_bind_group_layout,
            &self.uniform_buf,
            &self.telemetry_buf,
            &self.camera_buf,
            &self.observer_buf,
            &particle_buf,
        );

        self.projector_tensor_bind = Some(make_projector_tensor_bind_group(
            &self.device,
            &self.projector_tensor_layout,
            &tensor_raw_buf,
            count,
        )?);

        let particle_bytes = (particles.len() * std::mem::size_of::<ParticleInstance>()) as u64;
        global_vram_ledger().record_render(particle_bytes);
        global_vram_ledger().record_tensor(bytes.len() as u64);

        self.particle_buf = particle_buf;
        self.tensor_raw_buf = Some(tensor_raw_buf);
        self.tensor_node_count = count;
        self.particle_count = instance_count.max(1);

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

    /// Resolve this frame's per-artefact model transform: the joint pose at `time`, gated through
    /// the admission policy (refuse out-of-world → hold the last admitted pose), then write it.
    fn update_model(&mut self, time: f32) {
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
                proposed
            }
        };
        let model = motor_to_mat4_col(motor);
        self.queue
            .write_buffer(&self.model_buf, 0, bytemuck::cast_slice(&model));
    }

    pub fn has_tensor_buffer(&self) -> bool {
        self.tensor_raw_buf.is_some()
    }

    /// Upload an imported triangle mesh (Phase 1.2). `positions` are model-space `f32x3` (the caller
    /// centres + scales them to the orbit frame); `indices` is a flat triangle list (`tris * 3`).
    /// Returns the triangle count; clears any prior mesh when empty.
    pub fn upload_mesh(&mut self, positions: &[[f32; 3]], indices: &[u32]) -> u32 {
        if positions.is_empty() || indices.len() < 3 {
            self.mesh = None;
            return 0;
        }
        let vertex_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-mesh-verts"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-mesh-indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_count = indices.len() as u32;
        self.mesh = Some(MeshGpu { vertex_buf, index_buf, index_count });
        self.mesh_base_aabb = Aabb::from_points(positions); // for Phase 2 admission
        self.last_admitted = Motor::identity();
        self.last_refused = false;
        index_count / 3
    }

    /// Whether a mesh surface is resident.
    pub fn has_mesh(&self) -> bool {
        self.mesh.is_some()
    }

    pub fn set_camera(&mut self, yaw: f32, pitch: f32, zoom: f32) {
        self.camera = CameraState { yaw, pitch, zoom }.clamped();
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

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (depth_texture, depth_view) = create_depth_texture(&self.device, width, height);
        let (picking_texture, picking_view) = create_picking_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
        self.picking_texture = picking_texture;
        self.picking_view = picking_view;
        self.sync_bloom_targets();
    }

    /// Reconcile HDR bloom textures with current `VramLedger` operational mode.
    pub fn sync_bloom_targets(&mut self) {
        if portal_bloom_enabled() && probe_hdr_format(&self.device) {
            let bloom = create_bloom_chain(&self.device, self.width, self.height, self.config.format);
            let bloom_bytes = bloom.as_ref().map(|b| b.vram_bytes).unwrap_or(0);
            let particle_bytes =
                (self.particle_count as usize * std::mem::size_of::<ParticleInstance>()) as u64;
            global_vram_ledger().record_render(particle_bytes + bloom_bytes);
            self.bloom = bloom;
        } else {
            let particle_bytes =
                (self.particle_count as usize * std::mem::size_of::<ParticleInstance>()) as u64;
            global_vram_ledger().record_render(particle_bytes);
            self.bloom = None;
        }
    }

    fn write_camera_uniform(&self, time: f32) {
        let aspect = self.width as f32 / self.height.max(1) as f32;
        let mut uniform = self
            .camera
            .to_uniform(aspect, self.tensor_raw_buf.is_some());
        uniform._padding[0] = time;
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&uniform));
    }

    fn write_observer_uniform(&self) {
        self.queue
            .write_buffer(&self.observer_buf, 0, bytemuck::bytes_of(&self.observer));
    }

    pub fn queue_pick(&mut self, x: f32, y: f32) {
        let px = x.round().max(0.0) as u32;
        let py = y.round().max(0.0) as u32;
        self.pending_pick = Some((
            px.min(self.width.saturating_sub(1)),
            py.min(self.height.saturating_sub(1)),
        ));
        self.pick_copy_submitted = false;
        self.pick_result = None;
    }

    pub fn poll_pick_readback(&mut self) -> Option<u32> {
        if let Some(idx) = self.pick_result.take() {
            return Some(idx);
        }
        if !self.pick_copy_submitted {
            return None;
        }
        let slice = self.pick_staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        if !matches!(rx.try_recv(), Ok(Ok(()))) {
            return None;
        }
        let mapped = slice.get_mapped_range();
        let raw = if mapped.len() >= 4 {
            Some(u32::from_le_bytes(mapped[0..4].try_into().unwrap()))
        } else {
            None
        };
        drop(mapped);
        self.pick_staging_buf.unmap();
        self.pick_copy_submitted = false;
        raw.filter(|&id| id != PICK_SENTINEL)
    }

    fn record_picking_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(tensor_bind) = self.projector_tensor_bind.as_ref() else {
            return;
        };
        let count = self.tensor_node_count;
        if count == 0 {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("portal-picking-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.picking_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: PICK_SENTINEL as f64,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.picking_pipeline);
        pass.set_bind_group(0, &self.projector_camera_bind, &[]);
        pass.set_bind_group(1, tensor_bind, &[]);
        pass.draw(0..6, 0..count);
    }

    fn record_pick_copy(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some((px, py)) = self.pending_pick.take() else {
            return;
        };
        let py = self.height.saturating_sub(1) - py;
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.picking_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: px, y: py, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.pick_staging_buf,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.pick_copy_submitted = true;
    }

    pub fn render(&mut self, time: f32, telemetry: &SystemTelemetry) -> Result<(), String> {
        let uniforms = AmbientUniforms {
            time,
            view_width: self.width as f32,
            view_height: self.height as f32,
            _padding: 0.0,
        };
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));
        self.queue
            .write_buffer(&self.telemetry_buf, 0, bytemuck::bytes_of(telemetry));
        self.write_camera_uniform(time);
        self.write_observer_uniform();
        self.update_model(time);

        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("surface frame: {e}"))?;

        // On the web backend the swapchain texture tracks the canvas backing store, which can
        // diverge from the size our depth/picking/bloom targets were built at (device-pixel ratio,
        // layout settle, a ResizeObserver resizing in CSS pixels without a matching `resize()`).
        // A depth attachment whose dimensions don't match the colour attachment fails render-pass
        // validation, the whole frame is dropped, and the viewport stays black. Reconcile every
        // attachment to the *actual* acquired texture before recording any pass.
        let fw = frame.texture.width();
        let fh = frame.texture.height();
        if fw > 0 && fh > 0 && (fw, fh) != (self.width, self.height) {
            self.width = fw;
            self.height = fh;
            self.config.width = fw;
            self.config.height = fh;
            let (depth_texture, depth_view) = create_depth_texture(&self.device, fw, fh);
            let (picking_texture, picking_view) = create_picking_texture(&self.device, fw, fh);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
            self.picking_texture = picking_texture;
            self.picking_view = picking_view;
            self.sync_bloom_targets();
            self.write_camera_uniform(time);
        }

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("portal-viewport-encoder"),
            });

        self.record_picking_pass(&mut encoder);

        let use_bloom = self.bloom.is_some()
            && self.ambient_pipeline_hdr.is_some()
            && self.projector_pipeline_hdr.is_some()
            && portal_bloom_enabled();

        if use_bloom {
            let bloom = self.bloom.as_ref().expect("bloom chain");
            let ambient_hdr = self.ambient_pipeline_hdr.as_ref().expect("ambient hdr");
            let projector_hdr = self.projector_pipeline_hdr.as_ref().expect("projector hdr");
            let mesh_hdr = self.mesh_pipeline_hdr.as_ref();

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("portal-hdr-scene"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &bloom.hdr_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                if let (Some(mesh), Some(mesh_pipe)) = (self.mesh.as_ref(), mesh_hdr) {
                    pass.set_pipeline(mesh_pipe);
                    pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                    pass.set_bind_group(1, &self.mesh_model_bind, &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                    pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }

                if let (Some(tensor_bind), count) = (
                    self.projector_tensor_bind.as_ref(),
                    self.tensor_node_count,
                ) {
                    if count > 0 {
                        pass.set_pipeline(projector_hdr);
                        pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                        pass.set_bind_group(1, tensor_bind, &[]);
                        pass.draw(0..6, 0..count);
                    }
                }

                pass.set_pipeline(ambient_hdr);
                pass.set_bind_group(0, &self.ambient_bind_group, &[]);
                let ambient_draw = ambient_draw_instances(self.particle_count);
                if ambient_draw > 0 {
                    pass.draw(0..6, 0..ambient_draw);
                }
            }

            run_bloom_passes(&mut encoder, bloom, &self.queue, &self.device, &view);
        } else {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("portal-phenomenal-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if let Some(mesh) = self.mesh.as_ref() {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                pass.set_bind_group(1, &self.mesh_model_bind, &[]);
                pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }

            if let (Some(tensor_bind), count) = (
                self.projector_tensor_bind.as_ref(),
                self.tensor_node_count,
            ) {
                if count > 0 {
                    pass.set_pipeline(&self.projector_pipeline);
                    pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                    pass.set_bind_group(1, tensor_bind, &[]);
                    pass.draw(0..6, 0..count);
                }
            }

            pass.set_pipeline(&self.ambient_pipeline);
            pass.set_bind_group(0, &self.ambient_bind_group, &[]);
            let ambient_draw = ambient_draw_instances(self.particle_count);
            if ambient_draw > 0 {
                pass.draw(0..6, 0..ambient_draw);
            }
        }

        self.record_pick_copy(&mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }
}

// Phase 0.2a: render/gpu submodules (bloom post-pass, resource builders, particle field).
mod bloom;
mod resources;
mod particles;
use bloom::*;
use resources::*;
use particles::*;
pub use particles::particle_cap_for_mode;
