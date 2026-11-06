//! Vertex layouts, ambient configuration, and pipeline builders for [`WgpuRenderer`].

#[cfg(not(feature = "qualia"))]
use super::vegetation_effects;
use super::WgpuRenderer;

/// Vertex for 2D screen-space rendering.
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub(super) struct ScreenVertex {
    pub(super) position: [f32; 2],
    pub(super) color: [f32; 4],
}

/// Vertex for 3D projector pipeline.
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub(super) struct ProjectorVertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 4],
}

/// Ambient visualization configuration.
#[derive(Debug, Clone, Copy)]
pub struct AmbientConfig {
    /// Enable/disable ambient visualization.
    pub enabled: bool,
    /// Number of particles to render.
    pub particle_count: usize,
    /// Visual intensity multiplier (0.0 to 2.0).
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

#[cfg(not(feature = "qualia"))]
type ParticleInstance = vegetation_effects::GpuParticle;

/// Uniform buffer for ambient shader.
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub(super) struct AmbientUniforms {
    pub(super) time: f32,
    pub(super) view_width: f32,
    pub(super) view_height: f32,
    pub(super) world_frame: u32,
}

impl<'a> WgpuRenderer<'a> {
    #[cfg(not(feature = "qualia"))]
    pub(super) fn ambient_fields(
        device: &etalon::Device,
        width: u32,
        height: u32,
    ) -> (
        AmbientConfig,
        Option<etalon::RenderPipeline>,
        Option<etalon::Buffer>,
        Option<etalon::Buffer>,
        Option<etalon::Buffer>,
        usize,
        vegetation_effects::EffectPool,
    ) {
        let ambient_config = AmbientConfig::default();
        let (
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
            ambient_effects,
        ) = Self::init_ambient_visualization(device, &ambient_config, width, height);
        (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
            ambient_effects,
        )
    }

    pub(super) fn create_vertex_buffer(
        device: &etalon::Device,
    ) -> (etalon::Buffer, etalon::Buffer, usize) {
        let max_vertices = 10000;
        let vertex_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("webizen-vertex-buffer"),
            size: (max_vertices * std::mem::size_of::<ScreenVertex>()) as u64,
            usage: etalon::BufferUsages::VERTEX | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let projector_vertex_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("webizen-projector-vertex-buffer"),
            size: (max_vertices * std::mem::size_of::<ProjectorVertex>()) as u64,
            usage: etalon::BufferUsages::VERTEX | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        (vertex_buffer, projector_vertex_buffer, max_vertices)
    }

    pub(super) fn create_projector_pipeline(
        device: &etalon::Device,
        target_format: etalon::TextureFormat,
    ) -> etalon::RenderPipeline {
        let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
            label: Some("webizen-projector-shader"),
            source: etalon::ShaderSource::Wgsl(crate::shaders::PROJECTOR_WGSL.into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&etalon::BindGroupLayoutDescriptor {
                label: Some("webizen-projector-bgl"),
                entries: &[
                    etalon::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: etalon::ShaderStages::VERTEX,
                        ty: etalon::BindingType::Buffer {
                            ty: etalon::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    etalon::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: etalon::ShaderStages::VERTEX,
                        ty: etalon::BindingType::Buffer {
                            ty: etalon::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    etalon::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: etalon::ShaderStages::VERTEX,
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
            label: Some("webizen-projector-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
            label: Some("webizen-projector-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: etalon::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[Some(etalon::VertexBufferLayout {
                    array_stride: (std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 4]>())
                        as etalon::BufferAddress,
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
                    format: target_format,
                    blend: Some(etalon::BlendState::ALPHA_BLENDING),
                    write_mask: etalon::ColorWrites::ALL,
                })],
            }),
            primitive: etalon::PrimitiveState {
                topology: etalon::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: etalon::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: etalon::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: etalon::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub(super) fn create_render_pipeline(
        device: &etalon::Device,
        target_format: etalon::TextureFormat,
    ) -> etalon::RenderPipeline {
        let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
            label: Some("webizen-screen-shader"),
            source: etalon::ShaderSource::Wgsl(crate::shaders::SCREEN_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
            label: Some("webizen-screen-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
            label: Some("webizen-screen-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: etalon::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[Some(etalon::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ScreenVertex>() as etalon::BufferAddress,
                    step_mode: etalon::VertexStepMode::Vertex,
                    attributes: &[
                        etalon::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: etalon::VertexFormat::Float32x2,
                        },
                        etalon::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as etalon::BufferAddress,
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
                    format: target_format,
                    blend: Some(etalon::BlendState::ALPHA_BLENDING),
                    write_mask: etalon::ColorWrites::ALL,
                })],
            }),
            primitive: etalon::PrimitiveState {
                topology: etalon::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: etalon::FrontFace::Ccw,
                cull_mode: None,
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
        })
    }

