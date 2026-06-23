//! U2 WebGPU viewport for the Qualia WASM portal (wasm32).
//!
//! Phenomenal viewport: projector (depth write) → ambient → optional T2 Kawase bloom.

use crate::gpu_context::{
    ambient_draw_instances, global_vram_ledger, universe_orchestrator, ComputeUniverse,
    OperationalMode,
};
use crate::portal_camera::CameraState;
use crate::portal_navigation::PICK_SENTINEL;
use crate::portal_standpoint::spectator_default;
use crate::portal_telemetry::{AmbientUniforms, ObserverStandpoint, ParticleInstance, SystemTelemetry};
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

        let session_nonce = crate::portal_standpoint::generate_session_nonce();
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

        // Triangle-mesh pipeline (Phase 1.2). Reuses the projector camera bind layout (mesh shader
        // declares only camera@0, a valid subset); one f32x3 vertex buffer at slot 0; cull disabled
        // (imported meshes carry inconsistent winding). HDR variant built in the bloom block below.
        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portal-mesh-pipeline-layout"),
            bind_group_layouts: &[&projector_camera_layout],
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

fn create_picking_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-picking"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn picking_color_target_state() -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format: wgpu::TextureFormat::R32Uint,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn depth_stencil_state(format: wgpu::TextureFormat) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn depth_stencil_state_read_only() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn color_target_state(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn uniform_128_bind_entry(binding: u32, stages: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: stages,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(128),
        },
        count: None,
    }
}

fn tensor_storage_bind_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn ambient_bind_entries() -> [wgpu::BindGroupLayoutEntry; 5] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        uniform_128_bind_entry(3, wgpu::ShaderStages::VERTEX),
        uniform_128_bind_entry(4, wgpu::ShaderStages::VERTEX_FRAGMENT),
    ]
}

fn make_ambient_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buf: &wgpu::Buffer,
    telemetry_buf: &wgpu::Buffer,
    camera_buf: &wgpu::Buffer,
    observer_buf: &wgpu::Buffer,
    particle_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-ambient-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: telemetry_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: particle_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: camera_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: observer_buf.as_entire_binding(),
            },
        ],
    })
}

fn make_projector_tensor_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    tensor_raw_buf: &wgpu::Buffer,
    node_count: u32,
) -> Result<wgpu::BindGroup, String> {
    let body_bytes = (node_count as u64)
        .checked_mul(TENSOR_STRIDE as u64)
        .ok_or_else(|| "tensor body size overflow".to_string())?;
    let size = std::num::NonZeroU64::new(body_bytes)
        .ok_or_else(|| "empty tensor body".to_string())?;
    Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-projector-tensor-bind"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: tensor_raw_buf,
                // Buffer already starts at the first record (header stripped at upload); offset 0
                // satisfies the 256-byte minStorageBufferOffsetAlignment that Dawn enforces.
                offset: 0,
                size: Some(size),
            }),
        }],
    }))
}

fn particles_from_tensor(bytes: &[u8], cap: usize) -> Result<Vec<ParticleInstance>, String> {
    let count = tensor_node_count(bytes).map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let step = (count / cap).max(1);
    let mut out = Vec::with_capacity(cap.min(count));
    for i in (0..count).step_by(step) {
        let t = read_tensor_at(bytes, i).map_err(|e| e.to_string())?;
        out.push(ParticleInstance {
            position: [t.x, t.y, t.z],
            epistemic_q: t.q,
        });
        if out.len() >= cap {
            break;
        }
    }
    Ok(out)
}

fn generate_particles(count: usize) -> Vec<ParticleInstance> {
    let mut out = Vec::with_capacity(count);
    let mut seed: u32 = 0xC0FFEE_u32;
    for _ in 0..count {
        seed = lcg(seed);
        let x = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        seed = lcg(seed);
        let y = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        seed = lcg(seed);
        let z = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        out.push(ParticleInstance {
            position: [x, y, z],
            epistemic_q: 0.0,
        });
    }
    out
}

#[inline]
fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(1_103_515_245).wrapping_add(12_345)
}

#[inline]
pub fn particle_cap_for_mode(mode: OperationalMode, tier: u8) -> usize {
    if tier < 2 {
        return 0;
    }
    // Buffer is always allocated at Full capacity; ledger throttles draw instances.
    let _ = mode;
    MAX_AMBIENT_INSTANCES
}

#[inline]
fn portal_bloom_enabled() -> bool {
    let ledger = global_vram_ledger();
    universe_orchestrator().bloom_enabled(ComputeUniverse::Viewport, ledger.mode())
}

fn probe_hdr_format(device: &wgpu::Device) -> bool {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-hdr-probe"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default());
    true
}

fn hdr_color_target_state() -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format: HDR_FORMAT,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn bloom_vram_bytes(width: u32, height: u32) -> u64 {
    let w = width.max(1) as u64;
    let h = height.max(1) as u64;
    let hdr = w * h * 8;
    let half_w = (w / 2).max(1);
    let half_h = (h / 2).max(1);
    hdr + half_w * half_h * 8 * 2
}

