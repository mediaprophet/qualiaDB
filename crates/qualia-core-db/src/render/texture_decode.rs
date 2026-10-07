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
    UnsupportedImageFormat,
    UnsupportedSupercompression,
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
/// PNG and JPEG are supported on native and WASM. KTX2 supports only uncompressed 2D RGBA8
/// UNORM/SRGB base levels. Basis, Zstd/ZLIB supercompression, other KTX2 formats, and WebP remain
/// preserved source resources pending bounded decode/transcode support.
pub fn decode_hmc_texture_rgba8_into(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
    output: &mut [u8],
    scratch: &mut [u8],
) -> Result<DecodedTextureInfo, TextureDecodeError> {
    if resource.bytes.len() > limits.max_encoded_bytes {
        return Err(TextureDecodeError::EncodedInputTooLarge);
    }
    if mime_matches(resource.mime_type, "image/png") {
        decode_png_rgba8(resource.bytes, limits, output, scratch)
    } else if mime_matches(resource.mime_type, "image/jpeg")
        || mime_matches(resource.mime_type, "image/jpg")
    {
        decode_jpeg_rgba8(resource.bytes, limits, output)
    } else if mime_matches(resource.mime_type, "image/ktx2") {
        decode_ktx2_rgba8(resource.bytes, limits, output)
    } else {
        Err(TextureDecodeError::UnsupportedMime)
    }
}

/// Bounded cold-path convenience for asset loaders that own the destination allocation. The
/// maximum output and scratch sizes are established from codec metadata before either vector is
/// allocated; use the `_into` form when the caller already has workspace buffers.
pub fn decode_hmc_texture_rgba8(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
) -> Result<(Vec<u8>, DecodedTextureInfo), TextureDecodeError> {
    let (info, scratch_len) = inspect_hmc_texture_requirements(resource, limits)?;
    let mut output = vec![0; info.rgba8_bytes];
    let mut scratch = vec![0; scratch_len];
    let decoded = decode_hmc_texture_rgba8_into(resource, limits, &mut output, &mut scratch)?;
    Ok((output, decoded))
}

/// Validate codec metadata and report exact RGBA8 output and decoder scratch requirements without
/// allocating or decoding the image. Asset admission can use this to plan texture residency
/// before materializing pixel buffers.
pub fn inspect_hmc_texture_requirements(
    resource: &HmcTextureResource<'_>,
    limits: TextureDecodeLimits,
) -> Result<(DecodedTextureInfo, usize), TextureDecodeError> {
    if resource.bytes.len() > limits.max_encoded_bytes {
        return Err(TextureDecodeError::EncodedInputTooLarge);
    }
    if mime_matches(resource.mime_type, "image/png") {
        let mut decoder = png::Decoder::new_with_limits(
            resource.bytes,
            png::Limits {
                bytes: limits.max_decoded_bytes,
            },
        );
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let reader = decoder.read_info().map_err(map_png_error)?;
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
    } else if mime_matches(resource.mime_type, "image/jpeg")
        || mime_matches(resource.mime_type, "image/jpg")
    {
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
    } else if mime_matches(resource.mime_type, "image/ktx2") {
        inspect_ktx2_rgba8(resource.bytes, limits)
    } else {
        Err(TextureDecodeError::UnsupportedMime)
    }
}

fn mime_matches(actual: &str, expected: &str) -> bool {
    actual
        .split(';')
        .next()
        .map(str::trim)
        .is_some_and(|mime| mime.eq_ignore_ascii_case(expected))
}

fn inspect_ktx2_rgba8(
    bytes: &[u8],
    limits: TextureDecodeLimits,
) -> Result<(DecodedTextureInfo, usize), TextureDecodeError> {
    use super::texture_ktx2::Ktx2Document;
    use super::texture_ktx2_rgba8::inspect_ktx2_rgba8_base_level;

    let document = Ktx2Document::parse(bytes).map_err(|_| TextureDecodeError::InvalidImage)?;
    let image = inspect_ktx2_rgba8_base_level(&document).map_err(map_ktx2_rgba8_error)?;
    let rgba8_bytes = required_rgba_bytes(image.width, image.height, limits, usize::MAX)?;
    if rgba8_bytes != image.byte_len {
        return Err(TextureDecodeError::InvalidImage);
    }
    Ok((
        DecodedTextureInfo {
            width: image.width,
            height: image.height,
            rgba8_bytes,
        },
        0,
    ))
}

fn decode_ktx2_rgba8(
    bytes: &[u8],
    limits: TextureDecodeLimits,
    output: &mut [u8],
) -> Result<DecodedTextureInfo, TextureDecodeError> {
    use super::texture_ktx2::Ktx2Document;
    use super::texture_ktx2_rgba8::{decode_ktx2_rgba8_base_level, Rgba8Ktx2Error};

    let document = Ktx2Document::parse(bytes).map_err(|_| TextureDecodeError::InvalidImage)?;
    let image = super::texture_ktx2_rgba8::inspect_ktx2_rgba8_base_level(&document)
        .map_err(map_ktx2_rgba8_error)?;
    let rgba8_bytes = required_rgba_bytes(image.width, image.height, limits, output.len())?;
    decode_ktx2_rgba8_base_level(&document, &mut output[..rgba8_bytes]).map_err(
        |error| match error {
            Rgba8Ktx2Error::OutputTooSmall => TextureDecodeError::OutputBufferTooSmall,
            other => map_ktx2_rgba8_error(other),
        },
    )?;
    Ok(DecodedTextureInfo {
        width: image.width,
        height: image.height,
        rgba8_bytes,
    })
}

fn map_ktx2_rgba8_error(error: super::texture_ktx2_rgba8::Rgba8Ktx2Error) -> TextureDecodeError {
    use super::texture_ktx2_rgba8::Rgba8Ktx2Error;
    match error {
        Rgba8Ktx2Error::UnsupportedFormat | Rgba8Ktx2Error::UnsupportedTypeSize => {
            TextureDecodeError::UnsupportedImageFormat
        }
        Rgba8Ktx2Error::UnsupportedSupercompression => {
            TextureDecodeError::UnsupportedSupercompression
        }
        Rgba8Ktx2Error::OutputTooSmall => TextureDecodeError::OutputBufferTooSmall,
        Rgba8Ktx2Error::InvalidDimensions => TextureDecodeError::DecodedImageExceedsLimit,
        Rgba8Ktx2Error::UnsupportedDimensions
        | Rgba8Ktx2Error::UnsupportedArray
        | Rgba8Ktx2Error::UnsupportedFaceCount
        | Rgba8Ktx2Error::MissingBaseLevel
        | Rgba8Ktx2Error::InvalidBaseLevelLength => TextureDecodeError::InvalidImage,
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
    let mut reader = decoder.read_info().map_err(map_png_error)?;
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
    let info = reader.next_frame(scratch).map_err(map_png_error)?;
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

fn map_png_error(error: png::DecodingError) -> TextureDecodeError {
    match error {
        png::DecodingError::LimitsExceeded => TextureDecodeError::DecodedImageExceedsLimit,
        _ => TextureDecodeError::InvalidImage,
    }
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
#[path = "texture_decode_tests.rs"]
mod tests;
