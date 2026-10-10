//! Hardware-accelerated motion-vector and reactive-mask producer pipelines for temporal accumulation.
//!
//! Generates screen-space camera motion vectors (Rg16Float) by unprojecting scene depth with
//! `inv_current_view_proj` and reprojecting with `previous_view_proj`, alongside normalized
//! reactive masks (R8Unorm) to attenuate history weighting over high-frequency alpha edges,
//! water surfaces, and particle fields.

use crate::shaders::viewport::{TEMPORAL_MOTION_WGSL, TEMPORAL_REACTIVE_WGSL};
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;

pub const MOTION_VECTOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
pub const REACTIVE_MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraMotionParamsGpu {
    pub inv_current_view_proj: [[f32; 4]; 4],
    pub previous_view_proj: [[f32; 4]; 4],
    pub viewport: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ReactiveParamsGpu {
    pub weights: [f32; 4],
}

/// Renderer-owned GPU producers for temporal resolve.
#[allow(dead_code)]
pub struct TemporalProducersGpu {
    motion_texture: wgpu::Texture,
    motion_view: wgpu::TextureView,
    reactive_texture: wgpu::Texture,
    reactive_view: wgpu::TextureView,
    motion_pipeline: wgpu::RenderPipeline,
    motion_params_buf: wgpu::Buffer,
    motion_bind_group: wgpu::BindGroup,
    motion_bind_layout: wgpu::BindGroupLayout,
    motion_sampler: wgpu::Sampler,
    reactive_pipeline: wgpu::RenderPipeline,
    reactive_params_buf: wgpu::Buffer,
    reactive_bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

#[allow(dead_code)]
impl TemporalProducersGpu {
    pub fn try_new(
        device: &wgpu::Device,
        scene_depth_view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        let width = width.max(1);
        let height = height.max(1);

        let (motion_texture, motion_view) =
            Self::create_motion_target(device, width, height)?;
        let (reactive_texture, reactive_view) =
            Self::create_reactive_target(device, width, height)?;

        let motion_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal-motion-shader"),
            source: wgpu::ShaderSource::Wgsl(TEMPORAL_MOTION_WGSL.into()),
        });

        let reactive_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal-reactive-shader"),
            source: wgpu::ShaderSource::Wgsl(TEMPORAL_REACTIVE_WGSL.into()),
        });

        let motion_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temporal-motion-params-buf"),
            size: std::mem::size_of::<CameraMotionParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let motion_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("temporal-motion-depth-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let motion_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("temporal-motion-bind-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
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
                            min_binding_size: Some(
                                NonZeroU64::new(
                                    std::mem::size_of::<CameraMotionParamsGpu>() as u64,
                                )
                                .unwrap(),
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let motion_bind_group = Self::build_motion_bind_group(
            device,
            &motion_bind_layout,
            scene_depth_view,
            &motion_params_buf,
            &motion_sampler,
        );

        let motion_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("temporal-motion-pipeline-layout"),
                bind_group_layouts: &[Some(&motion_bind_layout)],
                immediate_size: 0,
            });

        let motion_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("temporal-motion-pipeline"),
            layout: Some(&motion_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &motion_shader,
                entry_point: Some("temporal_motion_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &motion_shader,
                entry_point: Some("temporal_motion_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: MOTION_VECTOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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

        let reactive_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temporal-reactive-params-buf"),
            size: std::mem::size_of::<ReactiveParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reactive_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("temporal-reactive-bind-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZeroU64::new(std::mem::size_of::<ReactiveParamsGpu>() as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        let reactive_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temporal-reactive-bind-group"),
            layout: &reactive_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: reactive_params_buf.as_entire_binding(),
            }],
        });

        let reactive_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("temporal-reactive-pipeline-layout"),
                bind_group_layouts: &[Some(&reactive_bind_layout)],
                immediate_size: 0,
            });

        let reactive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("temporal-reactive-pipeline"),
            layout: Some(&reactive_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &reactive_shader,
                entry_point: Some("temporal_reactive_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &reactive_shader,
                entry_point: Some("temporal_reactive_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: REACTIVE_MASK_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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

        Some(Self {
            motion_texture,
            motion_view,
            reactive_texture,
            reactive_view,
            motion_pipeline,
            motion_params_buf,
            motion_bind_group,
            motion_bind_layout,
            motion_sampler,
            reactive_pipeline,
            reactive_params_buf,
            reactive_bind_group,
            width,
            height,
        })
    }

    fn create_motion_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Option<(wgpu::Texture, wgpu::TextureView)> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("temporal-motion-vectors"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: MOTION_VECTOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some((texture, view))
    }

    fn create_reactive_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Option<(wgpu::Texture, wgpu::TextureView)> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("temporal-reactive-mask"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: REACTIVE_MASK_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some((texture, view))
    }

    fn build_motion_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        depth_view: &wgpu::TextureView,
        params_buf: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temporal-motion-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        scene_depth_view: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        let width = width.max(1);
        let height = height.max(1);
        if self.width == width && self.height == height {
            return;
        }
        if let Some((tex, view)) = Self::create_motion_target(device, width, height) {
            self.motion_texture = tex;
            self.motion_view = view;
        }
        if let Some((tex, view)) = Self::create_reactive_target(device, width, height) {
            self.reactive_texture = tex;
            self.reactive_view = view;
        }
        self.motion_bind_group = Self::build_motion_bind_group(
            device,
            &self.motion_bind_layout,
            scene_depth_view,
            &self.motion_params_buf,
            &self.motion_sampler,
        );
        self.width = width;
        self.height = height;
    }

    pub fn update_scene_depth_view(
        &mut self,
        device: &wgpu::Device,
        scene_depth_view: &wgpu::TextureView,
    ) {
        self.motion_bind_group = Self::build_motion_bind_group(
            device,
            &self.motion_bind_layout,
            scene_depth_view,
            &self.motion_params_buf,
            &self.motion_sampler,
        );
    }

    pub fn record_producers(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        inv_current_view_proj: [[f32; 4]; 4],
        previous_view_proj: [[f32; 4]; 4],
        reactive_weights: [f32; 4],
    ) {
        let motion_params = CameraMotionParamsGpu {
            inv_current_view_proj,
            previous_view_proj,
            viewport: [
                self.width as f32,
                self.height as f32,
                1.0 / self.width as f32,
                1.0 / self.height as f32,
            ],
        };
        queue.write_buffer(
            &self.motion_params_buf,
            0,
            bytemuck::bytes_of(&motion_params),
        );

        let reactive_params = ReactiveParamsGpu {
            weights: reactive_weights,
        };
        queue.write_buffer(
            &self.reactive_params_buf,
            0,
            bytemuck::bytes_of(&reactive_params),
        );

        {
            let mut motion_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("temporal-motion-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.motion_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            motion_pass.set_pipeline(&self.motion_pipeline);
            motion_pass.set_bind_group(0, &self.motion_bind_group, &[]);
            motion_pass.draw(0..3, 0..1);
        }

        {
            let mut reactive_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("temporal-reactive-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.reactive_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            reactive_pass.set_pipeline(&self.reactive_pipeline);
            reactive_pass.set_bind_group(0, &self.reactive_bind_group, &[]);
            reactive_pass.draw(0..3, 0..1);
        }
    }

    pub fn motion_view(&self) -> &wgpu::TextureView {
        &self.motion_view
    }

    pub fn reactive_view(&self) -> &wgpu::TextureView {
        &self.reactive_view
    }

    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
