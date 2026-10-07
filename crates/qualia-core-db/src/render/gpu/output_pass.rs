//! Final scene-linear-to-SDR pass used when the HDR bloom chain is unavailable.

use super::*;

const OUTPUT_PARAMS_BYTES: u64 = 32;

fn output_target_vram_bytes(width: u32, height: u32, bytes_per_pixel: u32) -> Option<u64> {
    u64::from(width.max(1))
        .checked_mul(u64::from(height.max(1)))?
        .checked_mul(u64::from(bytes_per_pixel))?
        .checked_add(OUTPUT_PARAMS_BYTES)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OutputParams {
    exposure_scale: f32,
    surface_is_srgb: f32,
    _padding: [f32; 2],
    white_balance_gains: [f32; 3],
    _padding2: f32,
}

/// Budgeted scene target and final SDR transform for the portable non-bloom path.
pub(super) struct OutputChain {
    _scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    composite_pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
    exposure_scale: f32,
    surface_is_srgb: bool,
    hdr_scene: bool,
    white_balance_gains: [f32; 3],
    _frame_target_reservation: crate::gpu_context::VramReservation<'static>,
}

impl OutputChain {
    pub(super) fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scene_format: wgpu::TextureFormat,
        surface_format: wgpu::TextureFormat,
        exposure_ev: f32,
        hdr_scene: bool,
        white_balance_gains: [f32; 3],
    ) -> Option<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let exposure_scale = crate::render::output::exposure_scale_from_ev(exposure_ev)?;
        let reservation_bytes =
            output_target_vram_bytes(width, height, scene_format.block_copy_size(None)?)?;
        let reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FrameTarget,
                reservation_bytes,
            )
            .ok()?;

        let scene_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-sdr-scene-linear"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: scene_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("qualia-sdr-output-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params = OutputParams {
            exposure_scale,
            surface_is_srgb: if surface_format.is_srgb() { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
            white_balance_gains,
            _padding2: 0.0,
        };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qualia-sdr-output-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-sdr-output-layout"),
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(OUTPUT_PARAMS_BYTES),
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-sdr-output-bind"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-sdr-output"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::OUTPUT_WGSL.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-sdr-output-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qualia-sdr-output-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("output_vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("output_fs"),
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
            multiview_mask: None,
            cache: None,
        });

        Some(Self {
            _scene_texture: scene_texture,
            scene_view,
            _sampler: sampler,
            bind_group,
            uniform_buf,
            composite_pipeline,
            width,
            height,
            exposure_scale,
            surface_is_srgb: surface_format.is_srgb(),
            hdr_scene,
            white_balance_gains,
            _frame_target_reservation: reservation,
        })
    }

    pub(super) fn scene_view(&self) -> &wgpu::TextureView {
        &self.scene_view
    }

    pub(super) fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(super) fn uses_hdr_scene(&self) -> bool {
        self.hdr_scene
    }

    pub(super) fn set_exposure(&mut self, queue: &wgpu::Queue, exposure_ev: f32) -> bool {
        let Some(exposure_scale) = crate::render::output::exposure_scale_from_ev(exposure_ev)
        else {
            return false;
        };
        self.exposure_scale = exposure_scale;
        let params = OutputParams {
            exposure_scale,
            surface_is_srgb: if self.surface_is_srgb { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
            white_balance_gains: self.white_balance_gains,
            _padding2: 0.0,
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&params));
        true
    }

    pub(super) fn set_white_balance(&mut self, queue: &wgpu::Queue, white_balance_gains: [f32; 3]) {
        self.white_balance_gains = white_balance_gains;
        let params = OutputParams {
            exposure_scale: self.exposure_scale,
            surface_is_srgb: if self.surface_is_srgb { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
            white_balance_gains,
            _padding2: 0.0,
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&params));
    }

    pub(super) fn composite(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("qualia-sdr-output"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{output_target_vram_bytes, OutputParams};

    #[test]
    fn sdr_output_budget_counts_the_full_scene_target_and_uniform() {
        assert_eq!(output_target_vram_bytes(1920, 1080, 4), Some(8_294_432));
        assert_eq!(output_target_vram_bytes(1920, 1080, 8), Some(16_588_832));
        assert_eq!(std::mem::size_of::<OutputParams>(), 32);
        assert_eq!(output_target_vram_bytes(u32::MAX, u32::MAX, 8), None);
    }
}
