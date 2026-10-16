//! Holographic 3D Engine for Semantic Graphs — Procedural Volumetric Light Field.
//!
//! Architected per `docs/Holographic 3D Engine for Semantic Graphs.md`:
//! - Proxy wireframe bounding volumes with zero CPU triangle meshing.
//! - Resolution-independent procedural fragment raymarching using C¹ smooth-min SDFs.
//! - Wave interference for N3Logic reasoning (constructive focal points and luminous bridges).
//! - Direct EMF spectral frequency emission into the CIE 1931 XYZ -> linear sRGB spectrum.

use bytemuck::{Pod, Zeroable};

/// Primitive geometric archetype evaluated procedurally inside the holographic shader.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolographicPrimitive {
    /// Harmonic pulsating sphere.
    Sphere = 0,
    /// Axis-aligned rounded box with continuous edge blending.
    RoundedBox = 1,
    /// Dynamic directional capsule.
    Capsule = 2,
    /// Multi-harmonic organic metaball field.
    Metaball = 3,
    /// Relational interference bridge linking two semantic entities.
    InterferenceBridge = 4,
}

/// 96-byte std140/std430 aligned GPU representation of a semantic holographic entity.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HolographicNodeGpu {
    /// World position xyz, bounding radius / proxy scale w.
    pub position_scale: [f32; 4],
    /// Half-extents xyz, primitive type w (`HolographicPrimitive` as u32/f32).
    pub half_extents_primitive: [f32; 4],
    /// Frequency x, amplitude y, phase z, manifold curvature w.
    pub frequency_amplitude: [f32; 4],
    /// 10D Manifold tensor: epistemic_q x, topological_spin y, temporal_decay z, density_threshold w.
    pub manifold_tensor: [f32; 4],
    /// Target endpoint xyz (for relational bridges), coherence weight w.
    pub connection_endpoint: [f32; 4],
    /// Modality emissive color rgba.
    pub emissive_color: [f32; 4],
}

const _: [(); 96] = [(); std::mem::size_of::<HolographicNodeGpu>()];

impl Default for HolographicNodeGpu {
    fn default() -> Self {
        Self {
            position_scale: [0.0, 0.0, 0.0, 1.0],
            half_extents_primitive: [1.0, 1.0, 1.0, HolographicPrimitive::Sphere as u32 as f32],
            frequency_amplitude: [1.0, 1.0, 0.0, 1.0],
            manifold_tensor: [1.0, 0.0, 0.0, 0.5],
            connection_endpoint: [0.0, 0.0, 0.0, 1.0],
            emissive_color: [0.0, 0.9, 1.0, 1.0],
        }
    }
}

impl HolographicNodeGpu {
    /// Constructs a holographic node in the hot path without heap allocation.
    #[inline]
    pub const fn new(
        position: [f32; 3],
        scale: f32,
        half_extents: [f32; 3],
        primitive: HolographicPrimitive,
        frequency: f32,
        amplitude: f32,
        phase: f32,
        curvature: f32,
        epistemic_q: f32,
        color: [f32; 4],
    ) -> Self {
        Self {
            position_scale: [position[0], position[1], position[2], scale],
            half_extents_primitive: [
                half_extents[0],
                half_extents[1],
                half_extents[2],
                primitive as u32 as f32,
            ],
            frequency_amplitude: [frequency, amplitude, phase, curvature],
            manifold_tensor: [epistemic_q, 0.0, 0.0, 0.5],
            connection_endpoint: [0.0, 0.0, 0.0, 0.0],
            emissive_color: color,
        }
    }

    /// Configures a relational interference bridge linking this node to a target node.
    #[inline]
    pub fn with_interference_bridge(mut self, target: [f32; 3], coherence_weight: f32) -> Self {
        self.half_extents_primitive[3] = HolographicPrimitive::InterferenceBridge as u32 as f32;
        self.connection_endpoint = [target[0], target[1], target[2], coherence_weight];
        self
    }
}

/// WebGPU pipeline wrapper for the Holographic 3D Engine.
pub struct HolographicPipelineGpu {
    pub pipeline: wgpu::RenderPipeline,
    pub node_storage_layout: wgpu::BindGroupLayout,
}

impl HolographicPipelineGpu {
    /// Creates the holographic render pipeline and storage layout.
    pub fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let node_storage_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("qualia-holographic-node-storage-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-holographic-wgsl"),
            source: wgpu::ShaderSource::Wgsl(crate::shaders::viewport::HOLOGRAPHIC_WGSL.into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-holographic-pipeline-layout"),
            bind_group_layouts: &[Some(camera_layout), Some(&node_storage_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("qualia-holographic-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            node_storage_layout,
        }
    }
}
