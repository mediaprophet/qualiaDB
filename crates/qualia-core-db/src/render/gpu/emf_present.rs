//! Bounded browser presentation using the existing Etalon EMF slice pipeline.

use super::PortalGpu;

impl PortalGpu {
    /// Submit the current field slice to the browser surface. This is a draw,
    /// not a pixel-readback receipt; the host must observe the resulting canvas.
    pub(crate) fn emf_present_slice(
        &mut self,
        bounds: [f32; 6],
        amplitude_gain: f32,
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
        self.emf_update_slice_params(0, 0, bounds, amplitude_gain, 0.0, 1.0);
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
