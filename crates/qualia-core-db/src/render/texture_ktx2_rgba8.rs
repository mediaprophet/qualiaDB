//! Caller-buffered decoder for a deliberately small KTX2 subset.
//!
//! Supports only 2D, non-array, single-face, uncompressed RGBA8 UNORM/SRGB images. The
//! container parser owns structural validation; this module checks the texture subset and exact
//! texel byte count before writing. It does not interpret the DFD or decode compressed formats.

use super::texture_ktx2::Ktx2Document;

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
    MissingBaseLevel,
    InvalidDimensions,
    InvalidBaseLevelLength,
    OutputTooSmall,
}

/// Validate the supported texture subset and report the base-level output requirements.
///
/// This inspection is allocation-free and does not write or decode texels, so callers can use
/// the dimensions and byte length to make bounded allocation/admission decisions first.
pub fn inspect_ktx2_rgba8_base_level(
    document: &Ktx2Document<'_>,
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
    if document.supercompression_scheme != 0 {
        return Err(Rgba8Ktx2Error::UnsupportedSupercompression);
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
    if level.bytes.len() != byte_len || level.uncompressed_byte_length != byte_len as u64 {
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

#[cfg(test)]
#[path = "texture_ktx2_rgba8_tests.rs"]
mod tests;
