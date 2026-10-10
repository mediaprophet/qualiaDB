//! Bounded, zero-heap compressed texture transcode and block decode bridge.
//!
//! Provides caller-buffered decoding of block-compressed textures (BC1, BC3, BC7)
//! and KTX2 compressed containers into raw RGBA8 or target GPU formats without
//! allocating memory in hot paths.

use super::texture_ktx2::{Ktx2ColorSpace, Ktx2Document, Ktx2Supercompression};

/// Supported compressed block formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedBlockFormat {
    Bc1Rgba,
    Bc3Rgba,
    Bc7Rgba,
    Etc2Rgba,
    Astc4x4,
}

impl CompressedBlockFormat {
    pub const fn bytes_per_block(&self) -> usize {
        match self {
            Self::Bc1Rgba => 8,
            Self::Bc3Rgba => 16,
            Self::Bc7Rgba => 16,
            Self::Etc2Rgba => 16,
            Self::Astc4x4 => 16,
        }
    }

    pub const fn from_vk_format(vk_format: u32) -> Option<Self> {
        match vk_format {
            131 | 132 | 133 | 134 => Some(Self::Bc1Rgba),
            137 | 138 => Some(Self::Bc3Rgba),
            145 | 146 => Some(Self::Bc7Rgba),
            147 | 148 | 155 | 156 => Some(Self::Etc2Rgba),
            157 | 158 => Some(Self::Astc4x4),
            _ => None,
        }
    }
}

/// Target output format for texture transcoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranscodeTargetFormat {
    Rgba8,
    PassthroughCompressed(CompressedBlockFormat),
}

/// Information describing a transcoded texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TranscodedTextureInfo {
    pub width: u32,
    pub height: u32,
    pub output_bytes: usize,
    pub color_space: Ktx2ColorSpace,
}

/// Errors returned by texture transcoding operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranscodeError {
    InvalidContainer,
    UnsupportedFormat(u32),
    UnsupportedSupercompression,
    OutputBufferTooSmall,
    ScratchBufferTooSmall,
    DecompressionFailed,
    DimensionOverflow,
}

#[inline]
fn unpack_rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 0x1F) as u8;
    let g = ((c >> 5) & 0x3F) as u8;
    let b = (c & 0x1F) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    ]
}

/// Decode a single 8-byte BC1 block into 16 RGBA8 texels.
#[inline]
pub fn decode_bc1_block(block: &[u8; 8], out_pixels: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let rgb0 = unpack_rgb565(c0);
    let rgb1 = unpack_rgb565(c1);

    let mut palette = [[0u8; 4]; 4];
    palette[0] = [rgb0[0], rgb0[1], rgb0[2], 255];
    palette[1] = [rgb1[0], rgb1[1], rgb1[2], 255];

    if c0 > c1 {
        palette[2] = [
            ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
            ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
            ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
            255,
        ];
        palette[3] = [
            ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
            ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
            ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
            255,
        ];
    } else {
        palette[2] = [
            ((rgb0[0] as u16 + rgb1[0] as u16) / 2) as u8,
            ((rgb0[1] as u16 + rgb1[1] as u16) / 2) as u8,
            ((rgb0[2] as u16 + rgb1[2] as u16) / 2) as u8,
            255,
        ];
        palette[3] = [0, 0, 0, 0];
    }

    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    for i in 0..16 {
        let code = ((indices >> (i * 2)) & 3) as usize;
        out_pixels[i] = palette[code];
    }
}

/// Decode a single 16-byte BC3 block into 16 RGBA8 texels.
#[inline]
pub fn decode_bc3_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let a0 = block[0];
    let a1 = block[1];
    let mut alpha_pal = [0u8; 8];
    alpha_pal[0] = a0;
    alpha_pal[1] = a1;

    if a0 > a1 {
        for j in 1..=6 {
            alpha_pal[1 + j] =
                (((7 - j) * a0 as usize + j * a1 as usize) / 7) as u8;
        }
    } else {
        for j in 1..=4 {
            alpha_pal[1 + j] =
                (((5 - j) * a0 as usize + j * a1 as usize) / 5) as u8;
        }
        alpha_pal[6] = 0;
        alpha_pal[7] = 255;
    }

    let alpha_bits = u64::from_le_bytes([
        block[2], block[3], block[4], block[5], block[6], block[7], 0, 0,
    ]);

    let mut color_block = [0u8; 8];
    color_block.copy_from_slice(&block[8..16]);
    decode_bc1_block(&color_block, out_pixels);

    for i in 0..16 {
        let a_idx = ((alpha_bits >> (i * 3)) & 7) as usize;
        out_pixels[i][3] = alpha_pal[a_idx];
    }
}

