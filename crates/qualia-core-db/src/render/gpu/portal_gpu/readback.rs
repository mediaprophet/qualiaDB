//! Offscreen RGBA8 readback and particle count query implementation for PortalGpu.

use super::super::PortalGpu;

impl PortalGpu {
    /// Number of bytes required by [`Self::read_rgba8_into`].
    pub fn required_rgba8_bytes(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    /// Read the most recently rendered native offscreen frame into tightly packed RGBA8 bytes.
    ///
    /// This is deliberately caller-buffered: no `Vec` is created in the renderer. Browser surface
    /// instances return an error because their swapchain images are presented, not retained.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_rgba8_into(&self, out: &mut [u8]) -> Result<usize, String> {
        let need = self.required_rgba8_bytes();
        if out.len() < need {
            return Err(format!(
                "RGBA8 output buffer too small: need {need}, got {}",
                out.len()
            ));
        }
        let texture = self
            .offscreen_texture
            .as_ref()
            .ok_or_else(|| "RGBA8 readback requires an offscreen renderer".to_string())?;
        let staging = self
            .readback_buf
            .as_ref()
            .ok_or_else(|| "offscreen readback buffer is unavailable".to_string())?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("qualia-render-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.readback_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|e| format!("RGBA8 readback callback failed: {e}"))?
            .map_err(|e| format!("RGBA8 buffer map failed: {e}"))?;

        let mapped = slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let tight_row = self.width as usize * 4;
        let padded_row = self.readback_bytes_per_row as usize;
        for row in 0..self.height as usize {
            let src = &mapped[row * padded_row..row * padded_row + tight_row];
            let dst = &mut out[row * tight_row..(row + 1) * tight_row];
            dst.copy_from_slice(src);
        }
        drop(mapped);
        staging.unmap();
        Ok(need)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn read_rgba8_into(&self, _out: &mut [u8]) -> Result<usize, String> {
        Err("Synchronous RGBA8 readback is not supported on wasm32 targets".to_string())
    }

    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }
}
