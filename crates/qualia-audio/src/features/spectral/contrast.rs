//! Spectral contrast — peak-vs-valley level (dB) per sub-band.

use crate::types::AudioError;

/// Spectral contrast per sub-band of a one-sided magnitude spectrum `mag`.
///
/// `band_edges` is an ascending list of bin indices partitioning the spectrum
/// into `band_edges.len() - 1` sub-bands `[edge[b], edge[b+1])`. For each band
/// the contrast is the level difference between the band's spectral peak
/// (maximum magnitude) and its valley (minimum magnitude), expressed in dB:
///
/// ```text
/// contrast_dB[b] = 20·log10(peak_b) - 20·log10(valley_b)
/// ```
///
/// A flat band gives ~`0 dB`; a band with a sharp peak over a quiet floor gives
/// a large positive dB. Magnitudes are floored to a tiny epsilon so an all-zero
/// band yields `0 dB` rather than a `NaN`. Results are written into
/// `out_contrast` (one per band).
///
/// Zero-heap: single pass per band, caller-supplied output.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if there are fewer than 2 edges, if the
///   edges are not strictly ascending, or if any edge exceeds `mag.len()`.
/// - [`AudioError::OutputBufferTooSmall`] if `out_contrast` is shorter than the
///   number of bands.
pub fn spectral_contrast(
    mag: &[f32],
    band_edges: &[usize],
    out_contrast: &mut [f32],
) -> Result<usize, AudioError> {
    if band_edges.len() < 2 {
        return Err(AudioError::InvalidParameter);
    }
    let bands = band_edges.len() - 1;
    if out_contrast.len() < bands {
        return Err(AudioError::OutputBufferTooSmall);
    }
    // Validate strictly ascending edges within bounds.
    for w in band_edges.windows(2) {
        if w[0] >= w[1] || w[1] > mag.len() {
            return Err(AudioError::InvalidParameter);
        }
    }

    const EPS: f32 = 1e-10;
    for b in 0..bands {
        let lo = band_edges[b];
        let hi = band_edges[b + 1];
        let mut peak = f32::MIN;
        let mut valley = f32::MAX;
        for &m in &mag[lo..hi] {
            let v = m.abs();
            if v > peak {
                peak = v;
            }
            if v < valley {
                valley = v;
            }
        }
        let peak_db = 20.0 * peak.max(EPS).log10();
        let valley_db = 20.0 * valley.max(EPS).log10();
        out_contrast[b] = peak_db - valley_db;
    }
    Ok(bands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_band_near_zero_contrast() {
        let mag = [1.0f32; 8];
        let edges = [0usize, 8];
        let mut out = [0.0f32; 1];
        let n = spectral_contrast(&mag, &edges, &mut out).expect("contrast");
        assert_eq!(n, 1);
        assert!(out[0].abs() < 1e-3, "contrast={}", out[0]);
    }

    #[test]
    fn peaky_band_large_contrast() {
        // Peak 1.0 over a 0.01 floor -> 20*log10(100) = 40 dB.
        let mag = [0.01f32, 0.01, 1.0, 0.01, 0.01, 0.01, 0.01, 0.01];
        let edges = [0usize, 8];
        let mut out = [0.0f32; 1];
        spectral_contrast(&mag, &edges, &mut out).expect("contrast");
        assert!((out[0] - 40.0).abs() < 0.5, "contrast={}", out[0]);
    }

    #[test]
    fn multiple_bands_independently() {
        // Band 0 flat, band 1 peaky.
        let mag = [1.0f32, 1.0, 1.0, 1.0, 0.01, 1.0, 0.01, 0.01];
        let edges = [0usize, 4, 8];
        let mut out = [0.0f32; 2];
        let n = spectral_contrast(&mag, &edges, &mut out).expect("contrast");
        assert_eq!(n, 2);
        assert!(out[0].abs() < 1e-3, "band0={}", out[0]);
        assert!(out[1] > 30.0, "band1={}", out[1]);
    }

    #[test]
    fn rejects_bad_edges() {
        let mag = [1.0f32; 8];
        let mut out = [0.0f32; 2];
        // Not ascending.
        assert_eq!(
            spectral_contrast(&mag, &[4usize, 2], &mut out),
            Err(AudioError::InvalidParameter)
        );
        // Out of bounds.
        assert_eq!(
            spectral_contrast(&mag, &[0usize, 9], &mut out),
            Err(AudioError::InvalidParameter)
        );
        // Too few edges.
        assert_eq!(
            spectral_contrast(&mag, &[0usize], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let mag = [1.0f32; 8];
        let edges = [0usize, 4, 8];
        let mut out = [0.0f32; 1];
        assert_eq!(
            spectral_contrast(&mag, &edges, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
