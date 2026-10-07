//! Bloom post-pass (Kawase) — HDR extract → blur → composite for the portal viewport.
use super::*;
use crate::gpu_context::{global_vram_ledger, VramResourceClass};
pub(super) fn portal_bloom_enabled() -> bool {
    let ledger = global_vram_ledger();
    universe_orchestrator().bloom_enabled(ComputeUniverse::Viewport, ledger.mode())
}

fn hdr_composite_available(
    has_bloom_targets: bool,
    has_ambient_pipeline: bool,
    has_projector_pipeline: bool,
    bloom_policy_enabled: bool,
) -> bool {
    has_bloom_targets && has_ambient_pipeline && has_projector_pipeline && bloom_policy_enabled
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

pub(super) fn bloom_vram_bytes(width: u32, height: u32) -> Option<u64> {
    let w = width.max(1) as u64;
    let h = height.max(1) as u64;
    let hdr = w.checked_mul(h)?.checked_mul(8)?;
    let half_w = (w / 2).max(1);
    let half_h = (h / 2).max(1);
    let half = half_w.checked_mul(half_h)?.checked_mul(8)?.checked_mul(2)?;
    hdr.checked_add(half)?
        .checked_add(8)? // one 1x1 dummy target
        .checked_add(std::mem::size_of::<BloomUniformBlock>() as u64)
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
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<BloomUniformBlock>() as u64,
                    ),
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
        ],
    })
}

pub(super) fn create_bloom_chain(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    surface_format: wgpu::TextureFormat,
    exposure_ev: f32,
) -> Option<BloomChain> {
    if !portal_bloom_enabled() {
        return None;
    }

    // Admit all persistent colour targets before creating any texture. During
    // resize the old BloomChain remains alive, so this reservation represents
    // the true old+new peak until the replacement is committed.
    let vram_bytes = bloom_vram_bytes(width, height)?;
    let frame_target_reservation = global_vram_ledger()
        .try_reserve_graphics(VramResourceClass::FrameTarget, vram_bytes)
        .ok()?;

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
                surface_is_srgb: if surface_format.is_srgb() { 1.0 } else { 0.0 },
                _pad1: 0.0,
                white_balance_gains: crate::render::output::DEFAULT_WHITE_BALANCE_GAINS,
                _pad2: 0.0,
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
        bind_group_layouts: &[Some(&bind_layout)],
        immediate_size: 0,
    });

    let extract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-extract"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: Some("extract_vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: Some("extract_fs"),
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
        multiview_mask: None,
        cache: None,
    });

    let kawase_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-kawase"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: Some("kawase_vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: Some("kawase_fs"),
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
        multiview_mask: None,
        cache: None,
    });

    let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-bloom-composite"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bloom_shader,
            entry_point: Some("composite_vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &bloom_shader,
            entry_point: Some("composite_fs"),
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
        exposure_scale: crate::render::output::exposure_scale_from_ev(exposure_ev)?,
        surface_is_srgb: surface_format.is_srgb(),
        white_balance_gains: crate::render::output::DEFAULT_WHITE_BALANCE_GAINS,
        _frame_target_reservation: frame_target_reservation,
    })
}

pub(super) fn write_bloom_uniform(
    queue: &wgpu::Queue,
    buf: &wgpu::Buffer,
    offset: f32,
    exposure_scale: f32,
    surface_is_srgb: bool,
    white_balance_gains: [f32; 3],
) {
    let block = BloomUniformBlock {
        bloom: BloomParamsGpu {
            threshold: BLOOM_THRESHOLD,
            intensity: BLOOM_INTENSITY,
            offset,
            _pad: 0.0,
        },
        composite: CompositeParamsGpu {
            exposure: exposure_scale,
            bloom_strength: BLOOM_STRENGTH,
            surface_is_srgb: if surface_is_srgb { 1.0 } else { 0.0 },
            _pad1: 0.0,
            white_balance_gains,
            _pad2: 0.0,
        },
    };
    queue.write_buffer(buf, 0, bytemuck::bytes_of(&block));
}

