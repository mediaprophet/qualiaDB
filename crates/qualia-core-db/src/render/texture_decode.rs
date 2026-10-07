//! Bounded cold-path image decoding for renderer resources.
//!
//! Decoders write into caller-owned RGBA8 and scratch buffers. Image payloads remain external to
//! the semantic arena; this module only enforces per-image decode limits before GPU admission.

use super::asset_package::HmcTextureResource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureDecodeLimits {
    pub max_encoded_bytes: usize,
    pub max_width: u32,
    pub max_height: u32,
    pub max_decoded_bytes: usize,
}

impl Default for TextureDecodeLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: 64 * 1024 * 1024,
            max_width: 16_384,
            max_height: 16_384,
            max_decoded_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedTextureInfo {
    pub width: u32,
    pub height: u32,
    pub rgba8_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDecodeError {
    EncodedInputTooLarge,
    InvalidImage,
    DimensionsExceedLimit,
    DecodedImageExceedsLimit,
    OutputBufferTooSmall,
    ScratchBufferTooSmall,
    UnsupportedMime,
}

impl std::fmt::Display for TextureDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture decode: {self:?}")
    }
}

impl std::error::Error for TextureDecodeError {}

/// Decode a verified HMC texture to tightly packed RGBA8.
///
/// `output` and `scratch` are caller-owned. The function never grows either buffer, and fails
/// before decoding when encoded, dimension, decoded-byte, or caller-buffer limits are exceeded.
/// PNG and JPEG are supported on native and WASM. KTX2/Basis and WebP remain preserved source
/// resources; they require the later GPU-format transcode integration.
pub fn decode_hmc_texture_rgba8_into(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
    output: &mut [u8],
    scratch: &mut [u8],
) -> Result<DecodedTextureInfo, TextureDecodeError> {
    if resource.bytes.len() > limits.max_encoded_bytes {
        return Err(TextureDecodeError::EncodedInputTooLarge);
    }
    match resource.mime_type {
        "image/png" => decode_png_rgba8(resource.bytes, limits, output, scratch),
        "image/jpeg" | "image/jpg" => decode_jpeg_rgba8(resource.bytes, limits, output),
        _ => Err(TextureDecodeError::UnsupportedMime),
    }
}

/// Bounded cold-path convenience for asset loaders that own the destination allocation. The
/// maximum output and scratch sizes are established from codec metadata before either vector is
/// allocated; use the `_into` form when the caller already has workspace buffers.
pub fn decode_hmc_texture_rgba8(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
) -> Result<(Vec<u8>, DecodedTextureInfo), TextureDecodeError> {
    let (info, scratch_len) = inspect_decode_requirements(resource, limits)?;
    let mut output = vec![0; info.rgba8_bytes];
    let mut scratch = vec![0; scratch_len];
    let decoded = decode_hmc_texture_rgba8_into(resource, limits, &mut output, &mut scratch)?;
    Ok((output, decoded))
}

fn inspect_decode_requirements(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
) -> Result<(DecodedTextureInfo, usize), TextureDecodeError> {
    if resource.bytes.len() > limits.max_encoded_bytes {
        return Err(TextureDecodeError::EncodedInputTooLarge);
    }
    match resource.mime_type {
        "image/png" => {
            let mut decoder = png::Decoder::new_with_limits(
                resource.bytes,
                png::Limits {
                    bytes: limits.max_decoded_bytes,
                },
            );
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let reader = decoder
                .read_info()
                .map_err(|_| TextureDecodeError::InvalidImage)?;
            let width = reader.info().width;
            let height = reader.info().height;
            let rgba8_bytes = required_rgba_bytes(width, height, limits, usize::MAX)?;
            let scratch_len = reader.output_buffer_size();
            if scratch_len > limits.max_decoded_bytes {
                return Err(TextureDecodeError::DecodedImageExceedsLimit);
            }
            Ok((
                DecodedTextureInfo {
                    width,
                    height,
                    rgba8_bytes,
                },
                scratch_len,
            ))
        }
        "image/jpeg" | "image/jpg" => {
            let mut decoder = jpeg_decoder::Decoder::new(resource.bytes);
            decoder.set_max_decoding_buffer_size(limits.max_decoded_bytes);
            decoder
                .read_info()
                .map_err(|_| TextureDecodeError::InvalidImage)?;
            let dimensions = decoder.info().ok_or(TextureDecodeError::InvalidImage)?;
            let width = u32::from(dimensions.width);
            let height = u32::from(dimensions.height);
            let rgba8_bytes = required_rgba_bytes(width, height, limits, usize::MAX)?;
            Ok((
                DecodedTextureInfo {
                    width,
                    height,
                    rgba8_bytes,
                },
                0,
            ))
        }
        _ => Err(TextureDecodeError::UnsupportedMime),
    }
}