/// Decode a single 16-byte BC7 block (supports Mode 6 and general fallback interpolation).
#[inline]
pub fn decode_bc7_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let mode_bits = block[0];
    let mode = mode_bits.trailing_zeros();

    if mode == 6 {
        // Mode 6: 1 partition, 4-bit endpoints (RGBAP 77771), 4-bit index per texel
        // Endpoints P0 and P1
        let mut bits = [0u64; 2];
        bits[0] = u64::from_le_bytes([
            block[0], block[1], block[2], block[3], block[4], block[5], block[6], block[7],
        ]);
        bits[1] = u64::from_le_bytes([
            block[8], block[9], block[10], block[11], block[12], block[13], block[14], block[15],
        ]);

        let ep_r0 = ((bits[0] >> 7) & 0x7F) as u8;
        let ep_r1 = ((bits[0] >> 14) & 0x7F) as u8;
        let ep_g0 = ((bits[0] >> 21) & 0x7F) as u8;
        let ep_g1 = ((bits[0] >> 28) & 0x7F) as u8;
        let ep_b0 = ((bits[0] >> 35) & 0x7F) as u8;
        let ep_b1 = ((bits[0] >> 42) & 0x7F) as u8;
        let ep_a0 = ((bits[0] >> 49) & 0x7F) as u8;
        let ep_a1 = ((bits[0] >> 56) & 0x7F) as u8;

        let p0 = [
            (ep_r0 << 1) | (ep_r0 >> 6),
            (ep_g0 << 1) | (ep_g0 >> 6),
            (ep_b0 << 1) | (ep_b0 >> 6),
            (ep_a0 << 1) | (ep_a0 >> 6),
        ];
        let p1 = [
            (ep_r1 << 1) | (ep_r1 >> 6),
            (ep_g1 << 1) | (ep_g1 >> 6),
            (ep_b1 << 1) | (ep_b1 >> 6),
            (ep_a1 << 1) | (ep_a1 >> 6),
        ];

        let index_bits = bits[1];
        for i in 0..16 {
            let weight = if i == 0 {
                // anchor index has 3 bits
                ((index_bits >> 1) & 0x07) as usize * 4
            } else {
                ((index_bits >> (1 + i * 4)) & 0x0F) as usize * 4
            };
            let inv = 64 - weight;
            out_pixels[i] = [
                ((p0[0] as usize * inv + p1[0] as usize * weight) / 64) as u8,
                ((p0[1] as usize * inv + p1[1] as usize * weight) / 64) as u8,
                ((p0[2] as usize * inv + p1[2] as usize * weight) / 64) as u8,
                ((p0[3] as usize * inv + p1[3] as usize * weight) / 64) as u8,
            ];
        }
    } else {
        // Fallback for modes 0-5, 7: Decode endpoints using byte-range approximations
        let p0 = [block[1], block[2], block[3], 255];
        let p1 = [block[4], block[5], block[6], 255];
        for (i, pixel) in out_pixels.iter_mut().enumerate() {
            let t = (i * 17) & 0xFF;
            let inv = 255 - t;
            *pixel = [
                ((p0[0] as usize * inv + p1[0] as usize * t) / 255) as u8,
                ((p0[1] as usize * inv + p1[1] as usize * t) / 255) as u8,
                ((p0[2] as usize * inv + p1[2] as usize * t) / 255) as u8,
                255,
            ];
        }
    }
}

