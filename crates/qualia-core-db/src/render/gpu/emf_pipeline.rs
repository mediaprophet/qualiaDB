//! EMF field-family volumetric visualizer pipeline (plan §7.3 W4 / W7).
//!
//! Extends [`PortalGpu`] with a render pipeline that draws 2D slices of the
//! 4D EMF field grid (x×y×z×t) with the full 10D manifold record and 3 Stokes
//! polarization parameters preserved.
//! This renderer is one field-family view over the shared scientific manifold;
//! it does not define or replace astrophysics, wave, diffusion, or other fields.
//!
//! The field data is uploaded as a storage buffer of `EmfFieldCell` structs
//! (64 bytes each: amplitude + phase + frequency + 10D manifold coordinate + 3 Stokes Q/U/V).
//! A full-screen quad fragment shader samples the grid bilinearly and maps:
//! - amplitude → brightness (HDR)
//! - phase → hue as a derived display projection (via Full-Spectrum σ → linear sRGB)
//! - manifold.scale → display saturation
//! - manifold.manifold_curvature + Stokes polarization → display rim glow & highlight

#[path = "emf_spectrum.rs"]
mod emf_spectrum;

use etalon::util::DeviceExt;
pub use emf_spectrum::{
    emf_cells_from_loaded, emf_field_cell_from_optical, emf_field_cell_from_record64,
    evaluate_display_slice_rgba8, full_spectrum_sigma_to_linear_rgb,
};
pub use etalon_render::emf::{
    checked_field_bytes, checked_grid_cell_count, EmfFieldCell, EmfPipeline, EmfPipelineDesc,
    EmfSliceUniform, EMF_VOLUMETRIC_WGSL,
};
/// Fixed grayscale display derivative; exact f64 observations stay in the retained source.
pub fn emf_field_cell_from_display(cell: &crate::render::portal_emf_state::DisplayEmfCell) -> EmfFieldCell {
    let brightness = if cell.valid { -(-cell.magnitude_v_per_m).exp_m1() as f32 } else { 0.0 };
    EmfFieldCell { amplitude: brightness, epistemic_weight: if cell.valid { 1.0 } else { 0.0 },
        phase: 0.0, frequency: 0.0, scale: 0.0, attention_depth: 0.0,
        topological_spin: 0.0, temporal_decay: 0.0, entropy_bias: 0.0,
        spatial_phase: 0.0, recurrence_frequency: 0.0, density_threshold: 0.0,
        manifold_curvature: 0.0, stokes_q: 0.0, stokes_u: 0.0, stokes_v: 0.0 }
}

/// Per-portal EMF visualizer state.
pub(crate) struct EmfState {
    /// Uploaded field cells (storage buffer).
    pub field_buf: Option<etalon::Buffer>,
    pub _field_reservation: Option<crate::gpu_context::VramReservation<'static>>,
    /// Number of cells in the field buffer.
    pub cell_count: u32,
    /// Grid dimensions.
    pub nx: u32,
    pub ny: u32,
    pub nz: u32,
    pub nt: u32,
    /// Field bind group (group 1). Rebuilt when field buffer changes.
    pub field_bind_group: Option<etalon::BindGroup>,
    /// Inner decoupled EMF pipeline from etalon-render.
    pub inner: etalon_render::emf::EmfPipeline,
    pub _param_buffer_reservation: crate::gpu_context::VramReservation<'static>,
}

impl std::ops::Deref for EmfState {
    type Target = etalon_render::emf::EmfPipeline;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for EmfState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl EmfState {
    /// Construct the EMF pipeline state on the given device.
    pub(crate) fn new(
        device: &etalon::Device,
        color_format: etalon::TextureFormat,
    ) -> Result<Self, String> {
        let param_bytes = std::mem::size_of::<EmfSliceUniform>() as u64;
        let param_buffer_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FrameTarget,
                param_bytes,
            )
            .map_err(|e| format!("EMF parameter buffer reservation failed: {e}"))?;

        let inner = etalon_render::emf::EmfPipeline::new(device, color_format);

        Ok(Self {
            field_buf: None,
            _field_reservation: None,
            cell_count: 0,
            nx: 0,
            ny: 0,
            nz: 0,
            nt: 0,
            field_bind_group: None,
            inner,
            _param_buffer_reservation: param_buffer_reservation,
        })
    }

    /// Whether a field has been uploaded.
    pub fn has_field(&self) -> bool {
        self.field_buf.is_some()
    }

    /// Grid dimensions of the uploaded field.
    pub fn grid_dims(&self) -> (u32, u32, u32, u32) {
        (self.nx, self.ny, self.nz, self.nt)
    }

    /// Access the underlying Etalon EMF pipeline.
    pub fn inner(&self) -> &etalon_render::emf::EmfPipeline {
        &self.inner
    }

    /// Mutable access to the underlying Etalon EMF pipeline.
    pub fn inner_mut(&mut self) -> &mut etalon_render::emf::EmfPipeline {
        &mut self.inner
    }
}

