//! Caller-buffered CPU reference for generating one RGBA8 texture mip level.
//!
//! Dimensions and area weights match the GPU reducer in `gpu/texture_mips.rs`. This path is
//! allocation-free and deliberately rejects alpha-mask filtering until the bounded coverage
//! planner can be shared without exposing its allocating chain builder.

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
    /// Coverage-aware alpha correction is not currently exposed as a caller-buffered primitive.
    AlphaCoverageCorrectionUnavailable,
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
mod tests {
    use super::*;

    #[test]
    fn odd_dimensions_area_filter_all_source_texels_and_floor_dimensions() {
        let source = [
            0, 0, 0, 0, 30, 30, 30, 30, 60, 60, 60, 60, 90, 90, 90, 90, 120, 120, 120, 120, 150,
            150, 150, 150, 180, 180, 180, 180, 210, 210, 210, 210, 240, 240, 240, 240,
        ];
        let mut out = [0u8; 2 * 4 * 4];
        assert_eq!(
            downsample_rgba8_level_into(&source, 3, 3, CpuTextureMipSemantic::LinearData, &mut out),
            Ok((1, 1))
        );
        // Floor-halved 3x3 is 1x1 and area filtering covers the full source extent.
        assert_eq!(&out[..4], &[120, 120, 120, 120]);

        let source = [42u8; 5 * 2 * 4];
        let mut out = [0u8; 2 * 1 * 4];
        assert_eq!(
            downsample_rgba8_level_into(&source, 5, 2, CpuTextureMipSemantic::LinearData, &mut out),
            Ok((2, 1))
        );
        assert_eq!(out, [42; 8]);
    }

    #[test]
    fn insufficient_output_is_unchanged() {
        let source = [255u8; 4 * 4 * 4];
        let mut out = [0xA5; 15];
        assert_eq!(
            downsample_rgba8_level_into(&source, 4, 4, CpuTextureMipSemantic::Color, &mut out),
            Err(CpuTextureMipError::OutputTooSmall {
                needed: 16,
                available: 15
            })
        );
        assert_eq!(out, [0xA5; 15]);
    }

    #[test]
    fn color_filter_averages_in_linear_light_and_preserves_linear_alpha() {
        let source = [
            0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 64, 255, 255, 255, 192,
        ];
        let mut color = [0u8; 4];
        let mut linear = [0u8; 4];
        downsample_rgba8_level_into(&source, 2, 2, CpuTextureMipSemantic::Color, &mut color)
            .unwrap();
        downsample_rgba8_level_into(
            &source,
            2,
            2,
            CpuTextureMipSemantic::LinearData,
            &mut linear,
        )
        .unwrap();
        assert!(color[0] > linear[0]);
        assert_eq!(color[0], color[1]);
        assert_eq!(color[0], color[2]);
        assert_eq!(color[3], 128);
        assert_eq!(linear[3], 128);
        assert_eq!(linear[0], 128);
    }

    #[test]
    fn normal_filter_renormalizes_rgb_and_averages_alpha() {
        let source = [
            255, 128, 128, 0, 128, 255, 128, 64, 0, 128, 128, 128, 128, 0, 255, 255,
        ];
        let mut out = [0u8; 4];
        downsample_rgba8_level_into(&source, 2, 2, CpuTextureMipSemantic::Normal, &mut out)
            .unwrap();
        let n = [
            f32::from(out[0]) / 127.5 - 1.0,
            f32::from(out[1]) / 127.5 - 1.0,
            f32::from(out[2]) / 127.5 - 1.0,
        ];
        let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((length - 1.0).abs() < 0.015);
        assert_eq!(out[3], 112);
    }

    #[test]
    fn alpha_mask_is_explicitly_rejected_without_coverage_correction() {
        let source = [0u8; 16];
        let mut out = [0x6D; 4];
        assert_eq!(
            downsample_rgba8_level_into(
                &source,
                2,
                2,
                CpuTextureMipSemantic::AlphaMask { cutoff: 0.5 },
                &mut out
            ),
            Err(CpuTextureMipError::AlphaCoverageCorrectionUnavailable)
        );
        assert_eq!(out, [0x6D; 4]);
        assert_eq!(
            downsample_rgba8_level_into(
                &source,
                2,
                2,
                CpuTextureMipSemantic::AlphaMask { cutoff: f32::NAN },
                &mut out
            ),
            Err(CpuTextureMipError::InvalidAlphaCutoff)
        );
    }
}
