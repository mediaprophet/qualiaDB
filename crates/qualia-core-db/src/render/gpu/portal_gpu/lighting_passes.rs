//! Ambient occlusion, cascaded sun shadows, and transparent draw sorting for PortalGpu.

use super::super::{ao, create_mesh_frame_bind, motor_to_mat4_col, shadows, PortalGpu};

impl PortalGpu {
    /// `sort_transparent_draws`.
    pub(super) fn sort_transparent_draws(&mut self) {
        let Some(mesh) = self.mesh.as_mut() else {
            return;
        };
        let eye = crate::render::camera::orbit_eye_position_target(
            self.camera.yaw,
            self.camera.pitch,
            self.camera.zoom,
            self.camera.target,
        );
        let forward = crate::render::camera::orbit_forward(self.camera.yaw, self.camera.pitch);
        let model = motor_to_mat4_col(self.last_admitted);
        let draws = &mut mesh.material_gpu.draws;
        for &index in &mesh.material_gpu.transparent_draw_order {
            let draw = &mut draws[index];
            let point = draw.center;
            let world = [
                model[0][0] * point[0]
                    + model[1][0] * point[1]
                    + model[2][0] * point[2]
                    + model[3][0],
                model[0][1] * point[0]
                    + model[1][1] * point[1]
                    + model[2][1] * point[2]
                    + model[3][1],
                model[0][2] * point[0]
                    + model[1][2] * point[1]
                    + model[2][2] * point[2]
                    + model[3][2],
            ];
            let to_center = [world[0] - eye[0], world[1] - eye[1], world[2] - eye[2]];
            draw.sort_depth =
                to_center[0] * forward[0] + to_center[1] * forward[1] + to_center[2] * forward[2];
        }
        mesh.material_gpu
            .transparent_draw_order
            .sort_by(|left, right| {
                draws[*right]
                    .sort_depth
                    .total_cmp(&draws[*left].sort_depth)
                    .then_with(|| left.cmp(right))
            });
    }

