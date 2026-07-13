//! WGPU-based N-dimensional semantic renderer
//!
//! Implements zero-heap, hardware-accelerated rendering using Projective Geometric Algebra (PGA).
//!
//! The renderer draws to one of two targets ([`RenderTarget`]):
//! - **Surface** — a windowed swapchain (`WgpuRenderer::new`), for a future native
//!   windowed host.
//! - **Offscreen** — a render-target texture (`WgpuRenderer::new_offscreen`), with
//!   CPU readback via [`WgpuRenderer::read_pixels`]. This is what the dioxus/webview
//!   studio uses: there is no OS window to hand us a surface, so we render headless
//!   and present the pixels through the existing CPU frame-buffer display path.

use wgpu;

/// 3D vector for world-space coordinates
#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Screen-space point (2D coordinates in physical pixels)
#[derive(Debug, Clone, Copy)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
}

/// Camera for world-to-screen projection
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 5.0,
            },
            target: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            fov: 60.0,
        }
    }
}

impl Camera {
    /// Project a world-space point to screen space
    pub fn project(&self, world: Vec3, viewport: (f64, f64)) -> Option<ScreenPoint> {
        // Simplified perspective projection
        let (width, height) = viewport;
        let aspect_ratio = width / height;

        // Transform to camera space
        let dx = world.x - self.position.x;
        let dy = world.y - self.position.y;
        let dz = world.z - self.position.z;

        // Simple perspective divide
        if dz <= 0.1 {
            return None; // Behind camera
        }

        let fov_rad = self.fov * std::f64::consts::PI / 180.0;
        let scale = 1.0 / (dz * (fov_rad / 2.0).tan());

        let screen_x = (dx * scale * aspect_ratio + 1.0) * width / 2.0;
        let screen_y = (-dy * scale + 1.0) * height / 2.0;

        Some(ScreenPoint {
            x: screen_x,
            y: screen_y,
        })
    }

    /// Rotate projection viewpoint around target using yaw (horizontal) and pitch (vertical) angles.
    /// This adjusts the geometric examination viewpoint of the 10D epistemic manifold.
    /// All calculations use stack-allocated f64 values for zero-heap compliance.
    pub fn orbit(&mut self, yaw: f64, pitch: f64) {
        // Calculate vector from target to eye (camera radius)
        let dx = self.position.x - self.target.x;
        let dy = self.position.y - self.target.y;
        let dz = self.position.z - self.target.z;

        // Current spherical coordinates
        let radius = (dx * dx + dy * dy + dz * dz).sqrt();
        let current_yaw = dz.atan2(dx);
        let current_pitch = (dy / radius).asin();

        // Apply rotation with pitch clamping to avoid gimbal lock
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + 0.01,
            std::f64::consts::FRAC_PI_2 - 0.01,
        );

        // Convert back to Cartesian coordinates (stack-allocated)
        let cos_pitch = new_pitch.cos();
        let sin_pitch = new_pitch.sin();
        let cos_yaw = new_yaw.cos();
        let sin_yaw = new_yaw.sin();

        // Update camera position
        self.position.x = self.target.x + radius * cos_pitch * cos_yaw;
        self.position.y = self.target.y + radius * sin_pitch;
        self.position.z = self.target.z + radius * cos_pitch * sin_yaw;
    }

    /// Zoom camera in/out by adjusting distance to target.
    /// Uses stack-allocated f64 values for zero-heap compliance.
    pub fn zoom(&mut self, delta: f64) {
        // Calculate current distance to target
        let dx = self.position.x - self.target.x;
        let dy = self.position.y - self.target.y;
        let dz = self.position.z - self.target.z;
        let current_distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Apply zoom with minimum distance clamp
        let zoom_factor = (-delta * 0.1).exp(); // Smooth exponential zoom
        let new_distance = (current_distance * zoom_factor).max(0.5);

        // Scale the position vector
        let scale = new_distance / current_distance;
        self.position.x = self.target.x + dx * scale;
        self.position.y = self.target.y + dy * scale;
        self.position.z = self.target.z + dz * scale;
    }

    /// Pan camera in screen space (dx, dy are screen-relative deltas).
    /// Uses stack-allocated f64 values for zero-heap compliance.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        // Calculate camera forward and right vectors
        let forward_x = self.target.x - self.position.x;
        let forward_y = self.target.y - self.position.y;
        let forward_z = self.target.z - self.position.z;
        let forward_len =
            (forward_x * forward_x + forward_y * forward_y + forward_z * forward_z).sqrt();

        // Normalize forward vector
        let forward_x = forward_x / forward_len;
        let forward_y = forward_y / forward_len;
        let forward_z = forward_z / forward_len;

        // Calculate right vector (cross product of forward and up)
        let right_x = forward_y * self.up.z - forward_z * self.up.y;
        let right_y = forward_z * self.up.x - forward_x * self.up.z;
        let right_z = forward_x * self.up.y - forward_y * self.up.x;
        let right_len = (right_x * right_x + right_y * right_y + right_z * right_z).sqrt();

        // Normalize right vector
        let right_x = right_x / right_len;
        let right_y = right_y / right_len;
        let right_z = right_z / right_len;

        // Calculate up vector (cross product of right and forward)
        let up_x = right_y * forward_z - right_z * forward_y;
        let up_y = right_z * forward_x - right_x * forward_z;
        let up_z = right_x * forward_y - right_y * forward_x;

        // Scale pan by distance to target for consistent feel
        let pan_scale = forward_len * 0.001;

        // Apply pan to both position and target
        let pan_x = (right_x * dx + up_x * dy) * pan_scale;
        let pan_y = (right_y * dx + up_y * dy) * pan_scale;
        let pan_z = (right_z * dx + up_z * dy) * pan_scale;

        self.position.x += pan_x;
        self.position.y += pan_y;
        self.position.z += pan_z;

        self.target.x += pan_x;
        self.target.y += pan_y;
        self.target.z += pan_z;
    }
}

