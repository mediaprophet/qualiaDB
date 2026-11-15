use crate::diffusion::DiffusionConfig;
use crate::error::RuntimeError;
use crate::kernel::ComputeBackend;
use crate::snapshot::{
    compute_state_hash, FrameHandle, SharedFrameBuffer, SimulationSnapshot, StateHash,
};
use bytemuck::{Pod, Zeroable};
use std::sync::{mpsc, Arc};
use etalon::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 8;

// Render pipeline shader (basic vertex + fragment)
const RENDER_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const DIFFUSION_SHADER: &str = r#"
struct DiffusionUniforms {
    width: u32,
    height: u32,
    diffusion_rate: f32,
    _pad: f32,
};

@group(0) @binding(0)
var<storage, read> src: array<f32>;

@group(0) @binding(1)
var<storage, read_write> dst: array<f32>;

@group(0) @binding(2)
var<uniform> uniforms: DiffusionUniforms;

fn idx(x: u32, y: u32) -> u32 {
    return y * uniforms.width + x;
}

fn left_of(x: u32) -> u32 {
    return select(x - 1u, uniforms.width - 1u, x == 0u);
}

fn right_of(x: u32) -> u32 {
    return select(x + 1u, 0u, x + 1u >= uniforms.width);
}

fn up_of(y: u32) -> u32 {
    return select(y - 1u, uniforms.height - 1u, y == 0u);
}

