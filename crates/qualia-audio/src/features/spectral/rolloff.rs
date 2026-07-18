//! Spectral rolloff frequency of a magnitude spectrum.

use crate::types::AudioError;

/// Spectral rolloff frequency of a one-sided magnitude spectrum `mag`.
///
/// The rolloff is the frequency below which a fraction `threshold` (e.g. `0.85`)
/// of the total spectral energy `Σ |X[k]|^2` is contained. The returned value
/// is the centre frequency of the first bin at which the running cumulative
/// energy reaches `threshold · total`.
///
/// `mag` spans DC..Nyquist inclusive (an `N`-bin spectrum came from an FFT of
/// size `2·(N-1)`), so the bin spacing is `sample_rate / (2·(N-1))` Hz.
///
/// A spectrum whose energy sits low returns a low frequency; energy sitting
/// high returns a high frequency. A silent spectrum returns `0.0` Hz.
///
/// Zero-heap: two passes, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate` is not a positive finite,
///   if `threshold` is not in `(0, 1]`, or if `mag` has fewer than 2 bins.
pub fn spectral_rolloff(
    mag: &[f32],
    sample_rate: f32,
    threshold: f32,
) -> Result<f32, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    if !(threshold > 0.0 && threshold <= 1.0) {
        return Err(AudioError::InvalidParameter);
    }
    if mag.len() < 2 {
        return Err(AudioError::InvalidParameter);
    }

    let mut total = 0.0f64;
    for m in mag {
        total += (*m as f64) * (*m as f64);
    }
    if total == 0.0 {
        return Ok(0.0);
    }

    let bin_hz = sample_rate as f64 / (2.0 * (mag.len() - 1) as f64);
    let target = threshold as f64 * total;
    let mut cum = 0.0f64;
    for (k, m) in mag.iter().enumerate() {
        cum += (*m as f64) * (*m as f64);
        if cum >= target {
            return Ok((k as f64 * bin_hz) as f32);
        }
    }
    // Numerical fall-through: all energy accounted for at the last bin.
    Ok(((mag.len() - 1) as f64 * bin_hz) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_energy_low_rolloff_high_energy_high_rolloff() {
        let sr = 44_100.0f32;
        let n = 513usize; // fft 1024, bin_hz = 44100/1024 ~= 43.07 Hz
        let bin_hz = sr / 1024.0;

        // Energy concentrated in the low bins.
        let mut low = vec![0.0f32; n];
        for m in low.iter_mut().take(10) {
            *m = 1.0;
        }
        let r_low = spectral_rolloff(&low, sr, 0.85).expect("rolloff");

        // Energy concentrated in the high bins.
        let mut high = vec![0.0f32; n];
        for m in high.iter_mut().skip(n - 10) {
            *m = 1.0;
        }
        let r_high = spectral_rolloff(&high, sr, 0.85).expect("rolloff");

        assert!(r_low < r_high, "low={r_low} high={r_high}");
        // Low spectrum rolls off within its first ~10 bins.
        assert!(r_low <= 10.0 * bin_hz, "r_low={r_low}");
        // High spectrum rolls off up in the last decile of the band.
        assert!(r_high > 400.0 * bin_hz, "r_high={r_high}");
    }

    #[test]
    fn full_threshold_reaches_last_energetic_bin() {
        let sr = 8000.0f32;
        let mag = [0.0f32, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let bin_hz = sr / (2.0 * 8.0); // 500 Hz
        // threshold 1.0 -> must include the last non-zero bin (index 8).
        let r = spectral_rolloff(&mag, sr, 1.0).expect("rolloff");
        assert!((r - 8.0 * bin_hz).abs() < 1e-3, "r={r}");
    }

    #[test]
    fn silent_spectrum_is_zero() {
        let mag = [0.0f32; 8];
        assert_eq!(spectral_rolloff(&mag, 8000.0, 0.85), Ok(0.0));
    }

    #[test]
    fn rejects_bad_params() {
        let mag = [0.0f32, 1.0, 0.0];
        assert_eq!(
            spectral_rolloff(&mag, 0.0, 0.85),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            spectral_rolloff(&mag, 8000.0, 0.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            spectral_rolloff(&mag, 8000.0, 1.5),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            spectral_rolloff(&[1.0], 8000.0, 0.85),
            Err(AudioError::InvalidParameter)
        );
    }
}
