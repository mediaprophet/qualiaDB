//! Caller-buffered decoder for a deliberately small KTX2 subset.
//!
//! Supports only 2D, non-array, single-face, RGBA8 UNORM/SRGB images. The container parser owns
//! structural validation; this module checks the texture subset and exact texel byte count before
//! writing. Zstd/Zlib are supercompression wrappers around RGBA8 and use caller scratch; GPU block
//! formats still require a real transcode/target decision.

use super::texture_ktx2::{Ktx2Document, Ktx2Supercompression, Ktx2TranscodeSource};

pub const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
pub const VK_FORMAT_R8G8B8A8_SRGB: u32 = 43;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rgba8Ktx2Format {
    Unorm,
    Srgb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba8Ktx2Image {
    pub width: u32,
    pub height: u32,
    pub byte_len: usize,
    pub format: Rgba8Ktx2Format,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rgba8Ktx2Error {
    UnsupportedFormat,
    UnsupportedTypeSize,
    UnsupportedDimensions,
    UnsupportedArray,
    UnsupportedFaceCount,
    UnsupportedSupercompression,
    RequiresTranscode(Ktx2TranscodeSource),
    MissingBaseLevel,
    InvalidDimensions,
    InvalidBaseLevelLength,
    InvalidCompressedLevel,
    CompressedScratchTooSmall,
    OutputTooSmall,
}

/// Validate the supported texture subset and report the base-level output requirements.
///
/// This inspection is allocation-free and does not write or decode texels, so callers can use
/// the dimensions and byte length to make bounded allocation/admission decisions first.
pub fn inspect_ktx2_rgba8_base_level(
    document: &Ktx2Document<'_>,
) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    inspect_ktx2_rgba8_base_level_with_compression(document, false)
}

/// Inspect RGBA8 KTX2 while allowing the bounded Zstd/Zlib paths. BasisLZ and GPU block formats
/// remain typed transcode requirements; no decoder is implied by this inspection API.
pub fn inspect_ktx2_rgba8_base_level_with_compression(
    document: &Ktx2Document<'_>,
    allow_supercompression: bool,
) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    let format = match document.vk_format {
        VK_FORMAT_R8G8B8A8_UNORM => Rgba8Ktx2Format::Unorm,
        VK_FORMAT_R8G8B8A8_SRGB => Rgba8Ktx2Format::Srgb,
        _ => return Err(Rgba8Ktx2Error::UnsupportedFormat),
    };
    if document.type_size != 1 {
        return Err(Rgba8Ktx2Error::UnsupportedTypeSize);
    }
    if document.pixel_width == 0 || document.pixel_height == 0 || document.pixel_depth != 0 {
        return Err(Rgba8Ktx2Error::UnsupportedDimensions);
    }
    if document.layer_count_field != 0 {
        return Err(Rgba8Ktx2Error::UnsupportedArray);
    }
    if document.face_count != 1 {
        return Err(Rgba8Ktx2Error::UnsupportedFaceCount);
    }
    match document.supercompression() {
        Ktx2Supercompression::None => {}
        Ktx2Supercompression::Zstd | Ktx2Supercompression::Zlib if allow_supercompression => {}
        Ktx2Supercompression::BasisLz => {
            return Err(Rgba8Ktx2Error::RequiresTranscode(
                Ktx2TranscodeSource::BasisLz,
            ));
        }
        Ktx2Supercompression::Unsupported(_)
        | Ktx2Supercompression::Zstd
        | Ktx2Supercompression::Zlib => {
            return Err(Rgba8Ktx2Error::UnsupportedSupercompression);
        }
    }

    let width =
        usize::try_from(document.pixel_width).map_err(|_| Rgba8Ktx2Error::InvalidDimensions)?;
    let height =
        usize::try_from(document.pixel_height).map_err(|_| Rgba8Ktx2Error::InvalidDimensions)?;
    let byte_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(Rgba8Ktx2Error::InvalidDimensions)?;
    let level = document.level(0).ok_or(Rgba8Ktx2Error::MissingBaseLevel)?;
    if level.uncompressed_byte_length != byte_len as u64
        || (document.supercompression() == Ktx2Supercompression::None
            && level.bytes.len() != byte_len)
    {
        return Err(Rgba8Ktx2Error::InvalidBaseLevelLength);
    }
    Ok(Rgba8Ktx2Image {
        width: document.pixel_width,
        height: document.pixel_height,
        byte_len,
        format,
    })
}

