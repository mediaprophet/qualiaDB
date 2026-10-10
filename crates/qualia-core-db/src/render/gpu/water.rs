//! Persistent HMC water geometry and depth-owned transparent rendering.

use super::{depth_stencil_state_read_only, PortalGpu};
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use crate::render::gpu::uniform_belt::UniformBelt;
use crate::shaders::viewport::WATER_WGSL;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub(super) const MAX_WATER_VERTICES: usize = 1_048_576;
pub(super) const MAX_WATER_INDICES: usize = 3_145_728;

/// Water quality tiers are deliberately numeric because the value crosses the
/// existing uniform ABI without adding another bind group or per-frame resource.
pub(super) const WATER_QUALITY_LOWER: u32 = 0;
pub(super) const WATER_QUALITY_BALANCED: u32 = 1;
pub(super) const WATER_QUALITY_CINEMATIC: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterUniform {
    colour_alpha: [f32; 4],
    wave: [f32; 4],
    quality: [f32; 4], // tier, reflection, foam, shore response
    viewport: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterVertex {
    position: [f32; 3],
    uv: [f32; 2],
}

pub(super) struct WaterGpu {
    pipeline: wgpu::RenderPipeline,
    pipeline_hdr: Option<wgpu::RenderPipeline>,
    uniform_buf: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    uniform: WaterUniform,
    vertex_buf: Option<wgpu::Buffer>,
    index_buf: Option<wgpu::Buffer>,
    index_count: u32,
    geometry_key: Option<u64>,
    geometry_bytes: u64,
    geometry_generation: u64,
    _geometry_reservation: Option<VramReservation<'static>>,
}

fn geometry_key(positions: &[[f32; 3]], indices: &[u32]) -> u64 {
    // FNV-1a is sufficient for the cache key: the upload path still validates
    // every element, and a collision only costs a harmless skipped rebuild.
    let mut key = 0xcbf29ce484222325u64;
    for byte in bytemuck::cast_slice::<[f32; 3], u8>(positions)
        .iter()
        .chain(bytemuck::cast_slice::<u32, u8>(indices).iter())
    {
        key ^= u64::from(*byte);
        key = key.wrapping_mul(0x100000001b3);
    }
    key ^ ((positions.len() as u64) << 32) ^ indices.len() as u64
}

fn geometry_admission(
    positions: &[[f32; 3]],
    indices: &[u32],
    max_bytes: u64,
) -> Result<(u64, u64), String> {
    if positions.is_empty() || indices.is_empty() {
        return Ok((0, 0));
    }
    if positions.len() > MAX_WATER_VERTICES
        || indices.len() > MAX_WATER_INDICES
        || indices.len() % 3 != 0
        || indices.iter().any(|&index| index as usize >= positions.len())
        || positions.iter().any(|position| position.iter().any(|value| !value.is_finite()))
    {
        return Err("HMC water geometry exceeds bounded or finite-input contract".into());
    }
    let bytes = (positions.len() as u64)
        .checked_mul(std::mem::size_of::<WaterVertex>() as u64)
        .and_then(|value| {
            value.checked_add((indices.len() as u64).checked_mul(std::mem::size_of::<u32>() as u64)?)
        })
        .ok_or_else(|| "HMC water geometry byte size overflow".to_string())?;
    if bytes > max_bytes {
        return Err(format!(
            "HMC water geometry exceeds admission budget: {bytes} > {max_bytes} bytes"
        ));
    }
    Ok((bytes, geometry_key(positions, indices)))
}

fn colour_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

impl WaterGpu {
    pub(super) fn new(
        device: &wgpu::Device,
        mesh_frame_layout: &wgpu::BindGroupLayout,
        mesh_model_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        hdr_supported: bool,
    ) -> Self {
        let uniform = WaterUniform {
            colour_alpha: [0.08, 0.34, 0.52, 0.72],
            wave: [0.0, 0.025, 2.0, 0.08],
            quality: [WATER_QUALITY_BALANCED as f32, 0.35, 0.60, 0.70],
            viewport: [1.0, 1.0, 0.0005, 0.0],
        };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qualia-water-frame-uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia-water-uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<WaterUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia-water"),
            source: wgpu::ShaderSource::Wgsl(WATER_WGSL.into()),
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("qualia-water-uniform-bind"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia-water-pipeline-layout"),
            bind_group_layouts: &[
                Some(mesh_frame_layout),
                Some(mesh_model_layout),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WaterVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        };
        let pipeline = |target_format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("qualia-water-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(vertex_layout.clone())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(colour_target(target_format))],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(depth_stencil_state_read_only()),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline_hdr = hdr_supported.then(|| pipeline(wgpu::TextureFormat::Rgba16Float));
        Self {
            pipeline: pipeline(format),
            pipeline_hdr,
            uniform_buf,
            uniform_bind,
            uniform,
            vertex_buf: None,
            index_buf: None,
            index_count: 0,
            geometry_key: None,
            geometry_bytes: 0,
            geometry_generation: 0,
            _geometry_reservation: None,
        }
    }

    pub(super) fn set_viewport(&mut self, width: u32, height: u32) {
        self.uniform.viewport[0] = width.max(1) as f32;
        self.uniform.viewport[1] = height.max(1) as f32;
    }

    pub(super) fn set_time(&mut self, time: f32) {
        self.uniform.wave[0] = time;
    }

    pub(super) fn write_uniform(&self, belt: &mut UniformBelt, encoder: &mut wgpu::CommandEncoder) {
        belt.write_and_unmap(bytemuck::bytes_of(&self.uniform));
        belt.record_copy(encoder, &self.uniform_buf, 0);
        // The caller advances the shared belt after this copy, keeping all
        // frame-owned writes in one preallocated ring.
    }

    pub(super) fn replace_geometry(
        &mut self,
        device: &wgpu::Device,
        positions: &[[f32; 3]],
        indices: &[u32],
    ) -> Result<(), String> {
        self.replace_geometry_with_budget(device, positions, indices, u64::MAX)
    }

    pub(super) fn replace_geometry_with_budget(
        &mut self,
        device: &wgpu::Device,
        positions: &[[f32; 3]],
        indices: &[u32],
        max_bytes: u64,
    ) -> Result<(), String> {
        if positions.is_empty() || indices.is_empty() {
            self.clear_geometry();
            return Ok(());
        }
        let (bytes, key) = geometry_admission(positions, indices, max_bytes)?;
        if self.geometry_key == Some(key)
            && self.index_count == u32::try_from(indices.len()).unwrap_or(u32::MAX)
        {
            return Ok(());
        }
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        for position in positions {
            min[0] = min[0].min(position[0]);
            min[1] = min[1].min(position[2]);
            max[0] = max[0].max(position[0]);
            max[1] = max[1].max(position[2]);
        }
        let span = [(max[0] - min[0]).max(f32::EPSILON), (max[1] - min[1]).max(f32::EPSILON)];
        let vertices: Vec<WaterVertex> = positions
            .iter()
            .map(|position| WaterVertex {
                position: *position,
                uv: [
                    (position[0] - min[0]) / span[0],
                    (position[2] - min[1]) / span[1],
                ],
            })
            .collect();
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::Geometry, bytes)
            .map_err(|error| format!("HMC water geometry admission failed: {error}"))?;
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qualia-water-vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("qualia-water-indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.vertex_buf = Some(vertex_buf);
        self.index_buf = Some(index_buf);
        self.index_count = u32::try_from(indices.len()).map_err(|_| "water index count overflow")?;
        self.geometry_key = Some(key);
        self.geometry_bytes = bytes;
        self.geometry_generation = self.geometry_generation.wrapping_add(1);
        self._geometry_reservation = Some(reservation);
        Ok(())
    }

    fn clear_geometry(&mut self) {
        self.vertex_buf = None;
        self.index_buf = None;
        self.index_count = 0;
        self.geometry_key = None;
        self.geometry_bytes = 0;
        self.geometry_generation = self.geometry_generation.wrapping_add(1);
        self._geometry_reservation = None;
    }

    pub(super) fn record<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        mesh_frame_bind: &'a wgpu::BindGroup,
        mesh_model_bind: &'a wgpu::BindGroup,
        hdr: bool,
    ) {
        let (Some(vertex_buf), Some(index_buf)) = (self.vertex_buf.as_ref(), self.index_buf.as_ref()) else {
            return;
        };
        pass.set_pipeline(if hdr {
            self.pipeline_hdr.as_ref().unwrap_or(&self.pipeline)
        } else {
            &self.pipeline
        });
        pass.set_bind_group(0, mesh_frame_bind, &[]);
        pass.set_bind_group(1, mesh_model_bind, &[]);
        pass.set_bind_group(2, &self.uniform_bind, &[]);
        pass.set_vertex_buffer(0, vertex_buf.slice(..));
        pass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

impl PortalGpu {
    pub fn upload_water_geometry(
        &mut self,
        positions: &[[f32; 3]],
        indices: &[u32],
    ) -> Result<(), String> {
        self.water.replace_geometry(&self.device, positions, indices)
    }

    /// Upload persistent water geometry with an explicit admission ceiling.
    /// The previous geometry remains live if validation or budget admission fails.
    pub fn upload_water_geometry_with_budget(
        &mut self,
        positions: &[[f32; 3]],
        indices: &[u32],
        max_bytes: u64,
    ) -> Result<(), String> {
        self.water.replace_geometry_with_budget(&self.device, positions, indices, max_bytes)
    }

    pub fn clear_water_geometry(&mut self) {
        let _ = self.water.replace_geometry(&self.device, &[], &[]);
    }

    pub fn set_water_surface(
        &mut self,
        colour: [f32; 3],
        alpha: f32,
        wave_amplitude: f32,
        wave_frequency: f32,
    ) {
        self.water.uniform.colour_alpha = [
            if colour[0].is_finite() { colour[0].max(0.0) } else { 0.0 },
            if colour[1].is_finite() { colour[1].max(0.0) } else { 0.0 },
            if colour[2].is_finite() { colour[2].max(0.0) } else { 0.0 },
            if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.0 },
        ];
        self.water.uniform.wave[1] = if wave_amplitude.is_finite() {
            wave_amplitude.max(0.0)
        } else {
            0.0
        };
        self.water.uniform.wave[2] = if wave_frequency.is_finite() {
            wave_frequency.max(0.0)
        } else {
            0.0
        };
    }

    /// Select bounded water shading without allocating or rebuilding resources.
    pub fn set_water_quality(&mut self, tier: u32) {
        let (tier, reflection, foam, shore) = match tier {
            WATER_QUALITY_CINEMATIC => (WATER_QUALITY_CINEMATIC, 0.55, 0.85, 1.0),
            WATER_QUALITY_BALANCED => (WATER_QUALITY_BALANCED, 0.35, 0.60, 0.70),
            _ => (WATER_QUALITY_LOWER, 0.0, 0.25, 0.35),
        };
        self.water.uniform.quality = [tier as f32, reflection, foam, shore];
    }

    #[allow(dead_code)]
    pub fn water_geometry_state(&self) -> (u64, u64, u32) {
        (
            self.water.geometry_generation,
            self.water.geometry_bytes,
            self.water.index_count / 3,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{geometry_admission, geometry_key, WATER_QUALITY_BALANCED, WATER_QUALITY_LOWER};

    #[test]
    fn finite_geometry_is_admitted_and_keyed_deterministically() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let indices = [0, 1, 2];
        let (bytes, key) = geometry_admission(&positions, &indices, 1024).unwrap();
        assert_eq!(bytes, 3 * std::mem::size_of::<super::WaterVertex>() as u64 + 12);
        assert_eq!(key, geometry_key(&positions, &indices));
    }

    #[test]
    fn invalid_geometry_fails_before_gpu_admission() {
        let positions = [[0.0, f32::NAN, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        assert!(geometry_admission(&positions, &[0, 1, 2], 1024).is_err());
        assert!(geometry_admission(&[[0.0, 0.0, 0.0]], &[0, 1, 0], 1024).is_err());
    }

    #[test]
    fn quality_tier_values_remain_explicit_lower_and_balanced() {
        assert_eq!(WATER_QUALITY_LOWER, 0);
        assert_eq!(WATER_QUALITY_BALANCED, 1);
    }
}
