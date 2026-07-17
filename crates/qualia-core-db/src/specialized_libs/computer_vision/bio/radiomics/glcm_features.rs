//! Gray-Level Co-occurrence Matrix (GLCM) texture features at distance 1.
//!
//! Angles: 0° (horizontal) and 90° (vertical). Gray levels quantized to
//! `levels` bins (typically 16 or 32). Features: contrast, homogeneity,
//! energy (ASM), correlation.

use super::first_order_stats::RadiomicsError;

/// Common quantization levels for radiomics GLCM.
pub const GLCM_LEVELS_16: usize = 16;
pub const GLCM_LEVELS_32: usize = 32;

/// Max supported quantization levels (stack table size).
pub const GLCM_MAX_LEVELS: usize = 32;

/// Haralick-style features averaged over 0° and 90° (symmetric GLCM).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlcmFeatures {
    pub contrast: f64,
    pub homogeneity: f64,
    /// Angular second moment (energy).
    pub energy: f64,
    pub correlation: f64,
    pub levels: usize,
}

/// Compute GLCM features on a 2D gray image at distance 1 with 16 levels (default).
pub fn glcm_features(
    pixels: &[u8],
    width: usize,
    height: usize,
) -> Result<GlcmFeatures, RadiomicsError> {
    glcm_features_d1(pixels, width, height, GLCM_LEVELS_16)
}

/// Compute GLCM features on a 2D gray image at distance 1.
///
/// `pixels` is row-major `width * height` of `u8` intensities.
/// Symmetric GLCM: (i,j) and (j,i) both counted.
/// Returns average of 0° and 90° orientations.
pub fn glcm_features_d1(
    pixels: &[u8],
    width: usize,
    height: usize,
    levels: usize,
) -> Result<GlcmFeatures, RadiomicsError> {
    if width == 0 || height == 0 || pixels.len() < width * height {
        return Err(RadiomicsError::DimensionMismatch);
    }
    if levels < 2 || levels > GLCM_MAX_LEVELS {
        return Err(RadiomicsError::InvalidParameter);
    }

    let mut glcm = [[0.0f64; GLCM_MAX_LEVELS]; GLCM_MAX_LEVELS];
    let mut pair_count = 0.0f64;

    for y in 0..height {
        for x in 0..(width.saturating_sub(1)) {
            let a = quantize(pixels[y * width + x], levels);
            let b = quantize(pixels[y * width + x + 1], levels);
            glcm[a][b] += 1.0;
            glcm[b][a] += 1.0;
            pair_count += 2.0;
        }
    }
    for y in 0..(height.saturating_sub(1)) {
        for x in 0..width {
            let a = quantize(pixels[y * width + x], levels);
            let b = quantize(pixels[(y + 1) * width + x], levels);
            glcm[a][b] += 1.0;
            glcm[b][a] += 1.0;
            pair_count += 2.0;
        }
    }

    if pair_count < 1.0 {
        return Ok(GlcmFeatures {
            contrast: 0.0,
            homogeneity: 1.0,
            energy: 1.0,
            correlation: 0.0,
            levels,
        });
    }

    for i in 0..levels {
        for j in 0..levels {
            glcm[i][j] /= pair_count;
        }
    }

    let mut mu_i = 0.0f64;
    let mut mu_j = 0.0f64;
    for i in 0..levels {
        for j in 0..levels {
            let p = glcm[i][j];
            mu_i += (i as f64) * p;
            mu_j += (j as f64) * p;
        }
    }
    let mut sigma_i = 0.0f64;
    let mut sigma_j = 0.0f64;
    for i in 0..levels {
        for j in 0..levels {
            let p = glcm[i][j];
            let di = (i as f64) - mu_i;
            let dj = (j as f64) - mu_j;
            sigma_i += di * di * p;
            sigma_j += dj * dj * p;
        }
    }
    sigma_i = sigma_i.sqrt();
    sigma_j = sigma_j.sqrt();

    let mut contrast = 0.0f64;
    let mut homogeneity = 0.0f64;
    let mut energy = 0.0f64;
    let mut corr_num = 0.0f64;

    for i in 0..levels {
        for j in 0..levels {
            let p = glcm[i][j];
            let di = (i as isize - j as isize).unsigned_abs() as f64;
            contrast += di * di * p;
            homogeneity += p / (1.0 + di);
            energy += p * p;
            corr_num += ((i as f64) - mu_i) * ((j as f64) - mu_j) * p;
        }
    }

    let correlation = if sigma_i > 1e-15 && sigma_j > 1e-15 {
        corr_num / (sigma_i * sigma_j)
    } else {
        0.0
    };

    Ok(GlcmFeatures {
        contrast,
        homogeneity,
        energy,
        correlation,
        levels,
    })
}

#[inline]
fn quantize(v: u8, levels: usize) -> usize {
    let idx = (v as usize * levels) / 256;
    idx.min(levels - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_image_high_energy() {
        let img = [128u8; 16];
        let f = glcm_features(&img, 4, 4).unwrap();
        assert!(f.contrast < 1e-9);
        assert!((f.energy - 1.0).abs() < 1e-9);
        assert!((f.homogeneity - 1.0).abs() < 1e-9);
        assert_eq!(f.levels, 16);
    }

    #[test]
    fn checkerboard_has_contrast() {
        let mut img = [0u8; 16];
        for y in 0..4 {
            for x in 0..4 {
                if (x + y) % 2 == 1 {
                    img[y * 4 + x] = 255;
                }
            }
        }
        let f = glcm_features_d1(&img, 4, 4, GLCM_LEVELS_16).unwrap();
        assert!(f.contrast > 0.5);
        assert!(f.energy < 1.0);
    }

    #[test]
    fn levels_32_ok() {
        let img = [10u8, 20, 30, 40, 50, 60, 70, 80, 90];
        let f = glcm_features_d1(&img, 3, 3, GLCM_LEVELS_32).unwrap();
        assert_eq!(f.levels, 32);
        assert!(f.energy > 0.0 && f.energy <= 1.0);
    }

    #[test]
    fn invalid_levels() {
        let img = [0u8; 4];
        assert_eq!(
            glcm_features_d1(&img, 2, 2, 1).unwrap_err(),
            RadiomicsError::InvalidParameter
        );
    }

    #[test]
    fn dim_mismatch() {
        let img = [0u8; 3];
        assert_eq!(
            glcm_features_d1(&img, 2, 2, 16).unwrap_err(),
            RadiomicsError::DimensionMismatch
        );
    }
}