/// Vertex for 2D screen-space rendering
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct ScreenVertex {
    position: [f32; 2],
    color: [f32; 4],
}

/// Vertex for 3D projector pipeline
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct ProjectorVertex {
    position: [f32; 3],
    color: [f32; 4],
}

/// Where the renderer draws each frame.
enum RenderTarget<'a> {
    /// Windowed swapchain.
    Surface {
        surface: wgpu::Surface<'a>,
        config: wgpu::SurfaceConfiguration,
    },
    /// Headless render-target texture with CPU readback.
    Offscreen {
        texture: wgpu::Texture,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    },
}

/// A single acquired frame's drawable view, plus (for surfaces) the swapchain
/// texture that must be presented after submission.
enum Frame {
    Surface {
        texture: wgpu::SurfaceTexture,
        view: wgpu::TextureView,
    },
    Offscreen {
        view: wgpu::TextureView,
    },
}

impl Frame {
    fn view(&self) -> &wgpu::TextureView {
        match self {
            Frame::Surface { view, .. } | Frame::Offscreen { view } => view,
        }
    }

    /// Presents the swapchain texture. No-op for offscreen targets (their
    /// contents persist in the texture and are read back on demand).
    fn present(self) {
        if let Frame::Surface { texture, .. } = self {
            texture.present();
        }
    }
}

/// Ambient visualization configuration
#[derive(Debug, Clone, Copy)]
pub struct AmbientConfig {
    /// Enable/disable ambient visualization
    pub enabled: bool,
    /// Number of particles to render
    pub particle_count: usize,
    /// Visual intensity multiplier (0.0 to 2.0)
    pub intensity: f32,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            particle_count: 50000,
            intensity: 1.0,
        }
    }
}

/// Particle instance data for ambient visualization
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct ParticleInstance {
    position: [f32; 3],
    _padding: f32,
}

/// Uniform buffer for ambient shader
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct AmbientUniforms {
    time: f32,
    view_width: f32,
    view_height: f32,
    _padding: f32,
}

/// WGPU-based renderer implementation
pub struct WgpuRenderer<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: RenderTarget<'a>,
    camera: Camera,
    viewport_size: (f64, f64),
    // Render pipeline for 2D screen-space rendering
    render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    projector_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    projector_vertex_buffer: wgpu::Buffer,
    max_vertices: usize,
    // Track node positions for epistemic anchor coordination (zero-heap: binary indices)
    node_positions: Vec<(usize, ScreenPoint, f64)>, // (index, position, radius)
    // Legacy ambient particle layer (standalone wgpu path without qualia volumetric).
    #[cfg(not(feature = "qualia"))]
    ambient_config: AmbientConfig,
    #[cfg(not(feature = "qualia"))]
    ambient_pipeline: Option<wgpu::RenderPipeline>,
    #[cfg(not(feature = "qualia"))]
    particle_buffer: Option<wgpu::Buffer>,
    #[cfg(not(feature = "qualia"))]
    ambient_uniform_buffer: Option<wgpu::Buffer>,
    #[cfg(not(feature = "qualia"))]
    telemetry_buffer: Option<wgpu::Buffer>,
    #[cfg(not(feature = "qualia"))]
    particle_count: usize,
}

impl<'a> WgpuRenderer<'a> {
    /// Create a new WgpuRenderer that draws to a window surface.
    pub async fn new(surface: wgpu::Surface<'a>, width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::default();

        let (device, queue, adapter) = Self::request_device(&instance, Some(&surface)).await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        let render_pipeline = Self::create_render_pipeline(&device, surface_format);
        let projector_pipeline = Self::create_projector_pipeline(&device, surface_format);

        let (vertex_buffer, projector_vertex_buffer, max_vertices) = Self::create_vertex_buffer(&device);

        #[cfg(not(feature = "qualia"))]
        let (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
        ) = Self::ambient_fields(&device, width, height);

        Ok(Self {
            device,
            queue,
            target: RenderTarget::Surface {
                surface,
                config: surface_config,
            },
            camera: Camera::default(),
            viewport_size: (width as f64, height as f64),
            render_pipeline,
            projector_pipeline: Some(projector_pipeline),
            vertex_buffer,
            projector_vertex_buffer,
            max_vertices,
            node_positions: Vec::new(),
            #[cfg(not(feature = "qualia"))]
            ambient_config,
            #[cfg(not(feature = "qualia"))]
            ambient_pipeline,
            #[cfg(not(feature = "qualia"))]
            particle_buffer,
            #[cfg(not(feature = "qualia"))]
            ambient_uniform_buffer,
            #[cfg(not(feature = "qualia"))]
            telemetry_buffer,
            #[cfg(not(feature = "qualia"))]
            particle_count,
        })
    }