/// Copy level zero into caller memory after validating all format and size constraints.
///
/// This function performs no allocation. On any error, `out` is left unchanged. Additional mip
/// levels may exist in the source; this routine intentionally returns only level zero.
pub fn decode_ktx2_rgba8_base_level(
    document: &Ktx2Document<'_>,
    out: &mut [u8],
) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    let image = inspect_ktx2_rgba8_base_level(document)?;
    if out.len() < image.byte_len {
        return Err(Rgba8Ktx2Error::OutputTooSmall);
    }
    let level = document
        .level(0)
        .expect("inspection validated the base level exists");
    out[..image.byte_len].copy_from_slice(level.bytes);
    Ok(image)
}

/// Decode an RGBA8 base level using caller-owned scratch for KTX2 Zstd/Zlib supercompression.
///
/// The output is written only after the compressed level has expanded to its exact declared
/// length. On every error, `out` is unchanged. Native builds use the already-declared crate
/// codecs; WASM callers receive `UnsupportedSupercompression` and can retain the source for a
/// device-side or external refinement path.
pub fn decode_ktx2_rgba8_base_level_with_scratch(
    document: &Ktx2Document<'_>,
    out: &mut [u8],
    scratch: &mut [u8],
) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    let image = inspect_ktx2_rgba8_base_level_with_compression(document, true)?;
    if out.len() < image.byte_len {
        return Err(Rgba8Ktx2Error::OutputTooSmall);
    }
    let level = document
        .level(0)
        .ok_or(Rgba8Ktx2Error::MissingBaseLevel)?;
    match document.supercompression() {
        Ktx2Supercompression::None => out[..image.byte_len].copy_from_slice(level.bytes),
        Ktx2Supercompression::Zstd | Ktx2Supercompression::Zlib => {
            if scratch.len() < image.byte_len {
                return Err(Rgba8Ktx2Error::CompressedScratchTooSmall);
            }
            let written = decompress_level(
                document.supercompression(),
                level.bytes,
                &mut scratch[..image.byte_len],
            )?;
            if written != image.byte_len {
                return Err(Rgba8Ktx2Error::InvalidCompressedLevel);
            }
            out[..image.byte_len].copy_from_slice(&scratch[..image.byte_len]);
        }
        Ktx2Supercompression::BasisLz | Ktx2Supercompression::Unsupported(_) => {
            return Err(Rgba8Ktx2Error::UnsupportedSupercompression);
        }
    }
    Ok(image)
}

#[cfg(not(target_arch = "wasm32"))]
fn decompress_level(
    scheme: Ktx2Supercompression,
    compressed: &[u8],
    output: &mut [u8],
) -> Result<usize, Rgba8Ktx2Error> {
    match scheme {
        Ktx2Supercompression::Zstd => zstd::bulk::decompress_to_buffer(compressed, output)
            .map_err(|_| Rgba8Ktx2Error::InvalidCompressedLevel),
        Ktx2Supercompression::Zlib => {
            use flate2::{Decompress, FlushDecompress, Status};
            let mut decoder = Decompress::new(true);
            let status = decoder
                .decompress(compressed, output, FlushDecompress::Finish)
                .map_err(|_| Rgba8Ktx2Error::InvalidCompressedLevel)?;
            if status != Status::StreamEnd || decoder.total_in() != compressed.len() as u64 {
                return Err(Rgba8Ktx2Error::InvalidCompressedLevel);
            }
            usize::try_from(decoder.total_out())
                .map_err(|_| Rgba8Ktx2Error::InvalidCompressedLevel)
        }
        _ => Err(Rgba8Ktx2Error::UnsupportedSupercompression),
    }
}

#[cfg(target_arch = "wasm32")]
fn decompress_level(
    _scheme: Ktx2Supercompression,
    _compressed: &[u8],
    _output: &mut [u8],
) -> Result<usize, Rgba8Ktx2Error> {
    Err(Rgba8Ktx2Error::UnsupportedSupercompression)
}

#[cfg(test)]
#[path = "texture_ktx2_rgba8_tests.rs"]
mod tests;