fn required_rgba_bytes(
    width: u32,
    height: u32,
    limits: TextureDecodeLimits,
    output_len: usize,
) -> Result<usize, TextureDecodeError> {
    if width == 0 || height == 0 || width > limits.max_width || height > limits.max_height {
        return Err(TextureDecodeError::DimensionsExceedLimit);
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(TextureDecodeError::DecodedImageExceedsLimit)?;
    if bytes > limits.max_decoded_bytes {
        return Err(TextureDecodeError::DecodedImageExceedsLimit);
    }
    if output_len < bytes {
        return Err(TextureDecodeError::OutputBufferTooSmall);
    }
    Ok(bytes)
}

fn decode_png_rgba8(
    bytes: &[u8],
    limits: TextureDecodeLimits,
    output: &mut [u8],
    scratch: &mut [u8],
) -> Result<DecodedTextureInfo, TextureDecodeError> {
    let mut decoder = png::Decoder::new_with_limits(
        bytes,
        png::Limits {
            bytes: limits.max_decoded_bytes,
        },
    );
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|_| TextureDecodeError::InvalidImage)?;
    let width = reader.info().width;
    let height = reader.info().height;
    let rgba_bytes = required_rgba_bytes(width, height, limits, output.len())?;
    let required_scratch = reader.output_buffer_size();
    if required_scratch > limits.max_decoded_bytes {
        return Err(TextureDecodeError::DecodedImageExceedsLimit);
    }
    if scratch.len() < required_scratch {
        return Err(TextureDecodeError::ScratchBufferTooSmall);
    }
    let info = reader
        .next_frame(scratch)
        .map_err(|_| TextureDecodeError::InvalidImage)?;
    if info.width != width || info.height != height {
        return Err(TextureDecodeError::InvalidImage);
    }

    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Indexed => return Err(TextureDecodeError::InvalidImage),
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    let row_bytes = width_usize
        .checked_mul(channels)
        .ok_or(TextureDecodeError::DecodedImageExceedsLimit)?;
    if info.line_size < row_bytes || info.buffer_size() > scratch.len() {
        return Err(TextureDecodeError::InvalidImage);
    }
    for y in 0..height_usize {
        let row = &scratch[y * info.line_size..y * info.line_size + row_bytes];
        for x in 0..width_usize {
            let src = x * channels;
            let dst = (y * width_usize + x) * 4;
            match channels {
                1 => output[dst..dst + 4].copy_from_slice(&[row[src], row[src], row[src], 255]),
                2 => output[dst..dst + 4].copy_from_slice(&[
                    row[src],
                    row[src],
                    row[src],
                    row[src + 1],
                ]),
                3 => output[dst..dst + 4].copy_from_slice(&[
                    row[src],
                    row[src + 1],
                    row[src + 2],
                    255,
                ]),
                4 => output[dst..dst + 4].copy_from_slice(&row[src..src + 4]),
                _ => unreachable!(),
            }
        }
    }
    Ok(DecodedTextureInfo {
        width,
        height,
        rgba8_bytes: rgba_bytes,
    })
}