    #[cfg(not(feature = "qualia"))]
    fn init_ambient_visualization(
        device: &etalon::Device,
        config: &AmbientConfig,
        width: u32,
        height: u32,
    ) -> (
        Option<etalon::RenderPipeline>,
        Option<etalon::Buffer>,
        Option<etalon::Buffer>,
        Option<etalon::Buffer>,
        usize,
        vegetation_effects::EffectPool,
    ) {
        if !config.enabled {
            return (
                None,
                None,
                None,
                None,
                0,
                vegetation_effects::EffectPool::new(0, vegetation_effects::DEFAULT_WORLD_SEED),
            );
        }

        let effects = vegetation_effects::EffectPool::new(
            config.particle_count,
            vegetation_effects::DEFAULT_WORLD_SEED,
        );
        let particle_count = effects.capacity();
        let particles = effects.particles();

        let particle_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("ambient-particle-buffer"),
            size: (particle_count * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: etalon::BufferUsages::STORAGE | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        particle_buffer
            .slice(..)
            .get_mapped_range_mut()
            .expect("ambient particle buffer was mapped at creation")
            .copy_from_slice(bytemuck::cast_slice(&particles));
        particle_buffer.unmap();

        let ambient_uniform_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("ambient-uniform-buffer"),
            size: std::mem::size_of::<AmbientUniforms>() as u64,
            usage: etalon::BufferUsages::UNIFORM | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let telemetry_buffer = device.create_buffer(&etalon::BufferDescriptor {
            label: Some("ambient-telemetry-buffer"),
            size: std::mem::size_of::<crate::scene_contract::SystemTelemetry>() as u64,
            usage: etalon::BufferUsages::UNIFORM | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ambient_pipeline = Self::create_ambient_pipeline(device, width, height);

        (
            Some(ambient_pipeline),
            Some(particle_buffer),
            Some(ambient_uniform_buffer),
            Some(telemetry_buffer),
            particle_count,
            effects,
        )
    }

    #[cfg(not(feature = "qualia"))]
    fn create_ambient_pipeline(
        device: &etalon::Device,
        _width: u32,
        _height: u32,
    ) -> etalon::RenderPipeline {
        let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
            label: Some("ambient-shader"),
            source: etalon::ShaderSource::Wgsl(crate::shaders::AMBIENT_WGSL.into()),
        });

        let bind_group_layout =
            device.create_bind_group_layout(&etalon::BindGroupLayoutDescriptor {
                label: Some("ambient-bind-group-layout"),
                entries: &[
                    etalon::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: etalon::ShaderStages::VERTEX,
                        ty: etalon::BindingType::Buffer {
                            ty: etalon::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    etalon::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: etalon::ShaderStages::VERTEX,
                        ty: etalon::BindingType::Buffer {
                            ty: etalon::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    etalon::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: etalon::ShaderStages::VERTEX,
                        ty: etalon::BindingType::Buffer {
                            ty: etalon::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
            label: Some("ambient-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
            label: Some("ambient-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: etalon::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(etalon::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(etalon::ColorTargetState {
                    format: etalon::TextureFormat::Rgba8Unorm,
                    blend: Some(etalon::BlendState {
                        color: etalon::BlendComponent {
                            src_factor: etalon::BlendFactor::SrcAlpha,
                            dst_factor: etalon::BlendFactor::One,
                            operation: etalon::BlendOperation::Add,
                        },
                        alpha: etalon::BlendComponent {
                            src_factor: etalon::BlendFactor::One,
                            dst_factor: etalon::BlendFactor::One,
                            operation: etalon::BlendOperation::Add,
                        },
                    }),
                    write_mask: etalon::ColorWrites::ALL,
                })],
            }),
            primitive: etalon::PrimitiveState {
                topology: etalon::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: etalon::FrontFace::Ccw,
                cull_mode: None,
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
        })
    }
}
