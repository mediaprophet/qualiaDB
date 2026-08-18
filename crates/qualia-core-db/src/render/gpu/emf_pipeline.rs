//! EMF 5D volumetric visualizer pipeline (plan §7.3 W4).
//!
//! Extends [`PortalGpu`] with a render pipeline that draws 2D slices of the
//! 4D EMF field grid (x×y×z×t) with 10D manifold tags mapped to color.
//!
//! The field data is uploaded as a storage buffer of `EmfFieldCell` structs
//! (48 bytes each: amplitude + phase + frequency + 10D manifold coordinate).
//! A full-screen quad fragment shader samples the grid bilinearly and maps:
//! - amplitude → brightness (HDR)
//! - phase → hue (via σ → CIE XYZ → linear sRGB)
//! - manifold.scale → saturation
//! - manifold.manifold_curvature → rim glow

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Per-cell field data uploaded to the GPU storage buffer.
///
/// Layout matches the WGSL `EmfFieldCell` struct (48 bytes = 3 f32 + 10 f32 + 1 pad).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct EmfFieldCell {
    pub amplitude: f32,
    pub phase: f32,
    pub frequency: f32,
    // 10D manifold coordinate.
    pub scale: f32,
    pub attention_depth: f32,
    pub epistemic_weight: f32,
    pub topological_spin: f32,
    pub temporal_decay: f32,
    pub entropy_bias: f32,
    pub spatial_phase: f32,
    pub recurrence_frequency: f32,
    pub density_threshold: f32,
    pub manifold_curvature: f32,
}

const _: [(); 52] = [(); std::mem::size_of::<EmfFieldCell>()];

/// Slice parameter uniform (64 bytes, std140-aligned).
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EmfSliceUniform {
    pub nx: u32,
    pub ny: u32,
    pub nz: u32,
    pub nt: u32,
    pub slice_z: u32,
    pub slice_t: u32,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub z_min: f32,
    pub z_max: f32,
    pub amplitude_scale: f32,
    pub phase_offset: f32,
    pub manifold_gain: f32,
    pub _pad: f32,
}

const _: [(); 64] = [(); std::mem::size_of::<EmfSliceUniform>()];

/// Per-portal EMF visualizer state.
pub(crate) struct EmfState {
    /// Uploaded field cells (storage buffer).
    field_buf: Option<wgpu::Buffer>,
    /// Number of cells in the field buffer.
    cell_count: u32,
    /// Grid dimensions.
    nx: u32,
    ny: u32,
    nz: u32,
    nt: u32,
    /// Slice parameter uniform buffer.
    param_buf: wgpu::Buffer,
    /// Params bind group (group 0). Created once; references param_buf.
    params_bind_group: wgpu::BindGroup,
    /// Field bind group (group 1). Rebuilt when field buffer changes.
    field_bind_group: Option<wgpu::BindGroup>,
    /// Render pipeline (full-screen quad, EMF fragment shader).
    pipeline: wgpu::RenderPipeline,
}

impl EmfState {
    /// Construct the EMF pipeline state on the given device.
    pub(crate) fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("emf-volumetric-shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::EMF_VOLUMETRIC_WGSL.into(),
            ),
        });

        let param_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("emf-slice-params"),
            size: std::mem::size_of::<EmfSliceUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Two bind group layouts: group 0 = params (uniform), group 1 = field (storage).
        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("emf-params-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZero::new(std::mem::size_of::<EmfSliceUniform>() as u64),
                },
                count: None,
            }],
        });

        let field_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("emf-field-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZero::new(std::mem::size_of::<EmfFieldCell>() as u64),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("emf-pipeline-layout"),
            bind_group_layouts: &[Some(&params_layout), Some(&field_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("emf-pipeline"),
            layout: Some(&pipeline_layout),
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
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        // Create the params bind group (group 0) once — it references the
        // param_buf which persists for the lifetime of the EmfState.
        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("emf-params-bind"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: param_buf.as_entire_binding(),
            }],
        });

        Self {
            field_buf: None,
            cell_count: 0,
            nx: 0,
            ny: 0,
            nz: 0,
            nt: 0,
            param_buf,
            params_bind_group,
            field_bind_group: None,
            pipeline,
        }
    }

    /// Whether a field has been uploaded.
    pub fn has_field(&self) -> bool {
        self.field_buf.is_some()
    }

    /// Grid dimensions of the uploaded field.
    pub fn grid_dims(&self) -> (u32, u32, u32, u32) {
        (self.nx, self.ny, self.nz, self.nt)
    }
}