fn down_of(y: u32) -> u32 {
    return select(y + 1u, 0u, y + 1u >= uniforms.height);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {
        return;
    }

    let x = gid.x;
    let y = gid.y;
    let center = src[idx(x, y)];
    let left = src[idx(left_of(x), y)];
    let right = src[idx(right_of(x), y)];
    let up = src[idx(x, up_of(y))];
    let down = src[idx(x, down_of(y))];
    let average = (left + right + up + down) * 0.25;

    dst[idx(x, y)] = center + (average - center) * uniforms.diffusion_rate;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DiffusionUniforms {
    width: u32,
    height: u32,
    diffusion_rate: f32,
    pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

pub struct WgpuDiffusionBackend<'a> {
    config: DiffusionConfig,
    device: Arc<etalon::Device>,
    queue: Arc<etalon::Queue>,
    compute_pipeline: etalon::ComputePipeline,
    bind_group_layout: etalon::BindGroupLayout,
    uniform_buffer: etalon::Buffer,
    state_buffers: [etalon::Buffer; 2],
    bind_groups: [etalon::BindGroup; 2],
    staging_buffer: etalon::Buffer,
    frames: SharedFrameBuffer,
    read_index: usize,
    // Render pipeline fields (Phase 1.1) - planned for future use
    #[allow(dead_code)]
    surface: Option<etalon::Surface<'a>>,
    #[allow(dead_code)]
    render_pipeline: Option<etalon::RenderPipeline>,
    #[allow(dead_code)]
    vertex_buffer: Option<etalon::Buffer>,
    #[allow(dead_code)]
    index_buffer: Option<etalon::Buffer>,
    #[allow(dead_code)]
    depth_texture: Option<etalon::Texture>,
    #[allow(dead_code)]
    depth_texture_view: Option<etalon::TextureView>,
}

/// Build a wgpu instance with the same Windows-safe backend policy as
/// `qualia_core_db::gpu_context`: **DX12 by default on Windows**.
///
/// `Instance::default()` enables Vulkan + DX12. On many Intel UHD systems the
/// Vulkan ICD (`igvk64.dll`) access-violates during `request_device`, which
/// kills the whole desktop process mid first-run setup. Prefer DX12 unless the
/// operator pins `QUALIA_WGPU_BACKEND` / `WGPU_BACKEND`.
fn create_instance() -> etalon::Instance {
    let mut desc = etalon::InstanceDescriptor::new_without_display_handle();

    if let Some(backends) = backend_override_from_env() {
        log::info!("webizen-runtime|backend_override|{backends:?}");
        desc.backends = backends;
    } else if cfg!(target_os = "windows") {
        desc.backends = etalon::Backends::DX12;
        log::info!(
            "webizen-runtime|backend_default|windows->dx12 (override with QUALIA_WGPU_BACKEND=vulkan)"
        );
    }

    etalon::Instance::new(desc)
}

fn backend_override_from_env() -> Option<etalon::Backends> {
    for key in ["QUALIA_WGPU_BACKEND", "WGPU_BACKEND"] {
        if let Ok(raw) = std::env::var(key) {
            let v = raw.trim().to_ascii_lowercase();
            if v.is_empty() {
                continue;
            }
            return Some(match v.as_str() {
                "vulkan" | "vk" => etalon::Backends::VULKAN,
                "dx12" | "d3d12" | "directx12" | "dx" => etalon::Backends::DX12,
                "metal" | "mtl" => etalon::Backends::METAL,
                "gl" | "opengl" | "gles" => etalon::Backends::GL,
                "primary" => etalon::Backends::PRIMARY,
                "all" => etalon::Backends::all(),
                other => {
                    log::warn!("{key}='{other}' unrecognized — using platform default");
                    return None;
                }
            });
        }
    }
    None
}

impl<'a> WgpuDiffusionBackend<'a> {
    /// Creates a new instance.
    pub async fn new(config: DiffusionConfig) -> Result<Self, RuntimeError> {
        let instance = create_instance();

        // CRITICAL FIX: Use HighPerformance on native desktop to capture NVIDIA GPU
        // LowPower bypasses discrete NVIDIA GPU on Windows
        #[cfg(not(target_arch = "wasm32"))]
        let power_preference = etalon::PowerPreference::HighPerformance;
        #[cfg(target_arch = "wasm32")]
        let power_preference = etalon::PowerPreference::LowPower;

        let adapter = instance
            .request_adapter(&etalon::RequestAdapterOptions {
                power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
                // wgpu 30 added `apply_limit_buckets`; take the crate default.
                ..Default::default()
            })
            .await
            .map_err(|_| RuntimeError::AdapterUnavailable)?;

        let adapter_info = adapter.get_info();
        log::info!(
            "wGPU adapter selected: {} (Backend: {:?})",
            adapter_info.name,
            adapter_info.backend
        );

        log::info!(
            "webizen-runtime|request_device|adapter={} backend={:?}",
            adapter_info.name,
            adapter_info.backend
        );
        let (device, queue) = adapter
            .request_device(&etalon::DeviceDescriptor {
                label: Some("webizen-runtime-device"),
                required_features: etalon::Features::empty(),
                required_limits: etalon::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|err| RuntimeError::DeviceRequestFailed(err.to_string()))?;
        log::info!("webizen-runtime|request_device|ok");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
            label: Some("webizen-diffusion-shader"),
            source: etalon::ShaderSource::Wgsl(DIFFUSION_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&etalon::BindGroupLayoutDescriptor {
            label: Some("webizen-diffusion-bind-group-layout"),
            entries: &[
                etalon::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: etalon::ShaderStages::COMPUTE,
                    ty: etalon::BindingType::Buffer {
                        ty: etalon::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                etalon::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: etalon::ShaderStages::COMPUTE,
                    ty: etalon::BindingType::Buffer {
                        ty: etalon::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                etalon::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: etalon::ShaderStages::COMPUTE,
                    ty: etalon::BindingType::Buffer {
                        ty: etalon::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
            label: Some("webizen-diffusion-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&etalon::ComputePipelineDescriptor {
            label: Some("webizen-diffusion-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let (uniform_buffer, state_buffers, bind_groups, staging_buffer, frames) =
            Self::allocate_io_resources(&device, &bind_group_layout, config);

        // Phase 1.1: Initialize render pipeline (without surface for now)
        let render_pipeline = Self::create_render_pipeline(&device);
        let vertex_buffer = None;
        let index_buffer = None;
        let depth_texture = None;
        let depth_texture_view = None;
        let surface = None;

        Ok(Self {
            config,
            device,
            queue,
            compute_pipeline,
            bind_group_layout,
            uniform_buffer,
            state_buffers,
            bind_groups,
            staging_buffer,
            frames,
            read_index: 0,
            surface,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            depth_texture,
            depth_texture_view,
        })
    }

    fn create_render_pipeline(device: &etalon::Device) -> Option<etalon::RenderPipeline> {
        let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
            label: Some("webizen-render-shader"),
            source: etalon::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
            label: Some("webizen-render-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        Some(
            device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
                label: Some("webizen-render-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: etalon::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    // wgpu 30: VertexState::buffers is &[Option<VertexBufferLayout>].
                    buffers: &[Some(etalon::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as etalon::BufferAddress,
                        step_mode: etalon::VertexStepMode::Vertex,
                        attributes: &[
                            etalon::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: etalon::VertexFormat::Float32x3,
                            },
                            etalon::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 3]>() as etalon::BufferAddress,
                                shader_location: 1,
                                format: etalon::VertexFormat::Float32x4,
                            },
                        ],
                    })],
                },
                fragment: Some(etalon::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(etalon::ColorTargetState {
                        format: etalon::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(etalon::BlendState::REPLACE),
                        write_mask: etalon::ColorWrites::ALL,
                    })],
                }),
                primitive: etalon::PrimitiveState {
                    topology: etalon::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: etalon::FrontFace::Ccw,
                    cull_mode: Some(etalon::Face::Back),
                    polygon_mode: etalon::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: etalon::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            }),
        )
    }

    fn allocate_io_resources(
        device: &etalon::Device,
        bind_group_layout: &etalon::BindGroupLayout,
        config: DiffusionConfig,
    ) -> (
        etalon::Buffer,
        [etalon::Buffer; 2],
        [etalon::BindGroup; 2],
        etalon::Buffer,
        SharedFrameBuffer,
    ) {
        let uniforms = DiffusionUniforms {
            width: config.width,
            height: config.height,
            diffusion_rate: config.diffusion_rate,
            pad: 0.0,
        };

        let uniform_buffer = device.create_buffer_init(&etalon::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-uniform-buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: etalon::BufferUsages::UNIFORM | etalon::BufferUsages::COPY_DST,
        });

        let initial_state = seeded_field(&config);
        let raw_bytes = bytemuck::cast_slice(&initial_state);

        let state_buffer_a = device.create_buffer_init(&etalon::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-state-a"),
            contents: raw_bytes,
            usage: etalon::BufferUsages::STORAGE
                | etalon::BufferUsages::COPY_SRC
                | etalon::BufferUsages::COPY_DST,
        });

        let state_buffer_b = device.create_buffer_init(&etalon::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-state-b"),
            contents: raw_bytes,
            usage: etalon::BufferUsages::STORAGE
                | etalon::BufferUsages::COPY_SRC
                | etalon::BufferUsages::COPY_DST,
        });

        let staging_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("webizen-diffusion-staging"),
            size: config.raw_byte_len(),
            usage: etalon::BufferUsages::COPY_DST | etalon::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group_forward = device.create_bind_group(&etalon::BindGroupDescriptor {
            label: Some("webizen-diffusion-bind-group-a-to-b"),
            layout: bind_group_layout,
            entries: &[
                etalon::BindGroupEntry {
                    binding: 0,
                    resource: state_buffer_a.as_entire_binding(),
                },
                etalon::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer_b.as_entire_binding(),
                },
                etalon::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_reverse = device.create_bind_group(&etalon::BindGroupDescriptor {
            label: Some("webizen-diffusion-bind-group-b-to-a"),
            layout: bind_group_layout,
            entries: &[
                etalon::BindGroupEntry {
                    binding: 0,
                    resource: state_buffer_b.as_entire_binding(),
                },
                etalon::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer_a.as_entire_binding(),
                },
                etalon::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let frames = SharedFrameBuffer::new(config.cell_count() * 4);

        (
            uniform_buffer,
            [state_buffer_a, state_buffer_b],
            [bind_group_forward, bind_group_reverse],
            staging_buffer,
            frames,
        )
    }

    fn readback_snapshot_data(&self, slot: usize) -> Result<StateHash, RuntimeError> {
        let slice = self.staging_buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(etalon::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        let _ = self.device.poll(etalon::PollType::wait_indefinitely());
        let map_result = rx.recv().map_err(|_| RuntimeError::ChannelClosed)?;
        map_result.map_err(|err| RuntimeError::BufferMapFailed(err.to_string()))?;

        let mapped = slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let hash = compute_state_hash(&mapped);
        let field = bytemuck::cast_slice::<u8, f32>(&mapped);
        let _ = self.frames.overwrite_slot(slot, |rgba| {
            for (index, value) in field.iter().enumerate() {
                let base = index * 4;
                let channel = (value.clamp(0.0, 1.0) * 255.0) as u8;
                rgba[base] = channel;
                rgba[base + 1] = channel;
                rgba[base + 2] = channel;
                rgba[base + 3] = 255;
            }
        });
        drop(mapped);
        self.staging_buffer.unmap();

        Ok(hash)
    }
}

impl<'a> ComputeBackend for WgpuDiffusionBackend<'a> {
    fn step(&mut self, epoch: u64) -> Result<SimulationSnapshot, RuntimeError> {
        let write_index = (self.read_index + 1) % 2;
        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("webizen-diffusion-encoder"),
            });

        {
            let mut compute = encoder.begin_compute_pass(&etalon::ComputePassDescriptor {
                label: Some("webizen-diffusion-pass"),
                timestamp_writes: None,
            });
            compute.set_pipeline(&self.compute_pipeline);
            compute.set_bind_group(0, &self.bind_groups[self.read_index], &[]);
            compute.dispatch_workgroups(
                self.config.width.div_ceil(WORKGROUP_SIZE),
                self.config.height.div_ceil(WORKGROUP_SIZE),
                1,
            );
        }

        encoder.copy_buffer_to_buffer(
            &self.state_buffers[write_index],
            0,
            &self.staging_buffer,
            0,
            self.config.raw_byte_len(),
        );
        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(etalon::PollType::wait_indefinitely());

        let state_hash = self.readback_snapshot_data(write_index)?;
        self.read_index = write_index;

        Ok(SimulationSnapshot {
            epoch,
            dimensions: (self.config.width, self.config.height),
            state_hash,
            frame: FrameHandle::CpuRgbaSlot(write_index as u8),
        })
    }

    fn reconfigure(&mut self, config: DiffusionConfig) -> Result<(), RuntimeError> {
        let (uniform_buffer, state_buffers, bind_groups, staging_buffer, frames) =
            Self::allocate_io_resources(&self.device, &self.bind_group_layout, config);

        self.config = config;
        self.uniform_buffer = uniform_buffer;
        self.state_buffers = state_buffers;
        self.bind_groups = bind_groups;
        self.staging_buffer = staging_buffer;
        self.frames = frames;
        self.read_index = 0;
        Ok(())
    }

    fn shared_frames(&self) -> Option<SharedFrameBuffer> {
        Some(self.frames.clone())
    }
}

fn seeded_field(config: &DiffusionConfig) -> Vec<f32> {
    let mut state = vec![0.0; config.cell_count()];
    let center_x = config.width / 2;
    let center_y = config.height / 2;
    let center = (center_y * config.width + center_x) as usize;
    if let Some(cell) = state.get_mut(center) {
        *cell = 1.0;
    }
    state
}
