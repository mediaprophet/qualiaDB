//! Portable render-pass mip generation for resident RGBA8 material textures.
//!
//! Mips are generated on-GPU from the preceding level. sRGB reads decode to linear and sRGB
//! render targets encode the averaged result; normal maps average decoded vectors and renormalize.

use wgpu::util::DeviceExt;

#[path = "alpha_coverage.rs"]
mod alpha_coverage;
use alpha_coverage::alpha_coverage_plan as build_alpha_coverage_plan;

pub(super) fn alpha_coverage_plan(
    rgba8: &[u8],
    width: u32,
    height: u32,
    cutoff: f32,
) -> alpha_coverage::AlphaCoveragePlan {
    build_alpha_coverage_plan(rgba8, width, height, cutoff)
}

/// Bounded prediction for cutoff coverage after RGBA8 quantization of each generated mip.
///
/// The residual is measured against the uncorrected base level. It is computed by the CPU
/// reference filter and is not a GPU readback measurement; hardware sampling and rasterization
/// still require pixel qualification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaCoverageDiagnostics {
    pub base_coverage: f32,
    pub level_count: usize,
    pub signed_residuals: [f32; 32],
}

impl From<alpha_coverage::AlphaCoveragePlan> for AlphaCoverageDiagnostics {
    fn from(plan: alpha_coverage::AlphaCoveragePlan) -> Self {
        Self {
            base_coverage: plan.base_coverage,
            level_count: plan.level_count,
            signed_residuals: plan.residuals,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureMipSemantic {
    Color,
    /// sRGB colour mips whose alpha is corrected to retain coverage at this cutoff.
    AlphaMask {
        cutoff_bits: u32,
    },
    LinearData,
    Normal,
}

impl TextureMipSemantic {
    pub fn alpha_mask(cutoff: f32) -> Option<Self> {
        (cutoff.is_finite() && (0.0..=1.0).contains(&cutoff)).then_some(Self::AlphaMask {
            cutoff_bits: if cutoff == 0.0 {
                0.0f32.to_bits()
            } else {
                cutoff.to_bits()
            },
        })
    }

    pub(super) fn alpha_cutoff(self) -> Option<f32> {
        match self {
            Self::AlphaMask { cutoff_bits } => {
                let cutoff = f32::from_bits(cutoff_bits);
                (cutoff.is_finite() && (0.0..=1.0).contains(&cutoff)).then_some(cutoff)
            }
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MipParams {
    values: [f32; 4],
}

pub(super) struct MipGenerator {
    bind_layout: wgpu::BindGroupLayout,
    srgb_pipeline: wgpu::RenderPipeline,
    linear_pipeline: wgpu::RenderPipeline,
    normal_pipeline: wgpu::RenderPipeline,
}

impl MipGenerator {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-texture-mip-source-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
                        min_binding_size: std::num::NonZeroU64::new(16),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-texture-mip-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-texture-mip-shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/viewport/mip_generate.wgsl").into(),
            ),
        });
        let create_pipeline = |label, entry_point, format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
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
            })
        };
        Self {
            bind_layout,
            srgb_pipeline: create_pipeline(
                "qualia-texture-mip-srgb",
                "fragment_color",
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ),
            linear_pipeline: create_pipeline(
                "qualia-texture-mip-linear",
                "fragment_linear_data",
                wgpu::TextureFormat::Rgba8Unorm,
            ),
            normal_pipeline: create_pipeline(
                "qualia-texture-mip-normal",
                "fragment_normal",
                wgpu::TextureFormat::Rgba8Unorm,
            ),
        }
    }

    pub fn generate(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        mip_level_count: u32,
        semantic: TextureMipSemantic,
        alpha_plan: Option<&alpha_coverage::AlphaCoveragePlan>,
    ) {
        if mip_level_count <= 1 {
            queue.submit(std::iter::empty());
            return;
        }
        let pipeline = match semantic {
            TextureMipSemantic::Color | TextureMipSemantic::AlphaMask { .. } => &self.srgb_pipeline,
            TextureMipSemantic::LinearData => &self.linear_pipeline,
            TextureMipSemantic::Normal => &self.normal_pipeline,
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("qualia-texture-mip-encoder"),
        });
        for mip_level in 1..mip_level_count {
            let source = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("qualia-texture-mip-source"),
                base_mip_level: mip_level - 1,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let destination = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("qualia-texture-mip-destination"),
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let alpha_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("qualia-texture-mip-alpha-coverage"),
                contents: bytemuck::bytes_of(&MipParams {
                    values: [
                        alpha_plan.map_or(1.0, |plan| plan.scales[mip_level as usize]),
                        alpha_plan.map_or(0.0, |plan| plan.rounding_biases[mip_level as usize]),
                        0.0,
                        0.0,
                    ],
                }),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("qualia-texture-mip-source-bind"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: alpha_buffer.as_entire_binding(),
                    },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("qualia-texture-mip-level"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &destination,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}