impl super::PortalGpu {
    /// Upload an EMF field grid to the GPU for volumetric visualization.
    ///
    /// `cells` is the flat field array indexed as [t][z][y][x], with
    /// `nx*ny*nz*nt` elements. `bounds` is [x_min, x_max, y_min, y_max, z_min, z_max].
    pub fn emf_upload_field(
        &mut self,
        cells: &[EmfFieldCell],
        nx: u32,
        ny: u32,
        nz: u32,
        nt: u32,
        bounds: [f32; 6],
    ) -> Result<(), String> {
        let expected = (nx as u64) * (ny as u64) * (nz as u64) * (nt as u64);
        if cells.len() as u64 != expected {
            return Err(format!(
                "emf_upload_field: expected {expected} cells for {nx}×{ny}×{nz}×{nt}, got {}",
                cells.len()
            ));
        }

        let field_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("emf-field-storage"),
            contents: bytemuck::cast_slice(cells),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        self.emf.field_buf = Some(field_buf);
        self.emf.cell_count = cells.len() as u32;
        self.emf.nx = nx;
        self.emf.ny = ny;
        self.emf.nz = nz;
        self.emf.nt = nt;

        // Update the slice parameter uniform with default slice (z=0, t=0).
        self.emf_update_slice_params(0, 0, bounds, 1.0, 0.0, 1.0);

        // Rebuild the field bind group (group 1) with the new field buffer.
        let field_buf = self.emf.field_buf.as_ref().unwrap();
        self.emf.field_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("emf-field-bind"),
            layout: &self.emf.pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: field_buf.as_entire_binding(),
            }],
        }));

        Ok(())
    }

    /// Update the slice parameters (which z/t slice to render, display controls).
    pub fn emf_update_slice_params(
        &mut self,
        slice_z: u32,
        slice_t: u32,
        bounds: [f32; 6],
        amplitude_scale: f32,
        phase_offset: f32,
        manifold_gain: f32,
    ) {
        let params = EmfSliceUniform {
            nx: self.emf.nx,
            ny: self.emf.ny,
            nz: self.emf.nz,
            nt: self.emf.nt,
            slice_z: slice_z.min(self.emf.nz.saturating_sub(1)),
            slice_t: slice_t.min(self.emf.nt.saturating_sub(1)),
            x_min: bounds[0],
            x_max: bounds[1],
            y_min: bounds[2],
            y_max: bounds[3],
            z_min: bounds[4],
            z_max: bounds[5],
            amplitude_scale,
            phase_offset,
            manifold_gain,
            _pad: 0.0,
        };
        self.queue.write_buffer(&self.emf.param_buf, 0, bytemuck::bytes_of(&params));
    }

    /// Render the current EMF slice to the portal's color target.
    ///
    /// Draws a full-screen quad with the EMF fragment shader. The caller is
    /// responsible for beginning the render pass and providing a color
    /// attachment view.
    pub fn emf_render_slice(&self, render_pass: &mut wgpu::RenderPass<'_>) -> Result<(), String> {
        let Some(field_bind) = &self.emf.field_bind_group else {
            return Err("emf_render_slice: no field uploaded".into());
        };
        render_pass.set_pipeline(&self.emf.pipeline);
        render_pass.set_bind_group(0, &self.emf.params_bind_group, &[]);
        render_pass.set_bind_group(1, field_bind, &[]);
        render_pass.draw(0..6, 0..1);
        Ok(())
    }

    /// Whether an EMF field is currently uploaded.
    pub fn emf_has_field(&self) -> bool {
        self.emf.has_field()
    }

    /// EMF grid dimensions (nx, ny, nz, nt).
    pub fn emf_grid_dims(&self) -> (u32, u32, u32, u32) {
        self.emf.grid_dims()
    }

    /// Render the current EMF slice to the offscreen target and read back the
    /// RGBA8 pixels. This is the combined render+readback path used by the
    /// `Render.emf_render_slice` invoke handler.
    pub fn emf_render_slice_to_rgba8(&mut self) -> Result<(u32, u32, Vec<u8>), String> {
        if !self.emf.has_field() {
            return Err("no EMF field uploaded".into());
        }
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("emf-slice-encoder"),
        });
        {
            let view = self
                .offscreen_texture
                .as_ref()
                .ok_or("no offscreen target")?
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("emf-slice-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.emf_render_slice(&mut pass)?;
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        let (w, h) = self.surface_size();
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        self.read_rgba8_into(&mut pixels).map_err(|e| e.to_string())?;
        Ok((w, h, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::PortalGpu;

    #[test]
    fn emf_field_cell_size() {
        // 3 f32 (amp/phase/freq) + 10 f32 (manifold) = 13 f32 = 52 bytes
        assert_eq!(std::mem::size_of::<EmfFieldCell>(), 52);
    }

    #[test]
    fn emf_slice_uniform_size() {
        assert_eq!(std::mem::size_of::<EmfSliceUniform>(), 64);
    }

    #[test]
    fn emf_slice_uniform_alignment() {
        assert_eq!(std::mem::align_of::<EmfSliceUniform>(), 16);
    }

    /// End-to-end: create a PortalGpu, upload a small field, render a slice,
    /// and read back pixels. Skips gracefully when no GPU adapter exists.
    #[test]
    fn c_emf_upload_and_render_slice() {
        if crate::gpu_context::try_shared_gpu().is_none() {
            eprintln!("[emf_test] no GPU adapter — skipping");
            return;
        }
        let mut portal = PortalGpu::new_offscreen(32, 32, 256).expect("offscreen");
        // 4×4×1×1 field = 16 cells.
        let nx = 4u32;
        let ny = 4u32;
        let nz = 1u32;
        let nt = 1u32;
        let cells: Vec<EmfFieldCell> = (0..16)
            .map(|i| EmfFieldCell {
                amplitude: (i as f32) / 16.0,
                phase: (i as f32) * 0.3927, // π/8 per cell
                frequency: 1.0e9,
                scale: 0.5,
                attention_depth: 0.3,
                epistemic_weight: 0.7,
                topological_spin: 0.1,
                temporal_decay: 0.0,
                entropy_bias: 0.2,
                spatial_phase: (i as f32) * 0.1,
                recurrence_frequency: 0.5,
                density_threshold: 0.4,
                manifold_curvature: 0.05,
            })
            .collect();
        let bounds = [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        portal
            .emf_upload_field(&cells, nx, ny, nz, nt, bounds)
            .expect("upload");
        assert!(portal.emf_has_field());
        assert_eq!(portal.emf_grid_dims(), (4, 4, 1, 1));

        // Render a slice and read back pixels via the public method.
        let (w, h, pixels) = portal.emf_render_slice_to_rgba8().expect("render+readback");
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        let non_black = pixels.chunks(4).filter(|px| px[0] > 0 || px[1] > 0 || px[2] > 0).count();
        assert!(
            non_black > 0,
            "EMF slice should produce non-black pixels, got {non_black}"
        );
    }

    #[test]
    fn c_emf_render_without_field_errors() {
        if crate::gpu_context::try_shared_gpu().is_none() {
            eprintln!("[emf_test] no GPU adapter — skipping");
            return;
        }
        let mut portal = PortalGpu::new_offscreen(16, 16, 64).expect("offscreen");
        let result = portal.emf_render_slice_to_rgba8();
        assert!(result.is_err(), "render without field should error");
    }
}
