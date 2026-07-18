//! Sum spectral energy into arbitrary caller-supplied frequency bands.

use crate::types::AudioError;

/// Sum the energy of a one-sided magnitude spectrum `mag` into arbitrary
/// frequency bands defined by ascending `band_edges_hz`.
///
/// `band_edges_hz` holds `B + 1` ascending edge frequencies (Hz) partitioning
/// the spectrum into `B` bands. Band `b` covers `[edge[b], edge[b+1])`
/// (half-open, so a bin lands in exactly one band). `out_energy[b]` receives
/// `Σ |X[k]|^2` over the bins whose centre frequency falls in band `b`.
///
/// `mag` spans DC..Nyquist inclusive, so bin `k` sits at
/// `k · sample_rate / (2·(N-1))` Hz for an `N`-bin spectrum.
///
/// Zero-heap: single pass, caller-supplied output.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate` is not a positive finite,
///   if `mag` has fewer than 2 bins, if there are fewer than 2 edges, or if the
///   edges are not strictly ascending / not finite.
/// - [`AudioError::OutputBufferTooSmall`] if `out_energy` is shorter than the
///   number of bands.
pub fn frequency_bands(
    mag: &[f32],
    sample_rate: f32,
    band_edges_hz: &[f32],
    out_energy: &mut [f32],
) -> Result<usize, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    if mag.len() < 2 || band_edges_hz.len() < 2 {
        return Err(AudioError::InvalidParameter);
    }
    let bands = band_edges_hz.len() - 1;
    if out_energy.len() < bands {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for w in band_edges_hz.windows(2) {
        if !w[0].is_finite() || !w[1].is_finite() || w[0] >= w[1] {
            return Err(AudioError::InvalidParameter);
        }
    }

    for e in out_energy.iter_mut().take(bands) {
        *e = 0.0;
    }
    let bin_hz = sample_rate as f64 / (2.0 * (mag.len() - 1) as f64);
    for (k, m) in mag.iter().enumerate() {
        let f = k as f64 * bin_hz;
        // Locate the half-open band [edge[b], edge[b+1]) containing f.
        for b in 0..bands {
            if f >= band_edges_hz[b] as f64 && f < band_edges_hz[b + 1] as f64 {
                out_energy[b] += (*m as f64 * *m as f64) as f32;
                break;
            }
        }
    }
    Ok(bands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_energy_across_bands() {
        let sr = 8000.0f32; // 9 bins, bin_hz = 500. Bins: 0,500,...,4000
        let mut mag = [0.0f32; 9];
        mag[1] = 2.0; // 500 Hz  -> band 0
        mag[3] = 3.0; // 1500 Hz -> band 1
        mag[7] = 1.0; // 3500 Hz -> band 2
        let edges = [0.0f32, 1000.0, 2000.0, 4000.0];
        let mut out = [0.0f32; 3];
        let n = frequency_bands(&mag, sr, &edges, &mut out).expect("bands");
        assert_eq!(n, 3);
        assert!((out[0] - 4.0).abs() < 1e-4, "band0={}", out[0]); // 2^2
        assert!((out[1] - 9.0).abs() < 1e-4, "band1={}", out[1]); // 3^2
        assert!((out[2] - 1.0).abs() < 1e-4, "band2={}", out[2]); // 1^2, 4000 excluded (half-open)
    }

    #[test]
    fn half_open_bins_land_once() {
        let sr = 8000.0f32;
        let mag = [1.0f32; 9]; // energy 1 at every bin
        // Two adjacent bands; the shared edge (1000 Hz) belongs to the upper band.
        let edges = [0.0f32, 1000.0, 5000.0];
        let mut out = [0.0f32; 2];
        frequency_bands(&mag, sr, &edges, &mut out).expect("bands");
        // Band 0: bins at 0,500 -> 2 bins. Band 1: 1000..4000 -> 7 bins.
        assert!((out[0] - 2.0).abs() < 1e-4, "band0={}", out[0]);
        assert!((out[1] - 7.0).abs() < 1e-4, "band1={}", out[1]);
    }

    #[test]
    fn rejects_bad_edges() {
        let mag = [1.0f32; 9];
        let mut out = [0.0f32; 2];
        assert_eq!(
            frequency_bands(&mag, 8000.0, &[1000.0f32, 500.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let mag = [1.0f32; 9];
        let edges = [0.0f32, 1000.0, 2000.0];
        let mut out = [0.0f32; 1];
        assert_eq!(
            frequency_bands(&mag, 8000.0, &edges, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