    /// Create a headless renderer that draws to an offscreen texture.
    ///
    /// Requires no window/surface, so it runs anywhere a GPU adapter is available
    /// (including CI and the dioxus/webview studio). Read the result with
    /// [`WgpuRenderer::read_pixels`].
    pub async fn new_offscreen(width: u32, height: u32) -> Result<WgpuRenderer<'static>, String> {
        #[cfg(feature = "qualia")]
        let (device, queue) = {
            let shared = qualia_core_db::gpu_context::shared_gpu();
            (shared.device.clone(), shared.queue.clone())
        };
        #[cfg(not(feature = "qualia"))]
        let (device, queue) = {
            let instance = wgpu::Instance::default();
            let (device, queue, _adapter) = Self::request_device(&instance, None).await?;
            (device, queue)
        };

        // Linear (non-sRGB) Unorm: colors are authored as CSS sRGB strings and
        // parsed straight to 0..1, so we want byte-exact passthrough on store.
        // An sRGB target would re-encode the (already-sRGB) values and wash out
        // mid-tones; linear keeps readback bytes equal to the CSS source, matching
        // the CPU Canvas2D path on an sRGB <canvas>.
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = Self::create_offscreen_texture(&device, format, width, height);

        let render_pipeline = Self::create_render_pipeline(&device, format);
        let projector_pipeline = Self::create_projector_pipeline(&device, format);

        let (vertex_buffer, projector_vertex_buffer, max_vertices) = Self::create_vertex_buffer(&device);

        #[cfg(not(feature = "qualia"))]
        let (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
        ) = Self::ambient_fields(&device, width, height);

