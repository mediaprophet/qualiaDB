//! Bloom post-pass (Kawase) — HDR extract → blur → composite for the portal viewport.
use super::*;
pub(super) fn portal_bloom_enabled() -> bool {
    let ledger = global_vram_ledger();
    universe_orchestrator().bloom_enabled(ComputeUniverse::Viewport, ledger.mode())
}

pub(super) fn probe_hdr_format(device: &wgpu::Device) -> bool {
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

pub(super) fn hdr_color_target_state() -> wgpu::ColorTargetState {
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

pub(super) fn bloom_vram_bytes(width: u32, height: u32) -> u64 {
    let w = width.max(1) as u64;
    let h = height.max(1) as u64;
    let hdr = w * h * 8;
    let half_w = (w / 2).max(1);
    let half_h = (h / 2).max(1);
    hdr + half_w * half_h * 8 * 2
}

pub(super) fn create_float_target(
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

pub(super) fn bloom_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

pub(super) fn make_bloom_bind_group(
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

pub(super) fn create_bloom_chain(
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

pub(super) fn write_bloom_uniform(queue: &wgpu::Queue, buf: &wgpu::Buffer, offset: f32) {
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

pub(super) fn run_bloom_passes(
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
