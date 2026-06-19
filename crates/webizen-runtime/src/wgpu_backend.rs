use crate::diffusion::DiffusionConfig;
use crate::error::RuntimeError;
use crate::kernel::ComputeBackend;
use crate::snapshot::{
    compute_state_hash, FrameHandle, SharedFrameBuffer, SimulationSnapshot, StateHash,
};
use bytemuck::{Pod, Zeroable};
use std::sync::{mpsc, Arc};
use wgpu::util::DeviceExt;

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
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    state_buffers: [wgpu::Buffer; 2],
    bind_groups: [wgpu::BindGroup; 2],
    staging_buffer: wgpu::Buffer,
    frames: SharedFrameBuffer,
    read_index: usize,
    // Render pipeline fields (Phase 1.1) - planned for future use
    #[allow(dead_code)]
    surface: Option<wgpu::Surface<'a>>,
    #[allow(dead_code)]
    render_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    vertex_buffer: Option<wgpu::Buffer>,
    #[allow(dead_code)]
    index_buffer: Option<wgpu::Buffer>,
    #[allow(dead_code)]
    depth_texture: Option<wgpu::Texture>,
    #[allow(dead_code)]
    depth_texture_view: Option<wgpu::TextureView>,
}

impl<'a> WgpuDiffusionBackend<'a> {
    pub async fn new(config: DiffusionConfig) -> Result<Self, RuntimeError> {
        let instance = wgpu::Instance::default();

        // CRITICAL FIX: Use HighPerformance on native desktop to capture NVIDIA GPU
        // LowPower bypasses discrete NVIDIA GPU on Windows
        #[cfg(not(target_arch = "wasm32"))]
        let power_preference = wgpu::PowerPreference::HighPerformance;
        #[cfg(target_arch = "wasm32")]
        let power_preference = wgpu::PowerPreference::LowPower;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RuntimeError::AdapterUnavailable)?;

        let adapter_info = adapter.get_info();
        log::info!(
            "wGPU adapter selected: {} (Backend: {:?})",
            adapter_info.name,
            adapter_info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("webizen-runtime-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .map_err(|err| RuntimeError::DeviceRequestFailed(err.to_string()))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("webizen-diffusion-shader"),
            source: wgpu::ShaderSource::Wgsl(DIFFUSION_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("webizen-diffusion-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("webizen-diffusion-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("webizen-diffusion-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
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

    fn create_render_pipeline(device: &wgpu::Device) -> Option<wgpu::RenderPipeline> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("webizen-render-shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("webizen-render-pipeline-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("webizen-render-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vertex_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fragment_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            }),
        )
    }

    fn allocate_io_resources(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        config: DiffusionConfig,
    ) -> (
        wgpu::Buffer,
        [wgpu::Buffer; 2],
        [wgpu::BindGroup; 2],
        wgpu::Buffer,
        SharedFrameBuffer,
    ) {
        let uniforms = DiffusionUniforms {
            width: config.width,
            height: config.height,
            diffusion_rate: config.diffusion_rate,
            pad: 0.0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-uniform-buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let initial_state = seeded_field(&config);
        let raw_bytes = bytemuck::cast_slice(&initial_state);

        let state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-state-a"),
            contents: raw_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let state_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("webizen-diffusion-state-b"),
            contents: raw_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-diffusion-staging"),
            size: config.raw_byte_len(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group_forward = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("webizen-diffusion-bind-group-a-to-b"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_reverse = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("webizen-diffusion-bind-group-b-to-a"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: state_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
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
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        self.device.poll(wgpu::Maintain::Wait);
        let map_result = rx.recv().map_err(|_| RuntimeError::ChannelClosed)?;
        map_result.map_err(|err| RuntimeError::BufferMapFailed(err.to_string()))?;

        let mapped = slice.get_mapped_range();
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
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("webizen-diffusion-encoder"),
            });

        {
            let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        self.device.poll(wgpu::Maintain::Wait);

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