    /// `record_sun_shadow_pass`.
    pub(super) fn record_sun_shadow_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(target), Some(mesh)) = (self.shadow_target.as_ref(), self.mesh.as_ref()) else {
            return;
        };
        if !self.shadow_enabled
            || !mesh.receives_shadows
            || mesh.shadow_draws.is_empty()
            || self.camera.sun_intensity <= 0.0
            || !self.shadow_map_dirty
        {
            return;
        }
        for (cascade, view) in target.views.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if cascade == 0 {
                    "portal-sun-shadow-near-depth-pass"
                } else {
                    "portal-sun-shadow-far-depth-pass"
                }),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                multiview_mask: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.shadow_pipelines[cascade]);
            pass.set_bind_group(0, &self.shadow_uniform_bind, &[]);
            pass.set_bind_group(1, &self.mesh_model_bind, &[]);
            pass.set_bind_group(3, self.mesh_instances.bind_group(), &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
            pass.set_vertex_buffer(1, mesh.color_buf.slice(..));
            pass.set_vertex_buffer(2, mesh.uv_buf.slice(..));
            pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            let cascade_matrix = self.shadow_uniform_cpu.light_view_projection[cascade];
            let model_matrix = motor_to_mat4_col(self.last_admitted);
            for draw in &mesh.shadow_draws {
                if self.mesh_instances.count() == 1
                    && shadows::aabb_outside_shadow_clip(
                        draw.bounds_min,
                        draw.bounds_max,
                        cascade_matrix,
                        model_matrix,
                    )
                {
                    continue;
                }
                pass.set_bind_group(
                    2,
                    &mesh.material_gpu.material_bind_groups[draw.material_index],
                    &[draw.material_offset],
                );
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    0,
                    0..self.mesh_instances.count(),
                );
            }
        }
        self.shadow_map_dirty = false;
    }

    /// `rebuild_ao_resources`.
    pub(in crate::render::gpu) fn rebuild_ao_resources(&mut self) {
        // Release old extents before admission so a resize can reuse their reserved bytes.
        self.ao_targets = None;
        let targets = ao::AoTargets::try_new(
            &self.device,
            self.width,
            self.height,
            &self.ao_prepass_camera_layout,
            &self.mesh_model_layout,
            &self.mesh_instance_layout,
            &self.mesh_material_layout,
            &self.mesh_texture_layout,
            &self.ao_uniform_buf,
        );
        let (ao_view, packed_surface_view) = targets
            .as_ref()
            .map(|resources| (resources.ao_view(), resources.normal_view()))
            .unwrap_or((
                self.material_texture_defaults.view(3),
                self.material_texture_defaults.view(1),
            ));
        let shadow_views = self
            .shadow_target
            .as_ref()
            .map(|target| [&target.views[0], &target.views[1]])
            .unwrap_or([self.scene_depth.view(), self.scene_depth.view()]);
        self.mesh_frame_bind = create_mesh_frame_bind(
            &self.device,
            &self.mesh_frame_layout,
            &self.camera_buf,
            &self.observer_buf,
            shadow_views,
            &self.shadow_sampler,
            &self.shadow_uniform_buf,
            ao_view,
            packed_surface_view,
            &self.ao_uniform_buf,
            &self.atmosphere_uniform_buf,
        );
        self.ao_targets = targets;
    }

    /// Writes `ao-uniform`.
    pub(super) fn write_ao_uniform(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let camera = self
            .camera
            .to_uniform(self.width as f32 / self.height.max(1) as f32, false);
        let enabled = self.ao_enabled && self.ao_targets.is_some() && self.mesh.is_some();
        let uniform = ao::make_uniform(
            self.width,
            self.height,
            [camera._padding[1], camera._padding[2], camera._padding[3]],
            crate::render::camera::orbit_forward(camera.yaw, camera.pitch),
            self.ao_radius,
            self.ao_strength,
            self.ao_bias,
            self.ao_sample_count,
            enabled,
        );
        self.uniform_belt
            .write_and_unmap(bytemuck::bytes_of(&uniform));
        self.uniform_belt
            .record_copy(encoder, &self.ao_uniform_buf, 0);
        self.uniform_belt.advance(&self.device);
    }

    /// `record_ao_pass`.
    pub(super) fn record_ao_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.ao_enabled {
            if let (Some(targets), Some(mesh)) = (self.ao_targets.as_ref(), self.mesh.as_ref()) {
                targets.record(
                    encoder,
                    mesh,
                    &self.ao_prepass_camera_bind,
                    &self.mesh_model_bind,
                    self.visible_mesh_instances.bind_group(),
                    self.visible_mesh_instances.count(),
                );
            }
        }
    }
}

/// 112-byte uniform block for `CLUSTER_LIGHT_WGSL` (`16 x 9 x 24 = 3456` frustum clusters).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ClusterGridParamsGpu {
    pub grid_dim: [u32; 4],
    pub view_size: [f32; 4],
    pub inverse_projection: [[f32; 4]; 4],
    pub light_count: u32,
    pub max_lights_per_cluster: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

const _: [(); 112] = [(); std::mem::size_of::<ClusterGridParamsGpu>()];

/// Active GPU compute pipeline owner for 3D Clustered Light Culling (`CLUSTER_LIGHT_WGSL`).
pub struct ClusterLightPipelineGpu {
    pub build_aabbs_pipeline: wgpu::ComputePipeline,
    pub cull_lights_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub params_buf: wgpu::Buffer,
    pub cluster_aabbs_buf: wgpu::Buffer,
    pub point_lights_buf: wgpu::Buffer,
    pub cluster_light_indices_buf: wgpu::Buffer,
    pub cluster_grid_buf: wgpu::Buffer,
}

impl ClusterLightPipelineGpu {
    pub const TOTAL_CLUSTERS: u32 = 16 * 9 * 24;
    pub const MAX_LIGHTS_PER_CLUSTER: u32 = 64;

