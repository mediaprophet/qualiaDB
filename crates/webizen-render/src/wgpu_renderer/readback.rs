//! Bounded offscreen pixel readback, PNG encoding, and runtime receipt generation.

use super::renderer::{reserve_graphics_bytes, GraphicsClass, RenderTarget, WgpuRenderer};
use super::{
    pixel_receipt_from_rgba8, readback_layout, PixelReadback, PixelReceipt, PixelReceiptStatus,
    WebGpuRuntimeReceipt,
};

impl<'a> WgpuRenderer<'a> {
    /// Attempt a bounded pixel readback and return both bytes and evidence.
    pub fn read_pixels_with_receipt(&self) -> PixelReadback {
        let adapter_backed = self.capabilities.device_available;
        let (texture, width, height) = match &self.target {
            RenderTarget::Offscreen {
                texture,
                width,
                height,
                ..
            } => (texture, *width, *height),
            RenderTarget::Surface { .. } => {
                return PixelReadback {
                    pixels: None,
                    receipt: PixelReceipt::refusal(
                        PixelReceiptStatus::NotOffscreen,
                        self.requested_extent.0,
                        self.requested_extent.1,
                        adapter_backed,
                    ),
                }
            }
        };

        let (padded_bytes_per_row, unpadded_bytes_per_row, readback_bytes, output_bytes) =
            match readback_layout(width, height) {
                Some(layout) => layout,
                None => {
                    return PixelReadback {
                        pixels: None,
                        receipt: PixelReceipt::refusal(
                            PixelReceiptStatus::Refused,
                            width,
                            height,
                            adapter_backed,
                        ),
                    }
                }
            };
        let _staging_reservation = match reserve_graphics_bytes(
            GraphicsClass::UploadStaging,
            readback_bytes,
            "readback",
        ) {
            Ok(reservation) => reservation,
            Err(error) => {
                log::warn!("webizen-render pixel readback refused: {error}");
                return PixelReadback {
                    pixels: None,
                    receipt: PixelReceipt::refusal(
                        PixelReceiptStatus::Refused,
                        width,
                        height,
                        adapter_backed,
                    ),
                };
            }
        };

        let readback = self.device.create_buffer(&etalon::BufferDescriptor {
            label: Some("webizen-readback-buffer"),
            size: readback_bytes,
            usage: etalon::BufferUsages::MAP_READ | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&etalon::CommandEncoderDescriptor {
                label: Some("webizen-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            etalon::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: etalon::Origin3d::ZERO,
                aspect: etalon::TextureAspect::All,
            },
            etalon::TexelCopyBufferInfo {
                buffer: &readback,
                layout: etalon::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            etalon::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(etalon::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        if let Err(error) = self.device.poll(etalon::PollType::wait_indefinitely()) {
            log::warn!("webizen-render pixel readback poll failed: {error:?}");
            return PixelReadback {
                pixels: None,
                receipt: PixelReceipt::refusal(
                    PixelReceiptStatus::MapFailed,
                    width,
                    height,
                    adapter_backed,
                ),
            };
        }
        if rx.recv().ok().and_then(|result| result.ok()).is_none() {
            log::warn!("webizen-render pixel readback mapping failed");
            return PixelReadback {
                pixels: None,
                receipt: PixelReceipt::refusal(
                    PixelReceiptStatus::MapFailed,
                    width,
                    height,
                    adapter_backed,
                ),
            };
        }

        let mapped = match slice.get_mapped_range() {
            Ok(mapped) => mapped,
            Err(error) => {
                log::warn!("webizen-render pixel readback map range failed: {error}");
                readback.unmap();
                return PixelReadback {
                    pixels: None,
                    receipt: PixelReceipt::refusal(
                        PixelReceiptStatus::MapFailed,
                        width,
                        height,
                        adapter_backed,
                    ),
                };
            }
        };
        let mut out = Vec::with_capacity(output_bytes);
        for row in 0..height {
            let start = row as usize * padded_bytes_per_row as usize;
            let end = start + unpadded_bytes_per_row as usize;
            out.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        readback.unmap();
        let mut receipt = pixel_receipt_from_rgba8(width, height, &out);
        receipt.adapter_backed = adapter_backed;
        PixelReadback {
            pixels: Some(out),
            receipt,
        }
    }

    /// Read the offscreen target to a tightly-packed RGBA8 buffer
    /// (`width * height * 4` bytes, row-major). Returns `None` for refusal.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        self.read_pixels_with_receipt().pixels
    }

    /// Return a complete bounded runtime receipt after attempting readback.
    pub fn runtime_receipt(&self) -> WebGpuRuntimeReceipt {
        let pixels = self.read_pixels_with_receipt();
        WebGpuRuntimeReceipt {
            capabilities: self.capabilities.clone(),
            requested_extent: self.requested_extent,
            effective_extent: self.offscreen_extent(),
            pixels: pixels.receipt,
        }
    }

    /// Read the offscreen target back as PNG-encoded bytes, ready to hand to a
    /// webview as a data-URI or via a custom protocol. Returns `None` for surface
    /// targets or if there is no GPU frame to read. Native-only (the wasm build
    /// renders to a `<canvas>` and never reads back).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_png(&self) -> Option<Vec<u8>> {
        use image::ImageEncoder;

        let (width, height) = match &self.target {
            RenderTarget::Offscreen { width, height, .. } => (*width, *height),
            RenderTarget::Surface { .. } => return None,
        };

        let rgba = self.read_pixels()?;
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(png_bytes)
    }

    /// Read the offscreen frame back as a `data:image/png;base64,...` URI — drop
    /// straight into a webview `<img src>`. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_data_uri(&self) -> Option<String> {
        use base64::Engine;
        let png = self.read_png()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png);
        Some(format!("data:image/png;base64,{b64}"))
    }
}
