//! Real 2-D medical-image signal processing over a caller-provided intensity grid.
//!
//! HONESTY (CLAUDE.md §15): this module performs genuine, testable digital signal
//! processing — intensity statistics, histogram, window/level normalization,
//! threshold segmentation, and a Sobel edge-magnitude map. It computes **metrics**,
//! not interpretations. Nothing here is a diagnosis, a radiological reading, or a
//! clinical finding; every result carries [`IMAGING_EPISTEMIC_STATUS`] to say so.
//! Statistical primitives delegate to `crate::solvers::statistics` per project rule.

use super::MedicalError;
use crate::solvers::statistics::{descriptive, histogram};

/// Honest epistemic label stamped on every [`ImageAnalysisResult`].
pub const IMAGING_EPISTEMIC_STATUS: &str = "Signal-processing metrics only \
(intensity statistics, histogram, window/level, threshold segmentation, Sobel edge \
magnitude). NOT a diagnosis, radiological reading, or clinical finding.";

/// How the segmentation threshold is chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentationThreshold {
    /// Otsu's method — threshold maximising between-class variance of the intensities.
    Otsu,
    /// Caller-fixed intensity threshold.
    Fixed(f64),
}

/// Result of processing one intensity grid. All fields are computed from the input;
/// none is fabricated. Foreground of `mask` is defined as `intensity > threshold`.
#[derive(Debug, Clone)]
pub struct ImageAnalysisResult {
    /// Honest label — DSP metrics only, never a clinical interpretation.
    pub epistemic_status: &'static str,
    pub width: usize,
    pub height: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    /// Population standard deviation of the intensities.
    pub std_dev: f64,
    /// Equal-width histogram counts (length = requested bin count).
    pub histogram: Vec<u32>,
    pub hist_min: f64,
    pub hist_max: f64,
    pub hist_bin_width: f64,
    /// Threshold actually used for segmentation (Otsu-derived or caller-fixed).
    pub threshold: f64,
    /// Binary segmentation mask, row-major, `true` where `intensity > threshold`.
    pub mask: Vec<bool>,
    /// Number of foreground pixels (`true` entries in `mask`).
    pub segmented_area: usize,
    /// Mean intensity over the foreground region (0.0 if the region is empty).
    pub segmented_mean_intensity: f64,
    /// Per-pixel Sobel edge-gradient magnitude, row-major (length = width*height).
    pub sobel_magnitude: Vec<f64>,
    /// Window/level normalized intensities in [0,1], row-major.
    pub windowed: Vec<f64>,
}

/// Process a caller-provided row-major intensity grid.
///
/// * `data` — row-major intensities, `len == width * height`.
/// * `bins` — histogram bin count (must be ≥ 1).
/// * `threshold` — Otsu or a fixed value.
/// * `window` — `Some((level, width))` for window/level mapping, else full-range normalize.
///
/// Returns [`MedicalError::ValidationError`] on dimension mismatch, empty/zero dims,
/// non-finite samples, or `bins == 0`. Never fabricates: the output is purely a
/// function of the input.
pub fn analyze_intensity_grid(
    data: &[f64],
    width: usize,
    height: usize,
    bins: usize,
    threshold: SegmentationThreshold,
    window: Option<(f64, f64)>,
) -> Result<ImageAnalysisResult, MedicalError> {
    if width == 0 || height == 0 {
        return Err(MedicalError::ValidationError(
            "image dimensions must be non-zero".to_string(),
        ));
    }
    if data.len() != width * height {
        return Err(MedicalError::ValidationError(format!(
            "data length {} does not match width*height = {}",
            data.len(),
            width * height
        )));
    }
    if bins == 0 {
        return Err(MedicalError::ValidationError(
            "histogram bin count must be >= 1".to_string(),
        ));
    }
    if data.iter().any(|v| !v.is_finite()) {
        return Err(MedicalError::ValidationError(
            "image contains non-finite intensities".to_string(),
        ));
    }

    // -- Intensity statistics (delegated to solvers::statistics) --------------
    let min = descriptive::min(data).unwrap();
    let max = descriptive::max(data).unwrap();
    let mean = descriptive::mean(data).unwrap();
    // Population std (sample = false): the grid is the whole population of pixels.
    let std_dev = descriptive::std_dev(data, false).unwrap();

    // -- Histogram (delegated) ------------------------------------------------
    let mut counts = vec![0u32; bins];
    let (hist_min, hist_max, hist_bin_width) = match histogram::histogram_into(data, &mut counts) {
        Some(r) => (r.min, r.max, r.bin_width),
        None => (min, max, 0.0),
    };

    // -- Threshold ------------------------------------------------------------
    let threshold = match threshold {
        SegmentationThreshold::Fixed(t) => t,
        SegmentationThreshold::Otsu => otsu_threshold(data, min, max),
    };

    // -- Segmentation mask + region metrics -----------------------------------
    let mask: Vec<bool> = data.iter().map(|&v| v > threshold).collect();
    let segmented_area = mask.iter().filter(|&&b| b).count();
    let seg_sum: f64 = data
        .iter()
        .zip(mask.iter())
        .filter(|(_, &m)| m)
        .map(|(&v, _)| v)
        .sum();
    let segmented_mean_intensity = if segmented_area > 0 {
        seg_sum / segmented_area as f64
    } else {
        0.0
    };

    // -- Sobel edge magnitude -------------------------------------------------
    let sobel_magnitude = sobel_magnitude(data, width, height);

    // -- Window/level normalization -------------------------------------------
    let windowed = window_level(data, min, max, window);

    Ok(ImageAnalysisResult {
        epistemic_status: IMAGING_EPISTEMIC_STATUS,
        width,
        height,
        min,
        max,
        mean,
        std_dev,
        histogram: counts,
        hist_min,
        hist_max,
        hist_bin_width,
        threshold,
        mask,
        segmented_area,
        segmented_mean_intensity,
        sobel_magnitude,
        windowed,
    })
}

