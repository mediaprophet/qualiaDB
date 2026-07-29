//! Ratio of energy between two frequency bands.

use crate::features::spectral::energy_band::energy_band;
use crate::types::AudioError;

/// Ratio of the spectral energy in band 1 to that in band 2.
///
/// Both bands are evaluated over the one-sided magnitude spectrum `mag` with
/// [`energy_band`](crate::features::spectral::energy_band): the numerator is the
/// energy in `[f_lo1, f_hi1]` Hz and the denominator the energy in
/// `[f_lo2, f_hi2]` Hz. The result is `E1 / E2`, a scale-invariant balance
/// measure (e.g. high-band vs low-band brightness).
///
/// Zero-heap: two passes, scalar result.
///
/// # Errors
/// - Any error from [`energy_band`] for either band (bad `sample_rate`, too few
///   bins, or malformed frequency bounds).
/// - [`AudioError::InvalidParameter`] if the denominator band has zero energy
///   (the ratio would be undefined).
#[allow(clippy::too_many_arguments)]
pub fn energy_band_ratio(
    mag: &[f32],
    sample_rate: f32,
    f_lo1: f32,
    f_hi1: f32,
    f_lo2: f32,
    f_hi2: f32,
) -> Result<f32, AudioError> {
    let e1 = energy_band(mag, sample_rate, f_lo1, f_hi1)?;
    let e2 = energy_band(mag, sample_rate, f_lo2, f_hi2)?;
    if e2 == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    Ok(e1 / e2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ratio() {
        let sr = 8000.0f32; // 9 bins, bin_hz = 500
        let mut mag = [0.0f32; 9];
        mag[1] = 2.0; // 500 Hz  (low band)  -> E = 4
        mag[6] = 4.0; // 3000 Hz (high band) -> E = 16
                      // high/low = 16/4 = 4.0
        let r = energy_band_ratio(&mag, sr, 2500.0, 3500.0, 250.0, 750.0).expect("ratio");
        assert!((r - 4.0).abs() < 1e-4, "ratio={r}");
    }

    #[test]
    fn unity_when_bands_match() {
        let sr = 8000.0f32;
        let mut mag = [0.0f32; 9];
        mag[2] = 3.0; // 1000 Hz
        mag[6] = 3.0; // 3000 Hz
        let r = energy_band_ratio(&mag, sr, 900.0, 1100.0, 2900.0, 3100.0).expect("ratio");
        assert!((r - 1.0).abs() < 1e-4, "ratio={r}");
    }

    #[test]
    fn rejects_zero_denominator() {
        let sr = 8000.0f32;
        let mut mag = [0.0f32; 9];
        mag[1] = 2.0; // only low band has energy
        assert_eq!(
            energy_band_ratio(&mag, sr, 250.0, 750.0, 2500.0, 3500.0),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn propagates_band_error() {
        let mag = [1.0f32; 9];
        assert_eq!(
            energy_band_ratio(&mag, 0.0, 0.0, 100.0, 200.0, 300.0),
            Err(AudioError::InvalidParameter)
        );
    }
}