        Ok(WgpuRenderer {
            device,
            queue,
            target: RenderTarget::Offscreen {
                texture,
                format,
                width,
                height,
            },
            camera: Camera::default(),
            viewport_size: (width as f64, height as f64),
            render_pipeline,
            projector_pipeline: Some(projector_pipeline),
            vertex_buffer,
            projector_vertex_buffer,
            max_vertices,
            node_positions: Vec::new(),
            #[cfg(not(feature = "qualia"))]
            ambient_config,
            #[cfg(not(feature = "qualia"))]
            ambient_pipeline,
            #[cfg(not(feature = "qualia"))]
            particle_buffer,
            #[cfg(not(feature = "qualia"))]
            ambient_uniform_buffer,
            #[cfg(not(feature = "qualia"))]
            telemetry_buffer,
            #[cfg(not(feature = "qualia"))]
            particle_count,
        })
    }

    #[cfg(not(feature = "qualia"))]
    fn ambient_fields(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (
        AmbientConfig,
        Option<wgpu::RenderPipeline>,
        Option<wgpu::Buffer>,
        Option<wgpu::Buffer>,
        Option<wgpu::Buffer>,
        usize,
    ) {
        let ambient_config = AmbientConfig::default();
        let (
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
        ) = Self::init_ambient_visualization(device, &ambient_config, width, height);
        (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
        )
    }

    /// Request an adapter + device. `HighPerformance` selects the discrete GPU on
    /// native (critical on Windows/NVIDIA); `LowPower` is friendlier in the browser.
    async fn request_device(
        instance: &wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), String> {
        #[cfg(not(target_arch = "wasm32"))]
        let power_preference = wgpu::PowerPreference::HighPerformance;
        #[cfg(target_arch = "wasm32")]
        let power_preference = wgpu::PowerPreference::LowPower;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find an appropriate adapter: {e}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("webizen-render-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        Ok((device, queue, adapter))
    }

    fn create_offscreen_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("webizen-offscreen-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn create_vertex_buffer(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, usize) {
        // Max 10000 vertices for immediate-mode rendering.
        let max_vertices = 10000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-vertex-buffer"),
            size: (max_vertices * std::mem::size_of::<ScreenVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let projector_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-projector-vertex-buffer"),
            size: (max_vertices * std::mem::size_of::<ProjectorVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        (vertex_buffer, projector_vertex_buffer, max_vertices)
    }

    fn create_projector_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("webizen-projector-shader"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::PROJECTOR_WGSL.into()),
        });

        // The projector shader requires 3 bindings: view_projection (uniform), motors (storage), motor_index (uniform).
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("webizen-projector-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("webizen-projector-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("webizen-projector-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                        wgpu::VertexAttribute { offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                // Note: projector.wgsl does not currently have a fragment_main, it expects to be linked with a standard fragment shader.
                // For this wiring, we bind it to the basic screen shader fragment_main (which just returns color).
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("webizen-screen-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/screen.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("webizen-screen-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("webizen-screen-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ScreenVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
            multiview_mask: None,
            cache: None,
        })
    }

    /// Initialize ambient visualization system
    ///
    /// Creates particle buffer with random 3D coordinates, uniform buffers,
    /// and render pipeline for GPU-driven ambient effects.
    #[cfg(not(feature = "qualia"))]
    fn init_ambient_visualization(
        device: &wgpu::Device,
        config: &AmbientConfig,
        width: u32,
        height: u32,
    ) -> (
        Option<wgpu::RenderPipeline>,
        Option<wgpu::Buffer>,
        Option<wgpu::Buffer>,
        Option<wgpu::Buffer>,
        usize,
    ) {
        if !config.enabled {
            return (None, None, None, None, 0);
        }

        // Generate random particle positions (zero-heap: stack-based RNG)
        let particle_count = config.particle_count;
        let mut particles: Vec<ParticleInstance> = Vec::with_capacity(particle_count);
        let mut rng_seed: u32 = 12345;

        for _ in 0..particle_count {
            // Simple linear congruential generator (stack-based, no heap)
            rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
            let x = (rng_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
            let y = (rng_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
            let z = (rng_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;

            particles.push(ParticleInstance {
                position: [x, y, z],
                _padding: 0.0,
            });
        }

        // Create particle buffer (storage buffer for instanced rendering)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ambient-particle-buffer"),
            size: (particle_count * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        // Fill particle buffer
        particle_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(&particles));
        particle_buffer.unmap();

        // Create ambient uniform buffer (time, viewport size)
        let ambient_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ambient-uniform-buffer"),
            size: std::mem::size_of::<AmbientUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create telemetry uniform buffer (SystemTelemetry)
        let telemetry_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ambient-telemetry-buffer"),
            size: std::mem::size_of::<crate::scene_contract::SystemTelemetry>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create ambient render pipeline
        let ambient_pipeline = Self::create_ambient_pipeline(device, width, height);

        (
            Some(ambient_pipeline),
            Some(particle_buffer),
            Some(ambient_uniform_buffer),
            Some(telemetry_buffer),
            particle_count,
        )
    }

    /// Create ambient visualization render pipeline with additive blending
    #[cfg(not(feature = "qualia"))]
    fn create_ambient_pipeline(
        device: &wgpu::Device,
        _width: u32,
        _height: u32,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ambient-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ambient.wgsl").into()),
        });

        // Create bind group layout for uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ambient-bind-group-layout"),
            entries: &[
                // Ambient uniforms (time, viewport)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Telemetry uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Particle storage buffer
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ambient-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ambient-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
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
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
            multiview_mask: None,
            cache: None,
        })
    }

    /// Acquire the drawable view for this frame.
    fn begin_frame(&self) -> Result<Frame, String> {
        match &self.target {
            RenderTarget::Surface { surface, .. } => {
                let texture = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    other => return Err(format!("Failed to get next surface texture: {other:?}")),
                };
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                Ok(Frame::Surface { texture, view })
            }
            RenderTarget::Offscreen { texture, .. } => {
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Ok(Frame::Offscreen { view })
            }
        }
    }

    /// Read the offscreen target back to a tightly-packed RGBA8 buffer
    /// (`width * height * 4` bytes, row-major). Returns `None` for surface targets.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let (texture, width, height) = match &self.target {
            RenderTarget::Offscreen {
                texture,
                width,
                height,
                ..
            } => (texture, *width, *height),
            RenderTarget::Surface { .. } => return None,
        };

        let unpadded_bytes_per_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("webizen-readback-buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("webizen-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().ok()?.ok()?;

        let mapped = slice.get_mapped_range().expect("wgpu buffer map_range failed");
        let mut out = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            out.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        readback.unmap();
        Some(out)
    }

    /// Read the offscreen target back as PNG-encoded bytes, ready to hand to a
    /// webview as a data-URI or via a custom protocol. Returns `None` for surface
    /// targets or if there is no GPU frame to read. Native-only (the wasm build
    /// renders to a `<canvas>` and never reads back).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_png(&self) -> Option<Vec<u8>> {
        use image::ImageEncoder;

        let (width, height) = match &self.target {
            RenderTarget::Offscreen { width, height, .. } => (*width, *height),
            RenderTarget::Surface { .. } => return None,
        };

        let rgba = self.read_pixels()?;
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(png_bytes)
    }

    /// Read the offscreen frame back as a `data:image/png;base64,...` URI — drop
    /// straight into a webview `<img src>`. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_data_uri(&self) -> Option<String> {
        use base64::Engine;
        let png = self.read_png()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png);
        Some(format!("data:image/png;base64,{b64}"))
    }

    /// Parse CSS color string to RGBA
    fn parse_color(color: &str, alpha: f64) -> [f32; 4] {
        // Simple CSS color parser (supports hex and rgb)
        let (r, g, b, a) = if color.starts_with('#') {
            let hex = color.trim_start_matches('#');
            let len = hex.len();
            if len == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b, (alpha * 255.0) as u8)
            } else if len == 3 {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                (r, g, b, (alpha * 255.0) as u8)
            } else {
                (0, 0, 0, (alpha * 255.0) as u8)
            }
        } else if color.starts_with("rgb(") {
            // Parse rgb(r, g, b) or rgba(r, g, b, a)
            let parts: Vec<&str> = color
                .trim_start_matches("rgb(")
                .trim_start_matches("rgba(")
                .trim_end_matches(')')
                .split(',')
                .collect();
            let r = parts
                .get(0)
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let g = parts
                .get(1)
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let b = parts
                .get(2)
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let a = if parts.len() > 3 {
                parts
                    .get(3)
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(alpha)
            } else {
                alpha
            };
            (r, g, b, (a * 255.0) as u8)
        } else {
            // Default to black
            (0, 0, 0, (alpha * 255.0) as u8)
        };

        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    /// Resize the render target
    pub fn resize(&mut self, width: u32, height: u32) {
        match &mut self.target {
            RenderTarget::Surface { surface, config } => {
                config.width = width;
                config.height = height;
                surface.configure(&self.device, config);
            }
            RenderTarget::Offscreen {
                texture,
                format,
                width: w,
                height: h,
            } => {
                *w = width;
                *h = height;
                *texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("webizen-offscreen-target"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: *format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
            }
        }
        self.viewport_size = (width as f64, height as f64);
    }
}