/// Otsu's threshold over a 256-level histogram of `[min, max]`. Returns an
/// intensity value; for a degenerate (all-equal) image returns `min`.
fn otsu_threshold(data: &[f64], min: f64, max: f64) -> f64 {
    if max <= min {
        return min;
    }
    const L: usize = 256;
    let scale = (L as f64 - 1.0) / (max - min);
    let mut hist = [0usize; L];
    for &v in data {
        let b = (((v - min) * scale).round() as usize).min(L - 1);
        hist[b] += 1;
    }
    let total = data.len() as f64;
    let sum_all: f64 = (0..L).map(|i| i as f64 * hist[i] as f64).sum();

    let mut w_b = 0.0;
    let mut sum_b = 0.0;
    let mut max_between = -1.0;
    let mut thr_bin = 0usize;
    for i in 0..L {
        w_b += hist[i] as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += i as f64 * hist[i] as f64;
        let m_b = sum_b / w_b;
        let m_f = (sum_all - sum_b) / w_f;
        let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if between > max_between {
            max_between = between;
            thr_bin = i;
        }
    }
    min + thr_bin as f64 / scale
}

/// 3×3 Sobel gradient magnitude with replicate (clamp) border padding.
fn sobel_magnitude(data: &[f64], width: usize, height: usize) -> Vec<f64> {
    let at = |r: isize, c: isize| -> f64 {
        let rr = r.clamp(0, height as isize - 1) as usize;
        let cc = c.clamp(0, width as isize - 1) as usize;
        data[rr * width + cc]
    };
    let mut out = vec![0.0f64; width * height];
    for r in 0..height as isize {
        for c in 0..width as isize {
            // Gx kernel [[-1,0,1],[-2,0,2],[-1,0,1]]
            let gx = -at(r - 1, c - 1) + at(r - 1, c + 1) - 2.0 * at(r, c - 1) + 2.0 * at(r, c + 1)
                - at(r + 1, c - 1)
                + at(r + 1, c + 1);
            // Gy kernel [[-1,-2,-1],[0,0,0],[1,2,1]]
            let gy = -at(r - 1, c - 1) - 2.0 * at(r - 1, c) - at(r - 1, c + 1)
                + at(r + 1, c - 1)
                + 2.0 * at(r + 1, c)
                + at(r + 1, c + 1);
            out[r as usize * width + c as usize] = (gx * gx + gy * gy).sqrt();
        }
    }
    out
}

/// Window/level normalization into [0,1]. With `Some((level, width))` the visible
/// window is `[level - width/2, level + width/2]`; otherwise the full `[min,max]`
/// range is used. A degenerate range maps everything to 0.
fn window_level(data: &[f64], min: f64, max: f64, window: Option<(f64, f64)>) -> Vec<f64> {
    let (lo, span) = match window {
        Some((level, width)) => (level - width / 2.0, width),
        None => (min, max - min),
    };
    data.iter()
        .map(|&v| {
            if span <= 0.0 || !span.is_finite() {
                0.0
            } else {
                ((v - lo) / span).clamp(0.0, 1.0)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dimension_mismatch() {
        let r = analyze_intensity_grid(&[0.0; 3], 2, 2, 4, SegmentationThreshold::Otsu, None);
        assert!(matches!(r, Err(MedicalError::ValidationError(_))));
    }

    #[test]
    fn otsu_between_two_levels() {
        // 4x4 step edge: left two cols = 0, right two cols = 100.
        let data: Vec<f64> = (0..16)
            .map(|i| if i % 4 >= 2 { 100.0 } else { 0.0 })
            .collect();
        let r = analyze_intensity_grid(&data, 4, 4, 8, SegmentationThreshold::Otsu, None).unwrap();
        // Otsu split point lies below the bright level, so the 8 bright pixels segment out.
        // (For a perfectly bimodal {0,100} image the optimal split bin is the background
        // level itself, i.e. threshold == 0.0; foreground = intensity > threshold.)
        assert!(r.threshold >= 0.0 && r.threshold < 100.0);
        assert_eq!(r.segmented_area, 8);
    }
}
