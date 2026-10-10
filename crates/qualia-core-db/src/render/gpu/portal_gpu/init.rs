//! Resident GPU pipeline, attachment, and uniform resource initialization.

use super::super::*;
use crate::gpu_context::global_vram_ledger;
use crate::render::atmosphere::AtmospherePreset;
use crate::render::camera::CameraState;
use crate::render::standpoint::spectator_default;
use crate::render::telemetry::ParticleInstance;
use crate::shaders::viewport::{AMBIENT_WGSL, MESH_WGSL, PROJECTOR_WGSL};
use std::sync::Arc;
use wgpu::util::DeviceExt;

impl PortalGpu {
    pub(super) async fn from_device(
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
        let temporal_producers = temporal_resolve.as_ref().and_then(|_| {
            temporal_producers_gpu::TemporalProducersGpu::try_new(
                &device,
                scene_depth.view(),
                width,
                height,
            )
        });
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
            temporal_producers,
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

}