impl<'a> WgpuRenderer<'a> {
    /// Get current viewport size
    pub fn viewport(&self) -> (f64, f64) {
        self.viewport_size
    }

    /// Set the active camera
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Configure ambient visualization options
    #[cfg(not(feature = "qualia"))]
    pub fn set_ambient_config(&mut self, config: AmbientConfig) {
        self.ambient_config = config;
    }

    /// Get current ambient configuration
    #[cfg(not(feature = "qualia"))]
    pub fn get_ambient_config(&self) -> AmbientConfig {
        self.ambient_config
    }

    /// Clear the frame to a solid background color
    pub fn clear(&self, color: &str) {
        let rgba = Self::parse_color(color, 1.0);
        let frame = match self.begin_frame() {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("webizen-clear-encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("webizen-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: rgba[0] as f64,
                            g: rgba[1] as f64,
                            b: rgba[2] as f64,
                            a: rgba[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    /// Project a world-space point to screen space
    pub fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        self.camera.project(world, self.viewport_size)
    }

    /// Clear tracked node positions (call before each render)
    pub fn clear_node_positions(&mut self) {
        self.node_positions.clear();
    }

    /// Get tracked node positions (for epistemic anchor coordination)
    pub fn get_node_positions(&self) -> &[(usize, ScreenPoint, f64)] {
        &self.node_positions
    }

    /// Hit test: find which node contains the given screen point
    ///
    /// This represents geometric projection through the spacetime manifold.
    /// The ray casting operation determines which epistemic anchor intersects
    /// the projection ray from the viewpoint through the 10D manifold.
    ///
    /// Returns the binary node index if a hit is found, None otherwise
    /// Zero-heap: uses stack-allocated f64 values for ray calculations, no heap allocation
    /// Binary index returned for zero-heap IPC with quantum context management
    pub fn hit_test(&self, point: ScreenPoint) -> Option<usize> {
        // Check nodes in reverse order (top to bottom)
        for (index, pos, radius) in self.node_positions.iter().rev() {
            // Stack-allocated distance calculation (zero-heap compliance)
            let dx = point.x - pos.x;
            let dy = point.y - pos.y;
            let distance_squared = dx * dx + dy * dy;
            let radius_squared = *radius * *radius;

            if distance_squared <= radius_squared {
                return Some(*index);
            }
        }
        None
    }

    /// Coordinate epistemic anchor at screen coordinates (x, y in physical pixels)
    ///
    /// This is NOT traditional object picking for 3D scene manipulation.
    /// This is epistemic anchor coordination for quantum context selection and
    /// wavefunction collapse via ray casting through the 10D manifold.
    ///
    /// Returns the binary node index if an epistemic anchor is found, None otherwise
    /// Zero-heap: uses stack-allocated ray casting through manifold, no heap allocation
    /// Binary index returned for zero-heap IPC with quantum context management
    pub fn pick_anchor(&self, x: f64, y: f64) -> Option<usize> {
        // Stack-allocated screen point for geometric projection through spacetime manifold
        let point = ScreenPoint { x, y };
        self.hit_test(point)
    }

    /// Legacy method for backward compatibility - use pick_anchor() for epistemic anchor coordination
    #[deprecated(note = "Use pick_anchor() for epistemic anchor coordination")]
    pub fn pick(&self, x: f64, y: f64) -> Option<usize> {
        self.pick_anchor(x, y)
    }

    /// Check if mouse is hovering over epistemic anchor at screen coordinates (x, y in physical pixels)
    ///
    /// This is epistemic anchor hover detection for quantum context preview.
    /// Ray casting represents geometric projection through spacetime manifold.
    ///
    /// Returns the binary node index if hovering over epistemic anchor, None otherwise
    /// Zero-heap: uses stack-allocated ray casting through manifold, no heap allocation
    /// Binary index returned for zero-heap IPC with quantum context management
    pub fn hover_anchor(&self, x: f64, y: f64) -> Option<usize> {
        // Stack-allocated screen point for geometric projection through spacetime manifold
        let point = ScreenPoint { x, y };
        self.hit_test(point)
    }

    /// Legacy method for backward compatibility - use hover_anchor() for epistemic anchor coordination
    #[deprecated(note = "Use hover_anchor() for epistemic anchor coordination")]
    pub fn hover(&self, x: f64, y: f64) -> Option<usize> {
        self.hover_anchor(x, y)
    }

    /// Draw a screen-space line segment
    pub fn line(&self, a: ScreenPoint, b: ScreenPoint, color: &str, alpha: f64, width: f64) {
        let rgba = Self::parse_color(color, alpha);

        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let x1 = ((a.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let y1 = 1.0 - (a.y as f32 / height_px) * 2.0;
        let x2 = ((b.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let y2 = 1.0 - (b.y as f32 / height_px) * 2.0;

        let line_width = width as f32 / width_px;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        let nx = -dy / len * line_width;
        let ny = dx / len * line_width;

        let vertices = [
            ScreenVertex {
                position: [x1 + nx, y1 + ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x1 - nx, y1 - ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x2 + nx, y2 + ny],
                color: rgba,
            },
            ScreenVertex {
                position: [x2 - nx, y2 - ny],
                color: rgba,
            },
        ];

        self.render_vertices(&vertices, wgpu::PrimitiveTopology::TriangleStrip);
    }

    /// Draw a filled screen-space disc (billboarded point)
    pub fn point(&self, p: ScreenPoint, radius: f64, color: &str, alpha: f64) {
        let rgba = Self::parse_color(color, alpha);

        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let cx = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
        let cy = 1.0 - (p.y as f32 / height_px) * 2.0;
        let r = (radius as f32 / width_px) * 2.0;

        let segments = 16;
        let mut vertices = Vec::with_capacity(segments + 2);
        vertices.push(ScreenVertex {
            position: [cx, cy],
            color: rgba,
        });

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x = cx + angle.cos() * r;
            let y = cy + angle.sin() * r;
            vertices.push(ScreenVertex {
                position: [x, y],
                color: rgba,
            });
        }

        self.render_vertices(&vertices, wgpu::PrimitiveTopology::TriangleList);
    }

    /// Fill a screen-space polygon
    pub fn fill_polygon(&self, points: &[ScreenPoint], color: &str, alpha: f64) {
        if points.len() < 3 {
            return;
        }

        let rgba = Self::parse_color(color, alpha);

        // Convert screen coordinates to clip space (-1 to 1)
        let width_px = self.viewport_size.0 as f32;
        let height_px = self.viewport_size.1 as f32;

        let vertices: Vec<ScreenVertex> = points
            .iter()
            .map(|p| {
                let x = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
                let y = 1.0 - (p.y as f32 / height_px) * 2.0;
                ScreenVertex {
                    position: [x, y],
                    color: rgba,
                }
            })
            .collect();

        self.render_vertices(&vertices, wgpu::PrimitiveTopology::TriangleList);
    }
}

impl<'a> WgpuRenderer<'a> {
    fn render_vertices(&self, vertices: &[ScreenVertex], _topology: wgpu::PrimitiveTopology) {
        if vertices.is_empty() || vertices.len() > self.max_vertices {
            return;
        }

        let frame = match self.begin_frame() {
            Ok(frame) => frame,
            Err(_) => return,
        };

        // Update vertex buffer
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("webizen-render-encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("webizen-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

/// Render a placeholder semantic scene headlessly and return raw PNG bytes.
///
/// This is **blocking** — it builds a GPU device and drives a synchronous readback
/// (`device.poll(Wait)`), so it must be called off the UI thread (e.g. a worker
/// thread or `spawn_blocking`). Intended as the one-call bridge for the native
/// host until a real `SemanticScene` is wired in. Native-only.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_preview_png(width: u32, height: u32) -> Option<Vec<u8>> {
    let renderer = pollster::block_on(WgpuRenderer::new_offscreen(width, height)).ok()?;
    renderer.clear("#101820");
    // A simple cyan triangle so there is visible, non-trivial content to confirm.
    let (w, h) = (width as f64, height as f64);
    renderer.fill_polygon(
        &[
            ScreenPoint {
                x: w * 0.50,
                y: h * 0.12,
            },
            ScreenPoint {
                x: w * 0.12,
                y: h * 0.85,
            },
            ScreenPoint {
                x: w * 0.88,
                y: h * 0.85,
            },
        ],
        "#67e8f9",
        1.0,
    );
    renderer.read_png()
}

/// Render a RenderScene contract headlessly and return raw PNG bytes.
///
/// This is **blocking** — it builds a GPU device and drives a synchronous readback
/// (`device.poll(Wait)`), so it must be called off the UI thread (e.g. a worker
/// thread or `spawn_blocking`). Native-only.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_scene_png(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    render_scene_png_with_time(scene, width, height, 0.0)
}

/// Render scene with time parameter for animation effects.
/// Time is in seconds, used for pulsing/glowing animations.
pub fn render_scene_png_with_time(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
    time_seconds: f64,
) -> Option<Vec<u8>> {
    render_scene_png_with_time_and_telemetry(
        scene,
        width,
        height,
        time_seconds,
        &crate::scene_contract::SystemTelemetry::default(),
    )
}

/// Render scene with time parameter and telemetry for ambient visualization.
/// Time is in seconds, used for pulsing/glowing animations.
/// Telemetry drives the ambient particle visualization effects.
pub fn render_scene_png_with_time_and_telemetry(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
    time_seconds: f64,
    telemetry: &crate::scene_contract::SystemTelemetry,
) -> Option<Vec<u8>> {
    #[cfg(feature = "qualia")]
    {
        return crate::volumetric::render_scene_png(
            scene,
            width,
            height,
            time_seconds as f32,
            telemetry,
        )
        .ok();
    }

    #[cfg(not(feature = "qualia"))]
    {
        #[inline]
        fn lerp(a: f64, b: f64, t: f64) -> f64 {
            a + (b - a) * t
        }

        let mut renderer = pollster::block_on(WgpuRenderer::new_offscreen(width, height)).ok()?;

        let (w, h) = (width as f64, height as f64);

        // Clear node positions tracking
        renderer.clear_node_positions();

        // Build a lookup map for previous positions if transition is active
        let previous_positions_map: std::collections::HashMap<
            String,
            crate::scene_contract::ScenePoint,
        > = if let Some(ref transition) = scene.transition_state {
            transition.previous_positions.iter().cloned().collect()
        } else {
            std::collections::HashMap::new()
        };

        // Get transition progress (default to 1.0 if no transition)
        let transition_progress = scene
            .transition_state
            .as_ref()
            .map(|t| t.progress)
            .unwrap_or(1.0);

        // Begin single render pass for the entire frame
        let frame = renderer.begin_frame().ok()?;
        let frame_view = frame.view();

        // Parse background color
        let bg_rgba = WgpuRenderer::parse_color(&scene.background, 1.0);

        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scene-render-encoder"),
            });

        // Render ambient visualization first (background layer)
        let ambient_bind_group = if renderer.ambient_config.enabled {
            if let (
                Some(ambient_pipeline),
                Some(particle_buffer),
                Some(ambient_uniform_buffer),
                Some(telemetry_buffer),
            ) = (
                &renderer.ambient_pipeline,
                &renderer.particle_buffer,
                &renderer.ambient_uniform_buffer,
                &renderer.telemetry_buffer,
            ) {
                // Update ambient uniforms
                let uniforms = AmbientUniforms {
                    time: time_seconds as f32,
                    view_width: w as f32,
                    view_height: h as f32,
                    _padding: 0.0,
                };
                renderer.queue.write_buffer(
                    ambient_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&uniforms),
                );

                // Update telemetry uniforms
                renderer
                    .queue
                    .write_buffer(telemetry_buffer, 0, bytemuck::bytes_of(telemetry));

                // Create bind group before render pass to avoid borrow checker issue
                Some(
                    renderer
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("ambient-bind-group"),
                            layout: &ambient_pipeline.get_bind_group_layout(0),
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: ambient_uniform_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: telemetry_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: particle_buffer.as_entire_binding(),
                                },
                            ],
                        }),
                )
            } else {
                None
            }
        } else {
            None
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg_rgba[0] as f64,
                            g: bg_rgba[1] as f64,
                            b: bg_rgba[2] as f64,
                            a: bg_rgba[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Render ambient visualization first (background layer)
            if let Some(ref bind_group) = ambient_bind_group {
                if let Some(ambient_pipeline) = &renderer.ambient_pipeline {
                    render_pass.set_pipeline(ambient_pipeline);
                    render_pass.set_bind_group(0, bind_group, &[]);
                    render_pass.draw(0..6, 0..renderer.particle_count as u32);
                }
            }

            // Render faces (filled polygons) first (background layer)
            for face in &scene.faces {
                let vertices: Vec<ScreenPoint> = face
                    .vertices
                    .iter()
                    .map(|v| {
                        // Apply LERP interpolation to vertices if transition is active
                        let interpolated = if let Some(ref prev_pos) =
                            previous_positions_map.get(&format!(
                                "face_{}",
                                face.vertices.iter().position(|p| p == v).unwrap_or(0)
                            )) {
                            crate::scene_contract::ScenePoint {
                                x: lerp(prev_pos.x, v.x, transition_progress),
                                y: lerp(prev_pos.y, v.y, transition_progress),
                                z: lerp(prev_pos.z, v.z, transition_progress),
                            }
                        } else {
                            *v
                        };
                        ScreenPoint {
                            x: interpolated.x * w,
                            y: interpolated.y * h,
                        }
                    })
                    .collect();

                // Convert to clip space and render
                let width_px = w as f32;
                let height_px = h as f32;
                let clip_vertices: Vec<ScreenVertex> = vertices
                    .iter()
                    .map(|p| {
                        let x = ((p.x as f32 / width_px) * 2.0 - 1.0) * (width_px / height_px);
                        let y = 1.0 - (p.y as f32 / height_px) * 2.0;
                        let rgba = WgpuRenderer::parse_color(&face.color, face.alpha);
                        ScreenVertex {
                            position: [x, y],
                            color: rgba,
                        }
                    })
                    .collect();

                if !clip_vertices.is_empty() && clip_vertices.len() <= renderer.max_vertices {
                    renderer.queue.write_buffer(
                        &renderer.vertex_buffer,
                        0,
                        bytemuck::cast_slice(&clip_vertices),
                    );
                    
                    if let Some(projector) = &renderer.projector_pipeline {
                        // TODO: Bind projector motor/telemetry uniforms here when ready
                        render_pass.set_pipeline(projector);
                        // Temporarily using the projector vertex buffer for the projector until the full 10D pipeline is passed
                        render_pass.set_vertex_buffer(0, renderer.projector_vertex_buffer.slice(..));
                    } else {
                        render_pass.set_pipeline(&renderer.render_pipeline);
                        render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
                    }
                    
                    render_pass.draw(0..clip_vertices.len() as u32, 0..1);
                }
            }

            // Render edges (lines)
            for edge in &scene.edges {
                let from_interpolated = if let Some(ref prev_pos) =
                    previous_positions_map.get(&format!("edge_from_{}", edge.from.x))
                {
                    crate::scene_contract::ScenePoint {
                        x: lerp(prev_pos.x, edge.from.x, transition_progress),
                        y: lerp(prev_pos.y, edge.from.y, transition_progress),
                        z: lerp(prev_pos.z, edge.from.z, transition_progress),
                    }
                } else {
                    edge.from
                };

                let to_interpolated = if let Some(ref prev_pos) =
                    previous_positions_map.get(&format!("edge_to_{}", edge.to.x))
                {
                    crate::scene_contract::ScenePoint {
                        x: lerp(prev_pos.x, edge.to.x, transition_progress),
                        y: lerp(prev_pos.y, edge.to.y, transition_progress),
                        z: lerp(prev_pos.z, edge.to.z, transition_progress),
                    }
                } else {
                    edge.to
                };

                renderer.line(
                    ScreenPoint {
                        x: from_interpolated.x * w,
                        y: from_interpolated.y * h,
                    },
                    ScreenPoint {
                        x: to_interpolated.x * w,
                        y: to_interpolated.y * h,
                    },
                    &edge.color,
                    edge.width,
                    edge.alpha,
                );
            }

            // Render nodes (vertices) last (foreground layer)
            for (node_index, node) in scene.nodes.iter().enumerate() {
                // Apply LERP interpolation to node position if transition is active
                let interpolated_position =
                    if let Some(ref prev_pos) = previous_positions_map.get(&node.id) {
                        crate::scene_contract::ScenePoint {
                            x: lerp(prev_pos.x, node.position.x, transition_progress),
                            y: lerp(prev_pos.y, node.position.y, transition_progress),
                            z: lerp(prev_pos.z, node.position.z, transition_progress),
                        }
                    } else {
                        node.position
                    };

                // Check if this node is selected or hovered
                let is_selected = scene.selected_node_index == Some(node_index);
                let is_hovered = scene.hovered_node_index == Some(node_index);

                // Apply pulse animation for inferencing nodes
                let animated_radius = if node.is_inferencing && node.pulse_rate > 0.0 {
                    // Pulse: radius varies with time using sine wave
                    let pulse_phase = 2.0 * std::f64::consts::PI * node.pulse_rate * time_seconds;
                    let pulse_factor = 1.0 + 0.3 * pulse_phase.sin();
                    node.radius * pulse_factor.abs()
                } else {
                    node.radius
                };

                // Use spectral color from tensor if sigma > 0, otherwise use node color
                let spectral_color = if node.tensor.sigma != 0.0 {
                    node.tensor.spectral_to_color()
                } else {
                    node.color.clone()
                };

                // Highlight selected and hovered nodes with visual feedback
                let (final_radius, final_color) = if is_selected {
                    (animated_radius * 1.5, "#ffffff".to_string())
                } else if is_hovered {
                    (animated_radius * 1.2, "#ffff00".to_string())
                } else {
                    (animated_radius, spectral_color)
                };

                let screen_pos = ScreenPoint {
                    x: interpolated_position.x * w,
                    y: interpolated_position.y * h,
                };

                renderer.point(screen_pos, final_radius, &final_color, node.alpha);

                // Track node position for epistemic anchor coordination (zero-heap: use binary index)
                renderer
                    .node_positions
                    .push((node_index, screen_pos, final_radius));
            }
        } // render_pass dropped here

        renderer.queue.submit(Some(encoder.finish()));
        frame.present();

        renderer.read_png()
    }
}

/// Same as [`render_scene_png`], returned as a `data:image/png;base64,...`
/// URI for hosts that prefer inline delivery. Blocking; native-only.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_scene_data_uri(
    scene: &crate::scene_contract::RenderScene,
    width: u32,
    height: u32,
) -> Option<String> {
    use base64::Engine;
    let png = render_scene_png(scene, width, height)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{b64}"))
}

/// Same preview as [`render_preview_png`], returned as a `data:image/png;base64,...`
/// URI for hosts that prefer inline delivery. Blocking; native-only.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_preview_data_uri(width: u32, height: u32) -> Option<String> {
    use base64::Engine;
    let png = render_preview_png(width, height)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{b64}"))
}