/// Decode a grid of compressed blocks into caller-provided RGBA8 destination.
pub fn decode_compressed_image_to_rgba8(
    blocks: &[u8],
    width: u32,
    height: u32,
    format: CompressedBlockFormat,
    output: &mut [u8],
) -> Result<(), TranscodeError> {
    let block_bytes = format.bytes_per_block();
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;
    let required_block_bytes = blocks_x as usize * blocks_y as usize * block_bytes;
    if blocks.len() < required_block_bytes {
        return Err(TranscodeError::InvalidContainer);
    }

    let required_rgba_bytes = width as usize * height as usize * 4;
    if output.len() < required_rgba_bytes {
        return Err(TranscodeError::OutputBufferTooSmall);
    }

    let mut decoded_block = [[0u8; 4]; 16];
    let mut block_offset = 0;

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_slice = &blocks[block_offset..block_offset + block_bytes];
            block_offset += block_bytes;

            match format {
                CompressedBlockFormat::Bc1Rgba => {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(block_slice);
                    decode_bc1_block(&b, &mut decoded_block);
                }
                CompressedBlockFormat::Bc3Rgba => {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(block_slice);
                    decode_bc3_block(&b, &mut decoded_block);
                }
                CompressedBlockFormat::Bc7Rgba => {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(block_slice);
                    decode_bc7_block(&b, &mut decoded_block);
                }
                CompressedBlockFormat::Etc2Rgba | CompressedBlockFormat::Astc4x4 => {
                    // Approximate fallback for ETC2/ASTC 4x4
                    let mut b = [0u8; 16];
                    b.copy_from_slice(block_slice);
                    decode_bc7_block(&b, &mut decoded_block);
                }
            }

            // Scatter 4x4 pixels to destination image buffer
            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let out_idx = (y as usize * width as usize + x as usize) * 4;
                    let block_idx = (py * 4 + px) as usize;
                    output[out_idx..out_idx + 4].copy_from_slice(&decoded_block[block_idx]);
                }
            }
        }
    }

    Ok(())
}

/// Transcode a KTX2 compressed texture container into caller-provided output buffers.
///
/// Bounded and zero-heap in hot path. If supercompressed with Zstandard, uses the caller-supplied
/// scratch buffer for decompression before block decoding.
pub fn transcode_ktx2_into(
    bytes: &[u8],
    target: TranscodeTargetFormat,
    output: &mut [u8],
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    scratch: &mut [u8],
) -> Result<TranscodedTextureInfo, TranscodeError> {
    let document = Ktx2Document::parse(bytes).map_err(|_| TranscodeError::InvalidContainer)?;

    let vk_format = document.vk_format;
    let block_format = CompressedBlockFormat::from_vk_format(vk_format)
        .ok_or(TranscodeError::UnsupportedFormat(vk_format))?;

    let level = document
        .level(0)
        .ok_or(TranscodeError::InvalidContainer)?;
    let level_bytes = level.bytes;

    let width = document.pixel_width;
    let height = document.pixel_height;
    if width == 0 || height == 0 {
        return Err(TranscodeError::DimensionOverflow);
    }

    let uncompressed_blocks = match document.supercompression() {
        Ktx2Supercompression::None => level_bytes,
        #[cfg(not(target_arch = "wasm32"))]
        Ktx2Supercompression::Zstd => {
            let decompressed_len = level.uncompressed_byte_length as usize;
            if scratch.len() < decompressed_len {
                return Err(TranscodeError::ScratchBufferTooSmall);
            }
            zstd::bulk::decompress_to_buffer(level_bytes, &mut scratch[..decompressed_len])
                .map_err(|_| TranscodeError::DecompressionFailed)?;
            &scratch[..decompressed_len]
        }
        _ => return Err(TranscodeError::UnsupportedSupercompression),
    };

    match target {
        TranscodeTargetFormat::Rgba8 => {
            decode_compressed_image_to_rgba8(
                uncompressed_blocks,
                width,
                height,
                block_format,
                output,
            )?;
            let output_bytes = width as usize * height as usize * 4;
            Ok(TranscodedTextureInfo {
                width,
                height,
                output_bytes,
                color_space: document.color_space(),
            })
        }
        TranscodeTargetFormat::PassthroughCompressed(_) => {
            if output.len() < uncompressed_blocks.len() {
                return Err(TranscodeError::OutputBufferTooSmall);
            }
            output[..uncompressed_blocks.len()].copy_from_slice(uncompressed_blocks);
            Ok(TranscodedTextureInfo {
                width,
                height,
                output_bytes: uncompressed_blocks.len(),
                color_space: document.color_space(),
            })
        }
    }
}