fn create_float_target(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn bloom_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("portal-bloom-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(32),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(32),
                },
                count: None,
            },
        ],
    })
}

fn make_bloom_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    tex_a: &wgpu::TextureView,
    tex_b: &wgpu::TextureView,
    uniform_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-bloom-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(tex_a),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(tex_b),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: uniform_buf.as_entire_binding(),
            },
        ],
    })
}

fn create_bloom_chain(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat,
) -> Option<BloomChain> {
    if !portal_bloom_enabled() {
        return None;
    }

    let half_width = (width / 2).max(1);
    let half_height = (height / 2).max(1);
    let (hdr_texture, hdr_view) = create_float_target(device, "portal-hdr", width, height);
    let (blur_a, blur_a_view) =
        create_float_target(device, "portal-bloom-a", half_width, half_height);
    let (blur_b, blur_b_view) =
        create_float_target(device, "portal-bloom-b", half_width, half_height);
    let (dummy, dummy_view) = create_float_target(device, "portal-bloom-dummy", 1, 1);

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("portal-bloom-sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("portal-bloom-uniforms"),
        contents: bytemuck::bytes_of(&BloomUniformBlock {
            bloom: BloomParamsGpu {
                threshold: BLOOM_THRESHOLD,
                intensity: BLOOM_INTENSITY,
                offset: 1.0,
                _pad: 0.0,
            },
            composite: CompositeParamsGpu {
                exposure: BLOOM_EXPOSURE,
                bloom_strength: BLOOM_STRENGTH,
                _pad0: 0.0,
                _pad1: 0.0,
            },
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_layout = bloom_bind_layout(device);
    let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("portal-bloom"),
        source: wgpu::ShaderSource::Wgsl(BLOOM_WGSL.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("portal-bloom-pipeline-layout"),
        bind_group_layouts: &[&bind_layout],
        push_constant_ranges: &[],
    });

    let extract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-extract"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: "extract_vs",
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: "extract_fs",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let kawase_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-kawase"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: "kawase_vs",
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: "kawase_fs",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-composite"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: "composite_vs",
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: "composite_fs",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let _ = dummy;

    Some(BloomChain {
        hdr_texture,
        hdr_view,
        blur_a,
        blur_a_view,
        blur_b,
        blur_b_view,
        dummy_view,
        sampler,
        uniform_buf,
        bind_layout,
        extract_pipeline,
        kawase_pipeline,
        composite_pipeline,
        half_width,
        half_height,
        vram_bytes: bloom_vram_bytes(width, height),
    })
}

fn write_bloom_uniform(queue: &wgpu::Queue, buf: &wgpu::Buffer, offset: f32) {
    let block = BloomUniformBlock {
        bloom: BloomParamsGpu {
            threshold: BLOOM_THRESHOLD,
            intensity: BLOOM_INTENSITY,
            offset,
            _pad: 0.0,
        },
        composite: CompositeParamsGpu {
            exposure: BLOOM_EXPOSURE,
            bloom_strength: BLOOM_STRENGTH,
            _pad0: 0.0,
            _pad1: 0.0,
        },
    };
    queue.write_buffer(buf, 0, bytemuck::bytes_of(&block));
}

fn run_bloom_passes(
    encoder: &mut wgpu::CommandEncoder,
    bloom: &BloomChain,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    surface_view: &wgpu::TextureView,
) {
    write_bloom_uniform(queue, &bloom.uniform_buf, 1.0);
    let extract_bind = make_bloom_bind_group(
        device,
        &bloom.bind_layout,
        &bloom.sampler,
        &bloom.hdr_view,
        &bloom.dummy_view,
        &bloom.uniform_buf,
    );
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("portal-bloom-extract"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bloom.blur_a_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&bloom.extract_pipeline);
        pass.set_bind_group(0, &extract_bind, &[]);
        pass.draw(0..3, 0..1);
    }

    let mut read = &bloom.blur_a_view;
    let mut write = &bloom.blur_b_view;
    for &offset in &KAWASE_OFFSETS {
        write_bloom_uniform(queue, &bloom.uniform_buf, offset);
        let kawase_bind = make_bloom_bind_group(
            device,
            &bloom.bind_layout,
            &bloom.sampler,
            read,
            &bloom.dummy_view,
            &bloom.uniform_buf,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("portal-bloom-kawase"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: write,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&bloom.kawase_pipeline);
            pass.set_bind_group(0, &kawase_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        std::mem::swap(&mut read, &mut write);
    }
    let bloom_result = read;

    write_bloom_uniform(queue, &bloom.uniform_buf, 1.0);
    let composite_bind = make_bloom_bind_group(
        device,
        &bloom.bind_layout,
        &bloom.sampler,
        &bloom.hdr_view,
        bloom_result,
        &bloom.uniform_buf,
    );
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("portal-bloom-composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
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
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&bloom.composite_pipeline);
        pass.set_bind_group(0, &composite_bind, &[]);
        pass.draw(0..3, 0..1);
    }

}