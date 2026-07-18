//! Adaptive noise-floor estimation by minimum statistics.
//!
//! The quietest recent frames are, with high probability, noise-only. Tracking
//! the *minimum* short-term energy over a trailing window therefore yields a
//! robust estimate of the noise level that follows slow drifts (fans, hiss,
//! room tone) without ever being pulled up by transient speech or tones.
//!
//! The raw minimum systematically *under*-estimates the mean noise power (it is
//! an extreme-order statistic), so it is multiplied by a fixed bias-compensation
//! factor to recover an estimate of the floor an activity gate should clear.
//!
//! Zero-heap: a single pass over a caller-supplied energy slice. Stateless, so
//! callers stream by keeping their own small ring of recent per-frame energies
//! (see [`crate::features::vad::segment_voiced`]).

/// Bias compensation for the minimum-of-window estimator.
///
/// The trailing minimum lies below the mean noise energy; scaling it up by this
/// factor (≈ +3.5 dB) approximates the level a voice-activity gate must exceed
/// to avoid firing on the noise itself. Empirical minimum-statistics work uses
/// window-length-dependent factors in roughly this range; a fixed conservative
/// value keeps the estimator honest (it never claims the floor is *below* the
/// observed quiet frames) and dependency-free.
pub const NOISE_BIAS: f32 = 1.5;

/// Estimate the noise floor as the bias-compensated minimum energy over the
/// trailing `window` entries of `energies` (per-frame RMS or power values).
///
/// - `energies`: recent per-frame energy statistics, oldest→newest. Only the
///   last `window` entries are considered; if fewer are present, all are used.
/// - `window`: number of trailing frames the minimum is taken over. A larger
///   window is more robust to sustained speech but slower to track rising noise.
///
/// Returns the estimated floor `min(recent) * NOISE_BIAS`, clamped to be
/// non-negative. Returns `0.0` for an empty slice (no evidence yet — the caller
/// should treat a zero floor as "unknown", which downstream gates guard with a
/// tiny epsilon rather than declaring everything voiced).
///
/// Non-finite inputs (`NaN`/`±inf`) are ignored so a single bad sample cannot
/// poison the estimate.
pub fn noise_floor_min_stat(energies: &[f32], window: usize) -> f32 {
    if energies.is_empty() {
        return 0.0;
    }
    let w = window.max(1);
    let start = energies.len().saturating_sub(w);
    let mut min_e = f32::INFINITY;
    let mut seen = false;
    for &e in &energies[start..] {
        if e.is_finite() && e >= 0.0 && e < min_e {
            min_e = e;
            seen = true;
        }
    }
    if !seen {
        return 0.0;
    }
    (min_e * NOISE_BIAS).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(noise_floor_min_stat(&[], 8), 0.0);
    }

    #[test]
    fn tracks_minimum_scaled_by_bias() {
        let energies = [0.5f32, 0.4, 0.01, 0.6, 0.3];
        let floor = noise_floor_min_stat(&energies, 8);
        assert!((floor - 0.01 * NOISE_BIAS).abs() < 1e-6, "floor={floor}");
        // The floor is inflated above the raw minimum (bias compensation).
        assert!(floor > 0.01, "floor should exceed raw min, got {floor}");
    }

    #[test]
    fn window_limits_to_trailing_frames() {
        // Quiet frame is old and outside the window → ignored; min is the recent 0.2.
        let energies = [0.01f32, 0.5, 0.4, 0.3, 0.2];
        let floor = noise_floor_min_stat(&energies, 3);
        assert!((floor - 0.2 * NOISE_BIAS).abs() < 1e-6, "floor={floor}");
    }

    #[test]
    fn window_larger_than_len_uses_all() {
        let energies = [0.05f32, 0.5, 0.4];
        let floor = noise_floor_min_stat(&energies, 64);
        assert!((floor - 0.05 * NOISE_BIAS).abs() < 1e-6, "floor={floor}");
    }

    #[test]
    fn ignores_non_finite() {
        let energies = [f32::NAN, 0.3, f32::INFINITY, 0.02];
        let floor = noise_floor_min_stat(&energies, 8);
        assert!((floor - 0.02 * NOISE_BIAS).abs() < 1e-6, "floor={floor}");
    }

    #[test]
    fn all_non_finite_is_zero() {
        let energies = [f32::NAN, f32::INFINITY];
        assert_eq!(noise_floor_min_stat(&energies, 8), 0.0);
    }
}
