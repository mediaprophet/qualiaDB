//! WebGPU holographic RDF pass on PortalGpu (desktop upgrade over WebGL2).
//! Cinema look lives in QualiaDB `docs/shaders/rdf-holograph.wgsl` — etalon
//! `holographic.wgsl` stays lockstep and is not mutated here.

use etalon::util::DeviceExt;

use super::super::{holographic::HolographicPipelineGpu, PortalGpu, HDR_FORMAT};
use crate::render::gpu::holographic::HolographicNodeGpu;

const CINEMA_RDF_WGSL: &str = include_str!("../../../../../../docs/shaders/rdf-holograph.wgsl");

pub struct RdfHolographGpu {
    pipeline: HolographicPipelineGpu,
    pipeline_sdr: etalon::RenderPipeline,
    pipeline_hdr: etalon::RenderPipeline,
    node_buf: etalon::Buffer,
    node_bind: etalon::BindGroup,
    count: u32,
}

fn pipeline_for_format(
    device: &etalon::Device,
    camera_layout: &etalon::BindGroupLayout,
    node_layout: &etalon::BindGroupLayout,
    format: etalon::TextureFormat,
) -> etalon::RenderPipeline {
    let shader = device.create_shader_module(etalon::ShaderModuleDescriptor {
        label: Some("qualia-rdf-cinema-wgsl"),
        source: etalon::ShaderSource::Wgsl(CINEMA_RDF_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&etalon::PipelineLayoutDescriptor {
        label: Some("qualia-rdf-holographic-pipeline-layout"),
        bind_group_layouts: &[Some(camera_layout), Some(node_layout)],
        immediate_size: 0,
    });
    let glass = etalon::BlendState {
        color: etalon::BlendComponent {
            src_factor: etalon::BlendFactor::SrcAlpha,
            dst_factor: etalon::BlendFactor::OneMinusSrcAlpha,
            operation: etalon::BlendOperation::Add,
        },
        alpha: etalon::BlendComponent {
            src_factor: etalon::BlendFactor::One,
            dst_factor: etalon::BlendFactor::OneMinusSrcAlpha,
            operation: etalon::BlendOperation::Add,
        },
    };
    device.create_render_pipeline(&etalon::RenderPipelineDescriptor {
        label: Some("qualia-rdf-cinema-pipeline"),
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
                blend: Some(glass),
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
            depth_compare: Some(etalon::CompareFunction::LessEqual),
            stencil: etalon::StencilState::default(),
            bias: etalon::DepthBiasState::default(),
        }),
        multisample: etalon::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

impl PortalGpu {
    /// Uploads 96-byte holographic nodes into the WebGPU SSBO theatre.
    pub fn upload_holographic_nodes(&mut self, nodes: &[HolographicNodeGpu]) -> Result<u32, String> {
        if nodes.is_empty() {
            self.holographic = None;
            return Ok(0);
        }
        let bytes: &[u8] = bytemuck::cast_slice(nodes);
        let pipeline = HolographicPipelineGpu::new(
            &self.device,
            &self.projector_camera_layout,
            self.color_format,
        );
        let pipeline_sdr = pipeline_for_format(
            &self.device,
            &self.projector_camera_layout,
            &pipeline.node_storage_layout,
            self.color_format,
        );
        let pipeline_hdr = pipeline_for_format(
            &self.device,
            &self.projector_camera_layout,
            &pipeline.node_storage_layout,
            HDR_FORMAT,
        );
        let node_buf = self
            .device
            .create_buffer_init(&etalon::util::BufferInitDescriptor {
                label: Some("portal-rdf-holograph-nodes"),
                contents: bytes,
                usage: etalon::BufferUsages::STORAGE | etalon::BufferUsages::COPY_DST,
            });
        let node_bind = self.device.create_bind_group(&etalon::BindGroupDescriptor {
            label: Some("portal-rdf-holograph-nodes-bind"),
            layout: &pipeline.node_storage_layout,
            entries: &[etalon::BindGroupEntry {
                binding: 0,
                resource: node_buf.as_entire_binding(),
            }],
        });
        let count = u32::try_from(nodes.len()).map_err(|_| "holograph node count exceeds u32")?;
        self.holographic = Some(RdfHolographGpu {
            pipeline,
            pipeline_sdr,
            pipeline_hdr,
            node_buf,
            node_bind,
            count,
        });
        Ok(count)
    }

    /// Records the holographic RDF pass into an open scene render pass.
    pub(super) fn record_holographic<'a>(&'a self, pass: &mut etalon::RenderPass<'a>, hdr: bool) {
        let Some(h) = self.holographic.as_ref() else {
            return;
        };
        if h.count == 0 {
            return;
        }
        let pipeline = if hdr {
            &h.pipeline_hdr
        } else {
            &h.pipeline_sdr
        };
        let _ = &h.pipeline;
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.projector_camera_bind, &[]);
        pass.set_bind_group(1, &h.node_bind, &[]);
        pass.draw(0..36, 0..h.count);
        let _ = &h.node_buf;
    }
}
