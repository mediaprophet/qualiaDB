//! Caller-buffered CPU reference for generating one RGBA8 texture mip level.
//!
//! Dimensions and area weights match the GPU reducer in `gpu/texture_mips.rs`. These paths are
//! allocation-free; alpha masks use a separate primitive that preserves authored base coverage.

/// Filtering interpretation for one texture's RGBA8 samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CpuTextureMipSemantic {
    /// RGB is sRGB encoded; alpha is linear coverage/opacity and is averaged unchanged.
    Color,
    /// sRGB RGB plus cutoff-specific coverage preservation.
    AlphaMask { cutoff: f32 },
    /// Every component is linear numeric data.
    LinearData,
    /// RGB stores an encoded tangent/object-space unit vector; alpha is averaged linearly.
    Normal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuTextureMipError {
    ZeroDimension,
    NoSmallerLevel,
    DimensionOverflow,
    SourceLength {
        expected: usize,
        actual: usize,
    },
    OutputTooSmall {
        needed: usize,
        available: usize,
    },
    InvalidAlphaCutoff,
    /// The generic reducer does not apply alpha-coverage correction; use the alpha-mask primitive.
    AlphaCoverageCorrectionUnavailable,
}

/// Downsample an alpha-masked texture level and preserve the authored base level's alpha
/// coverage at `cutoff` as closely as the destination's discrete pixels permit.
///
/// RGB is area-filtered in linear light and encoded back to sRGB. Alpha is area-filtered in
/// linear space, then a deterministic scale is chosen against the base-level covered fraction.
/// Correction targets the alpha bytes that will be uploaded as the coarse texture's new base
/// level, so it uses the authored cutoff directly and does not apply the later GPU-mip rounding
/// bias. The destination is untouched unless all dimensions, source lengths, cutoff, and output
/// capacity are valid.
pub fn downsample_alpha_mask_level_into(
    authored_base_rgba: &[u8],
    base_width: u32,
    base_height: u32,
    source: &[u8],
    width: u32,
    height: u32,
    cutoff: f32,
    out: &mut [u8],
) -> Result<(u32, u32), CpuTextureMipError> {
    if width == 0 || height == 0 || base_width == 0 || base_height == 0 {
        return Err(CpuTextureMipError::ZeroDimension);
    }
    if !cutoff.is_finite() || !(0.0..=1.0).contains(&cutoff) {
        return Err(CpuTextureMipError::InvalidAlphaCutoff);
    }
    let source_bytes = checked_rgba8_len(width, height)?;
    if source.len() != source_bytes {
        return Err(CpuTextureMipError::SourceLength {
            expected: source_bytes,
            actual: source.len(),
        });
    }
    let base_bytes = checked_rgba8_len(base_width, base_height)?;
    if authored_base_rgba.len() != base_bytes {
        return Err(CpuTextureMipError::SourceLength {
            expected: base_bytes,
            actual: authored_base_rgba.len(),
        });
    }
    let dest_width = (width / 2).max(1);
    let dest_height = (height / 2).max(1);
    if dest_width == width && dest_height == height {
        return Err(CpuTextureMipError::NoSmallerLevel);
    }
    let dest_bytes = checked_rgba8_len(dest_width, dest_height)?;
    if out.len() < dest_bytes {
        return Err(CpuTextureMipError::OutputTooSmall {
            needed: dest_bytes,
            available: out.len(),
        });
    }

    let base_pixel_count = (base_width as usize) * (base_height as usize);
    let base_covered = authored_base_rgba
        .chunks_exact(4)
        .filter(|pixel| f32::from(pixel[3]) / 255.0 >= cutoff)
        .count();
    let target_covered = base_covered as f64 / base_pixel_count as f64
        * (dest_width as usize * dest_height as usize) as f64;

    let scale_x = width as f32 / dest_width as f32;
    let scale_y = height as f32 / dest_height as f32;
    for dy in 0..dest_height {
        for dx in 0..dest_width {
            let begin_x = dx as f32 * scale_x;
            let end_x = (dx + 1) as f32 * scale_x;
            let begin_y = dy as f32 * scale_y;
            let end_y = (dy + 1) as f32 * scale_y;
            let x0 = begin_x.floor() as u32;
            let x1 = end_x.ceil() as u32;
            let y0 = begin_y.floor() as u32;
            let y1 = end_y.ceil() as u32;
            let mut rgb_sum = [0.0f32; 3];
            let mut weight_sum = 0.0f32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let overlap_x = (end_x.min((sx + 1) as f32) - begin_x.max(sx as f32)).max(0.0);
                    let overlap_y = (end_y.min((sy + 1) as f32) - begin_y.max(sy as f32)).max(0.0);
                    let weight = overlap_x * overlap_y;
                    let src_i = ((sy as usize * width as usize) + sx as usize) * 4;
                    for channel in 0..3 {
                        rgb_sum[channel] +=
                            srgb_to_linear(f32::from(source[src_i + channel]) / 255.0) * weight;
                    }
                    weight_sum += weight;
                }
            }
            let dst_i = ((dy as usize * dest_width as usize) + dx as usize) * 4;
            for channel in 0..3 {
                let linear = rgb_sum[channel] / weight_sum;
                out[dst_i + channel] =
                    (linear_to_srgb(linear).clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    let pixel_count = dest_width as usize * dest_height as usize;
    let bias = 0.0;
    let planning_cutoff = cutoff;
    let mut high = (0..pixel_count)
        .map(|index| area_alpha(source, width, height, dest_width, dest_height, index))
        .filter(|&alpha| alpha > 0.0)
        .map(|alpha| ((planning_cutoff - bias).max(0.0)) / alpha)
        .fold(1.0f32, f32::max)
        .min(1.0e12);
    let mut low = 0.0f32;
    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        let covered = quantized_coverage_count(
            source,
            width,
            height,
            dest_width,
            dest_height,
            mid,
            planning_cutoff,
            bias,
        ) as f64;
        if covered > target_covered {
            high = mid;
        } else {
            low = mid;
        }
    }
    let low_error = (quantized_coverage_count(
        source,
        width,
        height,
        dest_width,
        dest_height,
        low,
        planning_cutoff,
        bias,
    ) as f64
        - target_covered)
        .abs();
    let high_error = (quantized_coverage_count(
        source,
        width,
        height,
        dest_width,
        dest_height,
        high,
        planning_cutoff,
        bias,
    ) as f64
        - target_covered)
        .abs();
    let scale = if low_error < high_error {
        low
    } else if high_error < low_error {
        high
    } else if (low - 1.0).abs() <= (high - 1.0).abs() {
        low
    } else {
        high
    };
    for index in 0..pixel_count {
        let dst_i = index * 4 + 3;
        let alpha = area_alpha(source, width, height, dest_width, dest_height, index);
        out[dst_i] = ((alpha * scale + bias).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    Ok((dest_width, dest_height))
}

fn checked_rgba8_len(width: u32, height: u32) -> Result<usize, CpuTextureMipError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(CpuTextureMipError::DimensionOverflow)
}

fn area_alpha(
    source: &[u8],
    width: u32,
    height: u32,
    dest_width: u32,
    dest_height: u32,
    index: usize,
) -> f32 {
    let dx = (index % dest_width as usize) as u32;
    let dy = (index / dest_width as usize) as u32;
    let scale_x = width as f32 / dest_width as f32;
    let scale_y = height as f32 / dest_height as f32;
    let begin_x = dx as f32 * scale_x;
    let end_x = (dx + 1) as f32 * scale_x;
    let begin_y = dy as f32 * scale_y;
    let end_y = (dy + 1) as f32 * scale_y;
    let mut sum = 0.0f32;
    let mut weight_sum = 0.0f32;
    for sy in begin_y.floor() as u32..end_y.ceil() as u32 {
        for sx in begin_x.floor() as u32..end_x.ceil() as u32 {
            let overlap_x = (end_x.min((sx + 1) as f32) - begin_x.max(sx as f32)).max(0.0);
            let overlap_y = (end_y.min((sy + 1) as f32) - begin_y.max(sy as f32)).max(0.0);
            let weight = overlap_x * overlap_y;
            let source_index = (sy as usize * width as usize + sx as usize) * 4 + 3;
            sum += f32::from(source[source_index]) / 255.0 * weight;
            weight_sum += weight;
        }
    }
    sum / weight_sum
}

fn quantized_coverage_count(
    source: &[u8],
    width: u32,
    height: u32,
    dest_width: u32,
    dest_height: u32,
    scale: f32,
    cutoff: f32,
    bias: f32,
) -> usize {
    (0..dest_width as usize * dest_height as usize)
        .filter(|&index| {
            let alpha = area_alpha(source, width, height, dest_width, dest_height, index);
            let quantized = ((alpha * scale + bias).clamp(0.0, 1.0) * 255.0).round() as u8;
            f32::from(quantized) / 255.0 >= cutoff
        })
        .count()
}

/// Downsample exactly one floor-halved (minimum dimension one) RGBA8 level.
///
/// `source` must contain exactly `width * height * 4` bytes. The output dimensions are
/// `max(width / 2, 1)` by `max(height / 2, 1)`. The destination is untouched on every error;
/// on success only its required prefix is written. Color RGB uses sRGB transfer conversion,
/// alpha and linear data use area-weighted component averages, and normals are renormalized.
pub fn downsample_rgba8_level_into(
    source: &[u8],
    width: u32,
    height: u32,
    semantic: CpuTextureMipSemantic,
    out: &mut [u8],
) -> Result<(u32, u32), CpuTextureMipError> {
    if width == 0 || height == 0 {
        return Err(CpuTextureMipError::ZeroDimension);
    }
    if let CpuTextureMipSemantic::AlphaMask { cutoff } = semantic {
        if !cutoff.is_finite() || !(0.0..=1.0).contains(&cutoff) {
            return Err(CpuTextureMipError::InvalidAlphaCutoff);
        }
        return Err(CpuTextureMipError::AlphaCoverageCorrectionUnavailable);
    }

    let source_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(CpuTextureMipError::DimensionOverflow)?;
    let expected_source = source_pixels
        .checked_mul(4)
        .ok_or(CpuTextureMipError::DimensionOverflow)?;
    if source.len() != expected_source {
        return Err(CpuTextureMipError::SourceLength {
            expected: expected_source,
            actual: source.len(),
        });
    }

    let dest_width = (width / 2).max(1);
    let dest_height = (height / 2).max(1);
    if dest_width == width && dest_height == height {
        return Err(CpuTextureMipError::NoSmallerLevel);
    }
    let dest_bytes = (dest_width as usize)
        .checked_mul(dest_height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(CpuTextureMipError::DimensionOverflow)?;
    if out.len() < dest_bytes {
        return Err(CpuTextureMipError::OutputTooSmall {
            needed: dest_bytes,
            available: out.len(),
        });
    }

    let scale_x = width as f32 / dest_width as f32;
    let scale_y = height as f32 / dest_height as f32;
    for dy in 0..dest_height {
        for dx in 0..dest_width {
            let begin_x = dx as f32 * scale_x;
            let end_x = (dx + 1) as f32 * scale_x;
            let begin_y = dy as f32 * scale_y;
            let end_y = (dy + 1) as f32 * scale_y;
            let x0 = begin_x.floor() as u32;
            let x1 = end_x.ceil() as u32;
            let y0 = begin_y.floor() as u32;
            let y1 = end_y.ceil() as u32;
            let mut sum = [0.0f32; 4];
            let mut weight_sum = 0.0f32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let overlap_x = (end_x.min((sx + 1) as f32) - begin_x.max(sx as f32)).max(0.0);
                    let overlap_y = (end_y.min((sy + 1) as f32) - begin_y.max(sy as f32)).max(0.0);
                    let weight = overlap_x * overlap_y;
                    let src_i = ((sy as usize * width as usize) + sx as usize) * 4;
                    for channel in 0..4 {
                        let value = f32::from(source[src_i + channel]) / 255.0;
                        sum[channel] +=
                            if channel < 3 && matches!(semantic, CpuTextureMipSemantic::Color) {
                                srgb_to_linear(value) * weight
                            } else {
                                value * weight
                            };
                    }
                    weight_sum += weight;
                }
            }
            let mut average = [0.0f32; 4];
            for channel in 0..4 {
                average[channel] = sum[channel] / weight_sum;
            }
            if matches!(semantic, CpuTextureMipSemantic::Normal) {
                let mut n = [
                    average[0] * 2.0 - 1.0,
                    average[1] * 2.0 - 1.0,
                    average[2] * 2.0 - 1.0,
                ];
                let len2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
                if len2 > 1.0e-12 {
                    let inv_len = len2.sqrt().recip();
                    for component in &mut n {
                        *component *= inv_len;
                    }
                } else {
                    n = [0.0, 0.0, 1.0];
                }
                average[0] = n[0] * 0.5 + 0.5;
                average[1] = n[1] * 0.5 + 0.5;
                average[2] = n[2] * 0.5 + 0.5;
            } else if matches!(semantic, CpuTextureMipSemantic::Color) {
                for channel in &mut average[..3] {
                    *channel = linear_to_srgb(*channel);
                }
            }
            let dst_i = ((dy as usize * dest_width as usize) + dx as usize) * 4;
            for channel in 0..4 {
                out[dst_i + channel] = (average[channel].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    Ok((dest_width, dest_height))
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
#[path = "cpu_texture_mips_tests.rs"]
mod tests;
