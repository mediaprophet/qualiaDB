//! Energy contained in a `[f_lo, f_hi]` frequency band.

use crate::types::AudioError;

/// Spectral energy of a one-sided magnitude spectrum `mag` within the frequency
/// band `[f_lo, f_hi]` (Hz, inclusive).
///
/// `mag` spans DC..Nyquist inclusive, so bin `k` sits at
/// `k · sample_rate / (2·(N-1))` Hz for an `N`-bin spectrum. Energy is the sum
/// of squared magnitudes `Σ |X[k]|^2` over the bins whose centre frequency
/// falls in `[f_lo, f_hi]`.
///
/// Zero-heap: single pass, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate` is not a positive finite,
///   if `mag` has fewer than 2 bins, if `f_lo` is negative, or if
///   `f_hi < f_lo`.
pub fn energy_band(mag: &[f32], sample_rate: f32, f_lo: f32, f_hi: f32) -> Result<f32, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    if mag.len() < 2 {
        return Err(AudioError::InvalidParameter);
    }
    if f_lo < 0.0 || f_hi < f_lo || !f_lo.is_finite() || !f_hi.is_finite() {
        return Err(AudioError::InvalidParameter);
    }

    let bin_hz = sample_rate as f64 / (2.0 * (mag.len() - 1) as f64);
    let mut acc = 0.0f64;
    for (k, m) in mag.iter().enumerate() {
        let f = k as f64 * bin_hz;
        if f >= f_lo as f64 && f <= f_hi as f64 {
            acc += (*m as f64) * (*m as f64);
        }
    }
    Ok(acc as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_only_in_band() {
        let sr = 8000.0f32;
        // 9 bins -> fft 16 -> bin_hz = 8000/16 = 500 Hz. Bins at 0,500,...,4000.
        let mut mag = [0.0f32; 9];
        mag[2] = 2.0; // 1000 Hz -> in band
        mag[4] = 3.0; // 2000 Hz -> in band
        mag[7] = 5.0; // 3500 Hz -> out of band
                      // Band [900, 2100] Hz captures bins 2 and 4 -> 4 + 9 = 13.
        let e = energy_band(&mag, sr, 900.0, 2100.0).expect("energy");
        assert!((e - 13.0).abs() < 1e-4, "energy={e}");
    }

    #[test]
    fn full_band_is_total_energy() {
        let sr = 8000.0f32;
        let mag = [1.0f32, 1.0, 1.0, 1.0, 1.0];
        let e = energy_band(&mag, sr, 0.0, 4000.0).expect("energy");
        assert!((e - 5.0).abs() < 1e-4, "energy={e}");
    }

    #[test]
    fn empty_band_is_zero() {
        let sr = 8000.0f32;
        let mag = [1.0f32; 5];
        // Sub-bin window between bins -> no bin centre inside -> 0.
        let e = energy_band(&mag, sr, 100.0, 200.0).expect("energy");
        assert!(e.abs() < 1e-6, "energy={e}");
    }

    #[test]
    fn rejects_bad_params() {
        let mag = [1.0f32; 5];
        assert_eq!(
            energy_band(&mag, 0.0, 0.0, 100.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            energy_band(&mag, 8000.0, 200.0, 100.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            energy_band(&[1.0], 8000.0, 0.0, 100.0),
            Err(AudioError::InvalidParameter)
        );
    }
}
