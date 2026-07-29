//! Spectral complexity — count of significant peaks in a magnitude spectrum.

use crate::features::peaks::detect_peaks;
use crate::types::AudioError;

/// Spectral complexity of a one-sided magnitude spectrum `mag`.
///
/// Complexity is the number of significant spectral peaks: local maxima at or
/// above `threshold`, no two closer than `min_distance` bins (the weaker of a
/// too-close pair is dropped). A pure tone yields ~1; a dense, noisy, or
/// polyphonic spectrum yields many. Implemented over
/// [`detect_peaks`](crate::features::peaks::detect_peaks).
///
/// `scratch_pos` / `scratch_mag` are caller-supplied working buffers (they
/// receive the peak positions and magnitudes as a side effect); their common
/// length bounds how many peaks can be counted.
///
/// Zero-heap: all storage is caller-supplied.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `threshold` is not finite.
/// - [`AudioError::OutputBufferTooSmall`] if more significant peaks are present
///   than the scratch buffers can hold.
pub fn spectral_complexity(
    mag: &[f32],
    threshold: f32,
    min_distance: usize,
    scratch_pos: &mut [f32],
    scratch_mag: &mut [f32],
) -> Result<usize, AudioError> {
    detect_peaks(mag, threshold, min_distance, scratch_pos, scratch_mag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_tone_low_complexity() {
        // One resolved peak in an otherwise quiet spectrum.
        let mut mag = [0.0f32; 32];
        mag[9] = 0.5;
        mag[10] = 1.0;
        mag[11] = 0.5;
        let mut pos = [0.0f32; 8];
        let mut mg = [0.0f32; 8];
        let c = spectral_complexity(&mag, 0.25, 1, &mut pos, &mut mg).expect("complexity");
        assert_eq!(c, 1, "complexity={c}");
    }

    #[test]
    fn polyphonic_higher_complexity() {
        // Three well-separated peaks.
        let mut mag = [0.0f32; 32];
        for c in [5usize, 15, 25] {
            mag[c - 1] = 0.4;
            mag[c] = 1.0;
            mag[c + 1] = 0.4;
        }
        let mut pos = [0.0f32; 8];
        let mut mg = [0.0f32; 8];
        let c = spectral_complexity(&mag, 0.25, 1, &mut pos, &mut mg).expect("complexity");
        assert_eq!(c, 3, "complexity={c}");
    }

    #[test]
    fn threshold_suppresses_weak_peaks() {
        let mut mag = [0.0f32; 32];
        mag[5] = 0.1; // weak
        mag[15] = 1.0; // strong
        let mut pos = [0.0f32; 8];
        let mut mg = [0.0f32; 8];
        let c = spectral_complexity(&mag, 0.5, 1, &mut pos, &mut mg).expect("complexity");
        assert_eq!(c, 1, "complexity={c}");
    }

    #[test]
    fn rejects_nan_threshold() {
        let mag = [0.0f32, 1.0, 0.0];
        let mut pos = [0.0f32; 2];
        let mut mg = [0.0f32; 2];
        assert_eq!(
            spectral_complexity(&mag, f32::NAN, 1, &mut pos, &mut mg),
            Err(AudioError::InvalidParameter)
        );
    }
}
