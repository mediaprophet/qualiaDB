//! Bounded browser presentation using the existing Etalon EMF slice pipeline.

use super::PortalGpu;

impl PortalGpu {
    /// Submit the current field slice to the browser surface using the retained
    /// projection mode (`0` volumetric spectral + Stokes when uploaded with optical
    /// data, `1` electric-magnitude grayscale diagnostic for static grids).
    /// This is a draw, not a pixel-readback receipt; the host must observe the
    /// resulting canvas.
    pub fn emf_present_slice(
        &mut self,
        bounds: [f32; 6],
        amplitude_gain: f32,
    ) -> Result<(u32, u32), String> {
        let mode = self.emf_projection_mode();
        self.emf_present_slice_with_mode(bounds, amplitude_gain, mode)
    }

    /// Submit the current field slice to the browser surface with an explicit
    /// `projection_mode` (`0` volumetric spectral + Stokes, `1` grayscale diagnostic).
    pub fn emf_present_slice_with_mode(
        &mut self,
        bounds: [f32; 6],
        amplitude_gain: f32,
        projection_mode: u32,
    ) -> Result<(u32, u32), String> {
        if self.is_device_lost() || !self.emf_has_field() {
            return Err("EMF preview requires a live WebGPU device and uploaded field".into());
        }
        let surface = self
            .surface
            .as_ref()
            .ok_or("EMF browser preview requires a surface")?;
        let frame = match surface.get_current_texture() {
            etalon::CurrentSurfaceTexture::Success(frame)
            | etalon::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            other => return Err(format!("EMF preview surface unavailable: {other:?}")),
        };
        let (width, height) = (frame.texture.width(), frame.texture.height());
        if width == 0 || height == 0 || width > 1024 || height > 1024 {
            return Err("EMF browser preview surface must be 1..=1024 pixels per side".into());
        }
        let (slice_z, slice_t) = self.emf_slice_indices();
        self.emf_update_slice_params_with_mode(
            slice_z,
            slice_t,
            bounds,
            amplitude_gain,
            0.0,
            1.0,
            projection_mode,
        );
        let view = frame
            .texture
            .create_view(&etalon::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("emf-browser-preview"),
            });
        {
            let mut pass = encoder.begin_render_pass(&etalon::RenderPassDescriptor {
                label: Some("emf-browser-slice"),
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
        let _ = self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(frame);
        Ok((width, height))
    }
}