    pub fn new(device: &wgpu::Device, max_lights: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia_cluster_light_wgsl"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::CLUSTER_LIGHT_WGSL.into(),
            ),
        });

        let storage_entry = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_cluster_light_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1, false),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia_cluster_light_pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let build_aabbs_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("qualia_cluster_build_aabbs"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("cluster_build_aabbs"),
                compilation_options: Default::default(),
                cache: None,
            });

        let cull_lights_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("qualia_cluster_cull_lights"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("cluster_cull_lights"),
                compilation_options: Default::default(),
                cache: None,
            });

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_cluster_params_buf"),
            size: std::mem::size_of::<ClusterGridParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cluster_aabbs_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_cluster_aabbs_buf"),
            size: u64::from(Self::TOTAL_CLUSTERS) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let point_lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_cluster_point_lights_buf"),
            size: u64::from(max_lights.max(16)) * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cluster_light_indices_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_cluster_light_indices_buf"),
            size: u64::from(Self::TOTAL_CLUSTERS * Self::MAX_LIGHTS_PER_CLUSTER) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cluster_grid_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_cluster_grid_buf"),
            size: u64::from(Self::TOTAL_CLUSTERS) * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            build_aabbs_pipeline,
            cull_lights_pipeline,
            bind_group_layout,
            params_buf,
            cluster_aabbs_buf,
            point_lights_buf,
            cluster_light_indices_buf,
            cluster_grid_buf,
        }
    }
}

/// Active GPU compute pipeline owner for Continuous EMF Spectral Deferred Resolve (`DEFERRED_RESOLVE_WGSL`).
pub struct DeferredSpectralResolvePipelineGpu {
    pub pipeline: wgpu::ComputePipeline,
    pub frame_bgl: wgpu::BindGroupLayout,
    pub gbuffer_bgl: wgpu::BindGroupLayout,
    pub output_bgl: wgpu::BindGroupLayout,
}

impl DeferredSpectralResolvePipelineGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia_deferred_resolve_wgsl"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::DEFERRED_RESOLVE_WGSL.into(),
            ),
        });

        let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_deferred_resolve_frame_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let gbuffer_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_deferred_resolve_gbuffer_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let output_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_deferred_resolve_output_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba16Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia_deferred_resolve_pl"),
            bind_group_layouts: &[Some(&frame_bgl), Some(&gbuffer_bgl), Some(&output_bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("qualia_deferred_resolve_pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("cs_deferred_resolve"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            frame_bgl,
            gbuffer_bgl,
            output_bgl,
        }
    }
}

/// Active GPU compute pipeline owner for Continuous Multi-Scale Foliage LOD (`FOLIAGE_LOD_WGSL`).
pub struct FoliageLodPipelineGpu {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl FoliageLodPipelineGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia_foliage_lod_wgsl"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::FOLIAGE_LOD_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_foliage_lod_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia_foliage_lod_pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("qualia_foliage_lod_pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("foliage_cull_and_generate"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// 64-byte Inherent Optical Properties (IOP) descriptor mirroring `DielectricVolumeIop` in `HYDROLOGIC_OPTICS_WGSL`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DielectricVolumeIopGpu {
    pub ior_spectrum: [f32; 3],
    pub _pad0: f32,
    pub absorption_coeff: [f32; 3],
    pub _pad1: f32,
    pub rayleigh_scattering: [f32; 3],
    pub _pad2: f32,
    pub mie_scattering: [f32; 4],
}

const _: [(); 64] = [(); std::mem::size_of::<DielectricVolumeIopGpu>()];

impl DielectricVolumeIopGpu {
    /// Evaluates Beer-Lambert transmittance $T(d) = \exp(-(a + b_r + b_m) d)$ with zero heap allocations.
    #[inline]
    pub fn beer_lambert_transmittance(&self, distance_m: f32) -> [f32; 3] {
        let d = distance_m.max(0.0);
        [
            (-((self.absorption_coeff[0] + self.rayleigh_scattering[0] + self.mie_scattering[0])
                * d))
                .exp(),
            (-((self.absorption_coeff[1] + self.rayleigh_scattering[1] + self.mie_scattering[1])
                * d))
                .exp(),
            (-((self.absorption_coeff[2] + self.rayleigh_scattering[2] + self.mie_scattering[2])
                * d))
                .exp(),
        ]
    }
}