fn decode_jpeg_rgba8(
    bytes: &[u8],
    limits: TextureDecodeLimits,
    output: &mut [u8],
) -> Result<DecodedTextureInfo, TextureDecodeError> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.set_max_decoding_buffer_size(limits.max_decoded_bytes);
    decoder
        .read_info()
        .map_err(|_| TextureDecodeError::InvalidImage)?;
    let dimensions = decoder.info().ok_or(TextureDecodeError::InvalidImage)?;
    let width = u32::from(dimensions.width);
    let height = u32::from(dimensions.height);
    let rgba_bytes = required_rgba_bytes(width, height, limits, output.len())?;
    let pixels = decoder
        .decode()
        .map_err(|_| TextureDecodeError::InvalidImage)?;
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(TextureDecodeError::DecodedImageExceedsLimit)?;
    match dimensions.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            if pixels.len()
                != count
                    .checked_mul(3)
                    .ok_or(TextureDecodeError::DecodedImageExceedsLimit)?
            {
                return Err(TextureDecodeError::InvalidImage);
            }
            for (src, dst) in pixels
                .chunks_exact(3)
                .zip(output[..rgba_bytes].chunks_exact_mut(4))
            {
                dst.copy_from_slice(&[src[0], src[1], src[2], 255]);
            }
        }
        jpeg_decoder::PixelFormat::L8 => {
            if pixels.len() != count {
                return Err(TextureDecodeError::InvalidImage);
            }
            for (&gray, dst) in pixels.iter().zip(output[..rgba_bytes].chunks_exact_mut(4)) {
                dst.copy_from_slice(&[gray, gray, gray, 255]);
            }
        }
        jpeg_decoder::PixelFormat::L16 | jpeg_decoder::PixelFormat::CMYK32 => {
            return Err(TextureDecodeError::UnsupportedMime);
        }
    }
    Ok(DecodedTextureInfo {
        width,
        height,
        rgba8_bytes: rgba_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[10, 20, 30, 40, 50, 60, 70, 80])
                .unwrap();
        }
        bytes
    }

    fn resource<'a>(bytes: &'a [u8], mime_type: &'a str) -> HmcTextureResource<'a> {
        HmcTextureResource {
            digest: [0; 32],
            mime_type,
            bytes,
        }
    }

    #[test]
    fn png_decodes_to_caller_buffer_and_preserves_alpha() {
        let bytes = png_fixture();
        let image = resource(&bytes, "image/png");
        let mut output = [0; 8];
        let mut scratch = [0; 32];
        let info = decode_hmc_texture_rgba8_into(
            &image,
            TextureDecodeLimits::default(),
            &mut output,
            &mut scratch,
        )
        .unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(&output, &[10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn png_decode_fails_before_writing_when_output_or_limits_are_small() {
        let bytes = png_fixture();
        let image = resource(&bytes, "image/png");
        let mut output = [0; 7];
        let mut scratch = [0; 32];
        assert_eq!(
            decode_hmc_texture_rgba8_into(
                &image,
                TextureDecodeLimits::default(),
                &mut output,
                &mut scratch,
            ),
            Err(TextureDecodeError::OutputBufferTooSmall)
        );
        let limits = TextureDecodeLimits {
            max_decoded_bytes: 7,
            ..TextureDecodeLimits::default()
        };
        assert_eq!(
            decode_hmc_texture_rgba8_into(&image, limits, &mut [0; 8], &mut [0; 32]),
            Err(TextureDecodeError::DecodedImageExceedsLimit)
        );
    }

    #[test]
    fn compressed_texture_mime_is_preserved_but_not_claimed_as_decoded() {
        let image = resource(&[1, 2, 3], "image/ktx2");
        assert_eq!(
            decode_hmc_texture_rgba8_into(
                &image,
                TextureDecodeLimits::default(),
                &mut [0; 16],
                &mut [0; 16],
            ),
            Err(TextureDecodeError::UnsupportedMime)
        );
    }
}
