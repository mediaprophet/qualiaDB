//! True per-bin spectral flux between two consecutive magnitude frames.

use crate::types::AudioError;

/// Half-wave-rectified spectral flux between a previous and current magnitude
/// spectrum.
///
/// Flux is the sum over bins of the positive part of the bin-to-bin change,
/// `Σ_k max(cur[k] - prev[k], 0)`. Only rising energy contributes, so the
/// value is an onset-sensitive novelty: `0` for identical frames and strictly
/// positive when spectral energy increases in any bin. This is the true
/// per-bin flux (contrast with the frame-energy novelty used by `music.rs`).
///
/// `prev` and `cur` must be the same non-zero length (both one-sided magnitude
/// spectra of the same FFT size).
///
/// Zero-heap: single pass, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the slices are empty or differ in length.
pub fn spectral_flux(prev: &[f32], cur: &[f32]) -> Result<f32, AudioError> {
    if prev.is_empty() || prev.len() != cur.len() {
        return Err(AudioError::InvalidParameter);
    }
    let mut acc = 0.0f64;
    for (p, c) in prev.iter().zip(cur.iter()) {
        let d = (*c - *p) as f64;
        if d > 0.0 {
            acc += d;
        }
    }
    Ok(acc as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_frames_zero_flux() {
        let frame = [1.0f32, 2.0, 3.0, 0.5];
        let f = spectral_flux(&frame, &frame).expect("flux");
        assert!(f.abs() < 1e-6, "flux={f}");
    }

    #[test]
    fn rising_energy_positive_flux() {
        let prev = [1.0f32, 1.0, 1.0, 1.0];
        let cur = [1.5f32, 2.0, 1.0, 3.0]; // +0.5 +1.0 +0 +2.0 = 3.5
        let f = spectral_flux(&prev, &cur).expect("flux");
        assert!((f - 3.5).abs() < 1e-5, "flux={f}");
    }

    #[test]
    fn falling_energy_is_rectified_away() {
        // Only rising bins count; a purely decaying frame yields zero flux.
        let prev = [3.0f32, 3.0, 3.0];
        let cur = [1.0f32, 2.0, 0.0];
        let f = spectral_flux(&prev, &cur).expect("flux");
        assert!(f.abs() < 1e-6, "flux={f}");
    }

    #[test]
    fn mixed_counts_only_positive() {
        let prev = [2.0f32, 2.0, 2.0];
        let cur = [0.0f32, 5.0, 2.0]; // -2 -> 0, +3, 0  => 3.0
        let f = spectral_flux(&prev, &cur).expect("flux");
        assert!((f - 3.0).abs() < 1e-5, "flux={f}");
    }

    #[test]
    fn rejects_mismatched_lengths() {
        assert_eq!(
            spectral_flux(&[1.0, 2.0], &[1.0]),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(spectral_flux(&[], &[]), Err(AudioError::InvalidParameter));
    }
}