impl super::PortalGpu {
    /// Upload an EMF field grid to the GPU for volumetric visualization.
    ///
    /// `cells` is the flat field array indexed as \[t\]\[z\]\[y\]\[x\], with
    /// `nx*ny*nz*nt` elements. `bounds` is \[x_min, x_max, y_min, y_max, z_min, z_max\].
    pub fn emf_upload_field(
        &mut self,
        cells: &[EmfFieldCell],
        nx: u32,
        ny: u32,
        nz: u32,
        nt: u32,
        bounds: [f32; 6],
    ) -> Result<(), String> {
        let expected = checked_grid_cell_count(nx, ny, nz, nt)
            .ok_or_else(|| "emf_upload_field: grid cell count overflow".to_string())?;
        if cells.len() as u64 != expected {
            return Err(format!(
                "emf_upload_field: expected {expected} cells for {nx}×{ny}×{nz}×{nt}, got {}",
                cells.len()
            ));
        }
        if expected == 0 {
            return Err("emf_upload_field: all grid dimensions must be non-zero".into());
        }
        if expected > u64::from(u32::MAX) {
            return Err("emf_upload_field: cell count exceeds the renderer ABI".into());
        }
        let field_bytes = checked_field_bytes(expected)
            .ok_or_else(|| "emf_upload_field: field byte size overflow".to_string())?;
        let max_binding_bytes =
            super::portal_gpu::effective_readonly_storage_binding_size(&self.device);
        let max_buffer_bytes = self.device.limits().max_buffer_size;
        if field_bytes > max_binding_bytes || field_bytes > max_buffer_bytes {
            return Err(format!(
                "emf_upload_field: {field_bytes} bytes exceed this device's storage-buffer limit ({max_binding_bytes})"
            ));
        }
        let field_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FieldResidency,
                field_bytes,
            )
            .map_err(|e| format!("emf_upload_field: field residency refused: {e}"))?;

        let field_buf = self
            .device
            .create_buffer_init(&etalon::util::BufferInitDescriptor {
                label: Some("emf-field-storage"),
                contents: bytemuck::cast_slice(cells),
                usage: etalon::BufferUsages::STORAGE | etalon::BufferUsages::COPY_DST,
            });

        // Build all replacement handles before swapping out the old field. If
        // admission or allocation fails, the currently displayed field remains valid.
        let field_bind_group = self.device.create_bind_group(&etalon::BindGroupDescriptor {
            label: Some("emf-field-bind"),
            layout: &self.emf.inner().field_layout,
            entries: &[etalon::BindGroupEntry {
                binding: 0,
                resource: field_buf.as_entire_binding(),
            }],
        });
        self.emf.field_bind_group = Some(field_bind_group);
        self.emf.field_buf = Some(field_buf);
        self.emf._field_reservation = Some(field_reservation);
        self.emf.cell_count = cells.len() as u32;
        self.emf.nx = nx;
        self.emf.ny = ny;
        self.emf.nz = nz;
        self.emf.nt = nt;

        // Update the slice parameter uniform with default slice (z=0, t=0).
        self.emf_update_slice_params(0, 0, bounds, 1.0, 0.0, 1.0);

