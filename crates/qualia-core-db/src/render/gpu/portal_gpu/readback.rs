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

        etalon_render::readback::read_rgba8_into(
            &self.device,
            &self.queue,
            texture,
            staging,
            self.width,
            self.height,
            self.readback_bytes_per_row,
            out,
        )
    }

    #[cfg(target_arch = "wasm32")]
    /// Reads `rgba8-into`.
    pub fn read_rgba8_into(&self, _out: &mut [u8]) -> Result<usize, String> {
        Err("Synchronous RGBA8 readback is not supported on wasm32 targets".to_string())
    }

    /// `PortalGpu::particle_count`.
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }
}
