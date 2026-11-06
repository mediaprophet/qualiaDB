//! Low-cost analytic fullscreen sky for negotiated HDR/SDR colour targets.

/// Creates `pipeline`.
pub(super) fn create_pipeline(
    device: &etalon::Device,
    camera_layout: &etalon::BindGroupLayout,
    format: etalon::TextureFormat,
) -> etalon::RenderPipeline {
    let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
        label: Some("qualia-analytic-sky-wgsl"),
        source: etalon::ShaderSource::Wgsl(crate::shaders::viewport::SKY_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
        label: Some("qualia-analytic-sky-layout"),
        bind_group_layouts: &[Some(camera_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
        label: Some("qualia-analytic-sky-pipeline"),
        layout: Some(&layout),
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
                format,
                blend: None,
                write_mask: etalon::ColorWrites::ALL,
            })],
        }),
        primitive: etalon::PrimitiveState {
            topology: etalon::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(etalon::DepthStencilState {
            format: etalon::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(etalon::CompareFunction::Always),
            stencil: etalon::StencilState::default(),
            bias: etalon::DepthBiasState::default(),
        }),
        multisample: etalon::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