        Ok(())
    }

    /// Upload an admitted `LoadedEmf` derivative, enabling full-spectrum projection mode `0`
    /// when spectral/Stokes cells are present (falling back to grayscale mode `1` for static grids).
    pub fn emf_upload_loaded(
        &mut self,
        loaded: &crate::render::portal_emf_state::LoadedEmf,
    ) -> Result<(), String> {
        let cells = emf_cells_from_loaded(loaded)?;
        let [nx, ny, nz, nt] = loaded.dimensions();
        let bounds = loaded.local_bounds();
        self.emf_upload_field(&cells, nx, ny, nz, nt, bounds)?;
        let spectral = loaded.optical_cells().iter().any(|o| {
            o.valid && (o.spectral_sigma > 0.0 || o.carrier_hz > 0.0 || o.phase_rad != 0.0 || o.stokes_q != 0.0 || o.stokes_u != 0.0 || o.stokes_v != 0.0)
        });
        let mode = if spectral { 0 } else { 1 };
        self.emf_update_slice_params_with_mode(0, 0, bounds, 1.0, 0.0, 1.0, mode);
        Ok(())
    }

    /// Parse and upload a verified `Q42EMF64` container directly for volumetric display.
    pub fn emf_upload_emf64(&mut self, bytes: &[u8]) -> Result<(), String> {
        let loaded = crate::render::portal_emf_state::decode_emf64_display(bytes)?;
        self.emf_upload_loaded(&loaded)
    }

    /// Update the slice parameters (which z/t slice to render, display controls).
    /// Preserves `projection_mode: 1` grayscale compatibility; use `emf_update_slice_params_with_mode`
    /// for full volumetric spectral + Stokes mode `0`.
    pub fn emf_update_slice_params(
        &mut self,
        slice_z: u32,
        slice_t: u32,
        bounds: [f32; 6],
        amplitude_scale: f32,
        phase_offset: f32,
        manifold_gain: f32,
    ) {
        self.emf_update_slice_params_with_mode(
            slice_z,
            slice_t,
            bounds,
            amplitude_scale,
            phase_offset,
            manifold_gain,
            1,
        );
    }

    /// Update slice parameters with explicit `EmfSliceUniform::projection_mode` (`0` volumetric spectral, `1` grayscale).
    #[allow(clippy::too_many_arguments)]
    pub fn emf_update_slice_params_with_mode(
        &mut self,
        slice_z: u32,
        slice_t: u32,
        bounds: [f32; 6],
        amplitude_scale: f32,
        phase_offset: f32,
        manifold_gain: f32,
        projection_mode: u32,
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
            projection_mode,
        };
        self.emf.inner_mut().update_params(&self.queue, &params);
    }

    /// Render the current EMF slice to the portal's color target.
    ///
    /// Draws a full-screen quad with the EMF fragment shader. The caller is
    /// responsible for beginning the render pass and providing a color
    /// attachment view.
    pub fn emf_render_slice(&self, render_pass: &mut etalon::RenderPass<'_>) -> Result<(), String> {
        let Some(field_bind) = &self.emf.field_bind_group else {
            return Err("emf_render_slice: no field uploaded".into());
        };
        render_pass.set_pipeline(&self.emf.inner().pipeline);
        render_pass.set_bind_group(0, &self.emf.inner().params_bind_group, &[]);
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
        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("emf-slice-encoder"),
            });
        {
            let view = self
                .offscreen_texture
                .as_ref()
                .ok_or("no offscreen target")?
                .create_view(&etalon::TextureViewDescriptor::default());
            let mut pass = encoder.begin_render_pass(&etalon::RenderPassDescriptor {
                label: Some("emf-slice-pass"),
                color_attachments: &[Some(etalon::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: etalon::Operations {
                        load: etalon::LoadOp::Clear(etalon::Color::BLACK),
                        store: etalon::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.emf_render_slice(&mut pass)?;
        }
        let submission = self.queue.submit(std::iter::once(encoder.finish()));
        #[cfg(not(target_arch = "wasm32"))]
        if submission.is_none() {
            return Err("EMF rendering: native queue rejected the command submission".to_string());
        }
        #[cfg(target_arch = "wasm32")]
        let _ = submission;

        let (w, h) = self.surface_size();
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        self.read_rgba8_into(&mut pixels)
            .map_err(|e| e.to_string())?;
        Ok((w, h, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::super::PortalGpu;
    use super::*;


    #[test]
    fn emf_field_cell_size() {
        // 3 f32 (amp/phase/freq) + 10 f32 (manifold) + 3 f32 (Stokes Q/U/V) = 16 f32 = 64 bytes
        assert_eq!(std::mem::size_of::<EmfFieldCell>(), 64);
    }

    #[test]
    fn display_preserves_valid_zero_and_missing_and_saturates_extreme_fields() {
        use crate::render::portal_emf_state::DisplayEmfCell;
        for (magnitude, expected) in [(0.0,0.0),(1.0,1.0-(-1.0f32).exp()),(f64::MAX,1.0),(f64::INFINITY,1.0)] {
            let cell=emf_field_cell_from_display(&DisplayEmfCell { magnitude_v_per_m:magnitude,valid:true });
            assert!((cell.amplitude-expected).abs()<1e-6);
            assert_eq!(cell.epistemic_weight,1.0);
            assert_eq!(cell.frequency,0.0);
        }
        let missing=emf_field_cell_from_display(&DisplayEmfCell { magnitude_v_per_m:0.0,valid:false });
        assert_eq!(missing.epistemic_weight,0.0);
    }

    #[test]
    fn emf_field_cell_size_and_alignment() {
        assert_eq!(std::mem::size_of::<EmfFieldCell>(), 64);
        assert_eq!(std::mem::align_of::<EmfFieldCell>(), 4);
        let migrated = EmfFieldCell::from_v1_13f32([1.0; 13]);
        assert_eq!(migrated.amplitude, 1.0);
        assert_eq!(migrated.stokes_q, 0.0);
        assert_eq!(migrated.stokes_u, 0.0);
        assert_eq!(migrated.stokes_v, 0.0);
    }

    #[test]
    fn emf_grid_and_residency_sizing_is_checked() {
        assert_eq!(checked_grid_cell_count(4, 4, 1, 1), Some(16));
        assert_eq!(
            checked_grid_cell_count(u32::MAX, u32::MAX, u32::MAX, 2),
            None
        );
        assert_eq!(checked_grid_cell_count(0, 4, 1, 1), Some(0));
        assert_eq!(checked_field_bytes(16), Some(1024));
        assert_eq!(checked_field_bytes(u64::MAX), None);
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
                stokes_q: 0.2,
                stokes_u: 0.1,
                stokes_v: 0.05,
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
        let non_black = pixels
            .chunks(4)
            .filter(|px| px[0] > 0 || px[1] > 0 || px[2] > 0)
            .count();
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
