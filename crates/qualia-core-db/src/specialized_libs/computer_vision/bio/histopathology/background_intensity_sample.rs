//! Sample bright background pixels (percentile) for RGB illumination correction.

use super::HistoError;

/// Per-channel background estimate (linear RGB8 mean of bright pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbBg {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    /// Number of pixels at or above the intensity percentile threshold.
    pub n_samples: usize,
}

/// Sample background as the mean RGB of pixels whose luminance is at or above
/// the given percentile (e.g. `percentile = 95.0` → top ~5% brightest).
///
/// Uses a 256-bin luminance histogram (BT.601 integer) so the hot path is
/// stack-only; no heap.
pub fn background_intensity_sample(rgb: &[u8], percentile: f32) -> Result<RgbBg, HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if !(0.0..=100.0).contains(&percentile) {
        return Err(HistoError::InvalidParameter);
    }
    let n = rgb.len() / 3;
    // Luminance histogram.
    let mut hist = [0u32; 256];
    for i in 0..n {
        let base = i * 3;
        let r = rgb[base] as u32;
        let g = rgb[base + 1] as u32;
        let b = rgb[base + 2] as u32;
        let y = ((77 * r + 150 * g + 29 * b) / 256) as usize;
        hist[y.min(255)] += 1;
    }
    // Threshold: smallest Y such that cumulative from top ≥ (100-percentile)% of pixels.
    // "percentile=95" → keep pixels with Y ≥ Y95 (top 5%).
    let want = ((100.0 - percentile).max(0.0) / 100.0 * n as f32).ceil() as u32;
    let want = want.max(1);
    let mut acc = 0u32;
    let mut thresh = 0u8;
    for y in (0..256).rev() {
        acc += hist[y];
        if acc >= want {
            thresh = y as u8;
            break;
        }
    }
    // Mean RGB of pixels with Y ≥ thresh.
    let mut sum = [0.0f64; 3];
    let mut count = 0usize;
    for i in 0..n {
        let base = i * 3;
        let r = rgb[base];
        let g = rgb[base + 1];
        let b = rgb[base + 2];
        let y = ((77u32 * r as u32 + 150 * g as u32 + 29 * b as u32) / 256) as u8;
        if y >= thresh {
            sum[0] += r as f64;
            sum[1] += g as f64;
            sum[2] += b as f64;
            count += 1;
        }
    }
    if count == 0 {
        return Err(HistoError::DegenerateData);
    }
    Ok(RgbBg {
        r: (sum[0] / count as f64) as f32,
        g: (sum[1] / count as f64) as f32,
        b: (sum[2] / count as f64) as f32,
        n_samples: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_bright_corner() {
        // Dark tissue + bright corner.
        let mut rgb = vec![30u8; 8 * 8 * 3];
        // Last 4 pixels pure white.
        for i in (8 * 8 - 4)..(8 * 8) {
            let base = i * 3;
            rgb[base] = 250;
            rgb[base + 1] = 252;
            rgb[base + 2] = 248;
        }
        let bg = background_intensity_sample(&rgb, 95.0).unwrap();
        assert!(bg.n_samples >= 1);
        assert!(bg.r > 200.0 && bg.g > 200.0 && bg.b > 200.0);
    }

    #[test]
    fn rejects_bad_percentile() {
        let rgb = [255u8, 255, 255];
        assert_eq!(
            background_intensity_sample(&rgb, 120.0),
            Err(HistoError::InvalidParameter)
        );
    }
}