impl PortalGpu {
    /// Set bounded manual HDR exposure compensation in stops (EV).
    /// The value remains configured and is applied by whichever scene-output path is active.
    pub fn set_exposure_compensation(&mut self, compensation_ev: f32) -> bool {
        if !compensation_ev.is_finite() {
            return false;
        }
        let Some(exposure_scale) = crate::render::output::exposure_scale_from_ev(compensation_ev)
        else {
            return false;
        };
        let compensation_ev = compensation_ev.clamp(
            crate::render::output::MIN_HDR_EXPOSURE_EV,
            crate::render::output::MAX_HDR_EXPOSURE_EV,
        );
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            self.sync_bloom_targets();
        }
        self.hdr_exposure_ev = compensation_ev;
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.exposure_scale = exposure_scale;
        }
        let output_available = self
            .output_chain
            .as_mut()
            .map(|output| output.set_exposure(&self.queue, compensation_ev))
            .unwrap_or(false);
        self.hdr_exposure_available() || output_available
    }

    /// Compatibility alias for the original HDR-specific API name.
    pub fn set_hdr_exposure_compensation(&mut self, compensation_ev: f32) -> bool {
        self.set_exposure_compensation(compensation_ev)
    }

    /// Set bounded pre-tone-map white balance, retaining controls across target rebuilds.
    pub fn set_white_balance(&mut self, temperature_ev: f32, tint_ev: f32) -> bool {
        let Some(gains) =
            crate::render::output::white_balance_gains_from_stops(temperature_ev, tint_ev)
        else {
            return false;
        };
        self.white_balance_gains = gains;
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.white_balance_gains = gains;
        }
        let output_available = if let Some(output) = self.output_chain.as_mut() {
            output.set_white_balance(&self.queue, gains);
            true
        } else {
            false
        };
        self.hdr_exposure_available() || output_available
    }

    pub fn hdr_exposure_compensation(&self) -> f32 {
        self.hdr_exposure_ev
    }

    pub fn hdr_exposure_available(&self) -> bool {
        hdr_composite_available(
            self.bloom.is_some(),
            self.ambient_pipeline_hdr.is_some(),
            self.projector_pipeline_hdr.is_some(),
            portal_bloom_enabled(),
        )
    }

    /// Whether the active HDR or SDR scene-to-display path applies manual exposure.
    pub fn exposure_transform_available(&self) -> bool {
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            return false;
        }
        self.hdr_exposure_available() || self.output_chain.is_some()
    }

    /// Whether the current WebGPU scene path retains values above SDR white before output mapping.
    pub fn hdr_scene_available(&self) -> bool {
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            return false;
        }
        let hdr_pipelines = self.ambient_pipeline_hdr.is_some()
            && self.projector_pipeline_hdr.is_some()
            && self.sky_pipeline_hdr.is_some()
            && self.mesh_pipeline_hdr.is_some();
        hdr_pipelines
            && (self.hdr_exposure_available()
                || self
                    .output_chain
                    .as_ref()
                    .is_some_and(|o| o.uses_hdr_scene()))
    }
}

#[cfg(test)]
mod tests {
    use super::{bloom_vram_bytes, hdr_composite_available, BloomUniformBlock, CompositeParamsGpu};

    #[test]
    fn bloom_output_uniform_includes_white_balance_gains_with_wgsl_alignment() {
        assert_eq!(std::mem::size_of::<CompositeParamsGpu>(), 32);
        assert_eq!(std::mem::size_of::<BloomUniformBlock>(), 48);
    }

    #[test]
    fn exposure_capability_requires_the_active_composite_path() {
        assert!(hdr_composite_available(true, true, true, true));
        assert!(!hdr_composite_available(false, true, true, true));
        assert!(!hdr_composite_available(true, false, true, true));
        assert!(!hdr_composite_available(true, true, false, true));
        assert!(!hdr_composite_available(true, true, true, false));
    }

    #[test]
    fn bloom_budget_counts_hdr_ping_pong_dummy_and_uniform_buffer() {
        assert_eq!(bloom_vram_bytes(1920, 1080), Some(24_883_256));
        assert_eq!(bloom_vram_bytes(1, 1), Some(80));
    }

    #[test]
    fn bloom_target_budget_rejects_size_overflow() {
        assert_eq!(bloom_vram_bytes(u32::MAX, u32::MAX), None);
    }
}

pub(super) fn run_bloom_passes(
    encoder: &mut wgpu::CommandEncoder,
    bloom: &BloomChain,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    surface_view: &wgpu::TextureView,
    clear_color: [f64; 4],
) {
    let (blur_w, blur_h) = bloom.blur_extent();
    let _ = bloom.texture_handles();
    debug_assert_eq!(blur_w, bloom.blur_a.width());
    debug_assert_eq!(blur_h, bloom.blur_b.height());

    write_bloom_uniform(
        queue,
        &bloom.uniform_buf,
        1.0,
        bloom.exposure_scale,
        bloom.surface_is_srgb,
        bloom.white_balance_gains,
    );
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
        pass.set_pipeline(&bloom.extract_pipeline);
        pass.set_bind_group(0, &extract_bind, &[]);
        pass.draw(0..3, 0..1);
    }

    let mut read = &bloom.blur_a_view;
    let mut write = &bloom.blur_b_view;
    for &offset in &KAWASE_OFFSETS {
        write_bloom_uniform(
            queue,
            &bloom.uniform_buf,
            offset,
            bloom.exposure_scale,
            bloom.surface_is_srgb,
            bloom.white_balance_gains,
        );
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
            pass.set_pipeline(&bloom.kawase_pipeline);
            pass.set_bind_group(0, &kawase_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        std::mem::swap(&mut read, &mut write);
    }
    let bloom_result = read;

    write_bloom_uniform(
        queue,
        &bloom.uniform_buf,
        1.0,
        bloom.exposure_scale,
        bloom.surface_is_srgb,
        bloom.white_balance_gains,
    );
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
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color[0],
                        g: clear_color[1],
                        b: clear_color[2],
                        a: clear_color[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&bloom.composite_pipeline);
        pass.set_bind_group(0, &composite_bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
