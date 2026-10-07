//! Half-resolution, depth/normal screen-space ambient visibility.
//!
//! This portable baseline borrows CACAO's adaptive-resolution strategy but keeps a bounded
//! single-ring kernel suitable for WebGPU/WASM. It is an indirect-light approximation, never a
//! replacement for sun shadows. All targets are explicitly budgeted and optional.

use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};

#[path = "ao_math.rs"]
mod math;
pub(super) use math::make_uniform;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AoUniform {
    /// Half-resolution width, height, and their reciprocals.
    pub viewport: [f32; 4],
    pub eye: [f32; 4],
    pub camera_forward: [f32; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    /// World-space radius, strength, world-space bias, enabled.
    pub params: [f32; 4],
    /// Far distance, reciprocal depth-code range, tan(vertical FOV / 2), aspect.
    pub depth: [f32; 4],
}

pub(super) struct AoTargets {
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    _normal_texture: wgpu::Texture,
    normal_view: wgpu::TextureView,
    _ao_texture: wgpu::Texture,
    ao_view: wgpu::TextureView,
    _reservation: VramReservation<'static>,
    depth_prepass: wgpu::RenderPipeline,
    depth_prepass_uniform_bind: wgpu::BindGroup,
    ao_pipeline: wgpu::RenderPipeline,
    ao_generate_bind: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl AoTargets {
    pub fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        camera_layout: &wgpu::BindGroupLayout,
        model_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
        material_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        uniform_buf: &wgpu::Buffer,
    ) -> Option<Self> {
        let bytes = math::target_bytes(width, height)?; // Depth32Float + packed RGBA8 surface + R8 AO.
        let (width, height) = math::half_extent(width, height);
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
            .ok()?;

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-ao-depth-half"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        #[cfg(all(test, not(target_arch = "wasm32")))]
        let normal_usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        #[cfg(not(all(test, not(target_arch = "wasm32"))))]
        let normal_usage =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-ao-normal-half"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: normal_usage,
            view_formats: &[],
        });
        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let ao_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-ao-visibility-half"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ao_view = ao_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_prepass_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("qualia-ao-prepass-uniform-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<AoUniform>() as u64,
                        ),
                    },
                    count: None,
                }],
            });
        let depth_prepass_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-ao-depth-normal-pipeline-layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(model_layout),
                Some(&depth_prepass_uniform_layout),
                Some(material_layout),
                Some(texture_layout),
                Some(instance_layout),
            ],
            immediate_size: 0,
        });
        let depth_prepass_uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-ao-prepass-uniform"),
            layout: &depth_prepass_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let prepass_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-ao-depth-normal-wgsl"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::AO_PREPASS_WGSL.into()),
        });
        let position_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };
        let normal_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 1,
            }],
        };
        let color_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 2,
            }],
        };
        let uv_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 3,
            }],
        };
        let depth_prepass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qualia-ao-depth-normal-prepass"),
            layout: Some(&depth_prepass_layout),
            vertex: wgpu::VertexState {
                module: &prepass_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(position_layout),
                    Some(normal_layout),
                    Some(color_layout),
                    Some(uv_layout),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &prepass_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let depth_normal_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("qualia-ao-input-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                                std::mem::size_of::<AoUniform>() as u64,
                            ),
                        },
                        count: None,
                    },
                ],
            });
        let ao_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-ao-pipeline-layout"),
            bind_group_layouts: &[Some(&depth_normal_layout)],
            immediate_size: 0,
        });
        let ao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-ao-generate-wgsl"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::AO_GENERATE_WGSL.into()),
        });
        let ao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qualia-ao-half-resolution"),
            layout: Some(&ao_layout),
            vertex: wgpu::VertexState {
                module: &ao_shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ao_shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let ao_generate_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-ao-generate-inputs"),
            layout: &depth_normal_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        Some(Self {
            _depth_texture: depth_texture,
            depth_view,
            _normal_texture: normal_texture,
            normal_view,
            _ao_texture: ao_texture,
            ao_view,
            _reservation: reservation,
            depth_prepass,
            depth_prepass_uniform_bind,
            ao_pipeline,
            ao_generate_bind,
            width,
            height,
        })
    }

    pub fn sample_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-ao-reconstruction-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<AoUniform>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn sample_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        ao_view: &wgpu::TextureView,
        packed_surface_view: &wgpu::TextureView,
        uniform_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-ao-reconstruction-inputs"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ao_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(packed_surface_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        })
    }

    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        mesh: &super::MeshGpu,
        camera_bind: &wgpu::BindGroup,
        model_bind: &wgpu::BindGroup,
        instance_bind: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("qualia-ao-depth-normal-prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 0.0,
                        a: 0.0,
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
            multiview_mask: None,
            timestamp_writes: None,
        });
        prepass.set_pipeline(&self.depth_prepass);
        prepass.set_bind_group(0, camera_bind, &[]);
        prepass.set_bind_group(1, model_bind, &[]);
        prepass.set_bind_group(2, &self.depth_prepass_uniform_bind, &[]);
        prepass.set_bind_group(5, instance_bind, &[]);
        prepass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
        prepass.set_vertex_buffer(1, mesh.normal_buf.slice(..));
        prepass.set_vertex_buffer(2, mesh.color_buf.slice(..));
        prepass.set_vertex_buffer(3, mesh.uv_buf.slice(..));
        prepass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        for draw in &mesh.material_gpu.draws {
            if draw.opacity_mode == crate::container_10d::OpacityMode::Blend {
                continue;
            }
            prepass.set_bind_group(3, &mesh.material_gpu.bind_group, &[draw.material_offset]);
            prepass.set_bind_group(
                4,
                &mesh.material_gpu.texture_bind_groups[draw.material_index],
                &[],
            );
            prepass.draw_indexed(
                draw.first_index..draw.first_index + draw.index_count,
                0,
                0..instance_count,
            );
        }
        drop(prepass);
        let mut ao = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("qualia-ao-evaluate-half-resolution"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.ao_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        ao.set_pipeline(&self.ao_pipeline);
        ao.set_bind_group(0, &self.ao_generate_bind, &[]);
        ao.draw(0..3, 0..1);
    }

    pub fn ao_view(&self) -> &wgpu::TextureView {
        &self.ao_view
    }
    pub fn normal_view(&self) -> &wgpu::TextureView {
        &self.normal_view
    }
    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn normal_texture_for_test(&self) -> &wgpu::Texture {
        &self._normal_texture
    }
}
