//! Log-spectral distance (LSD) between a reference and a degraded magnitude
//! spectrum.
//!
//! LSD is the classic intrusive spectral-envelope error: the root-mean-square,
//! over frequency bins, of the difference between the two log-magnitude spectra,
//! expressed in dB. Zero for identical spectra; grows with envelope mismatch.
//!
//! This operates on already-computed magnitude spectra (e.g. from
//! [`crate::features::fft::real_fft_magnitude`]) so it stays pure, allocation-free
//! and reusable per-frame. Caller supplies both magnitude slices; a single scalar
//! is returned.

use crate::types::AudioError;

/// Log-spectral distance in dB between two magnitude spectra.
///
/// - `reference_mag`: reference magnitude spectrum (`|X_ref[k]|`, linear, ≥ 0).
/// - `degraded_mag`: degraded magnitude spectrum, same number of bins.
///
/// Each bin is converted to a power dB value `10*log10(mag^2 + eps)` (equivalently
/// `20*log10(mag)` with a small floor to bound silence), and the result is
/// `sqrt(mean_k (dB_ref[k] - dB_deg[k])^2)`.
///
/// Returns [`AudioError::InvalidParameter`] if the slices are empty or of
/// differing length.
pub fn log_spectral_distance(
    reference_mag: &[f32],
    degraded_mag: &[f32],
) -> Result<f32, AudioError> {
    if reference_mag.is_empty() || reference_mag.len() != degraded_mag.len() {
        return Err(AudioError::InvalidParameter);
    }

    // Power floor: bins below this are treated as silence, bounding the log so a
    // zero-vs-zero comparison is 0 dB rather than -inf. -100 dB-ish floor.
    const POWER_EPS: f32 = 1.0e-10;

    let mut acc = 0.0f32;
    for k in 0..reference_mag.len() {
        let pr = reference_mag[k] * reference_mag[k] + POWER_EPS;
        let pd = degraded_mag[k] * degraded_mag[k] + POWER_EPS;
        let db_ref = 10.0 * pr.log10();
        let db_deg = 10.0 * pd.log10();
        let diff = db_ref - db_deg;
        acc += diff * diff;
    }

    Ok((acc / reference_mag.len() as f32).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_spectra_are_zero() {
        let mag = [0.0, 1.0, 4.0, 2.0, 0.5, 0.0, 3.0, 1.5];
        let lsd = log_spectral_distance(&mag, &mag).expect("valid");
        assert!(lsd.abs() < 1e-3, "identical LSD should be ~0, got {lsd}");
    }

    #[test]
    fn different_spectra_are_positive_and_grow() {
        let a = [1.0f32, 2.0, 3.0, 4.0];
        // A 6 dB-per-bin level offset (factor 2 in magnitude ≈ 6.02 dB power).
        let b: Vec<f32> = a.iter().map(|&v| v * 2.0).collect();
        let lsd_small = log_spectral_distance(&a, &b).expect("valid");
        assert!(lsd_small > 0.0, "different spectra must be positive");

        // A bigger offset (factor 8 ≈ 18 dB) must yield a larger distance.
        let c: Vec<f32> = a.iter().map(|&v| v * 8.0).collect();
        let lsd_big = log_spectral_distance(&a, &c).expect("valid");
        assert!(
            lsd_big > lsd_small,
            "larger spectral mismatch {lsd_big} must exceed smaller {lsd_small}"
        );

        // A uniform factor-of-2 offset is ~6.02 dB on every bin → RMS ≈ 6.02.
        assert!(
            (lsd_small - 6.02).abs() < 0.2,
            "uniform 6 dB offset LSD ≈ 6.02, got {lsd_small}"
        );
    }

    #[test]
    fn rejects_bad_shapes() {
        assert_eq!(
            log_spectral_distance(&[], &[]),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            log_spectral_distance(&[1.0, 2.0], &[1.0]),
            Err(AudioError::InvalidParameter)
        );
    }
}
