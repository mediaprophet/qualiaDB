//! Reference tuning estimation from spectral peaks — **does not assume 440 Hz**.
//!
//! Given peak frequencies, we measure how far each sits from an equal-tempered
//! semitone grid anchored at a *nominal* reference, then take the magnitude-
//! weighted **circular mean** of those deviations (deviations wrap at ±50 cents,
//! so a plain average would be wrong). The resulting global deviation recovers the
//! true tuning reference: `ref = nominal * 2^(deviation_cents / 1200)`.
//!
//! The nominal anchor is a parameter, so nothing here hardcodes 440 Hz as truth —
//! it is only the grid we measure *against*, and the estimate is free to land
//! anywhere (432, 415, 443, …).
//!
//! Zero-heap: a single streaming pass over the peaks; no allocation.

use crate::types::AudioError;

/// Estimated tuning: the recovered reference frequency and its deviation from the
/// nominal anchor, plus a confidence in `[0, 1]` (resultant-vector concentration —
/// high when peaks agree on one tuning, low for noise / inharmonic material).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuningEstimate {
    /// Recovered reference frequency (Hz) for the equal-tempered grid.
    pub ref_freq_hz: f32,
    /// Deviation from the nominal anchor, in cents (range about ±50).
    pub deviation_cents: f32,
    /// Confidence `[0, 1]`: circular concentration of the peak deviations.
    pub confidence: f32,
}

/// Estimate the reference tuning frequency from spectral peaks.
///
/// - `peak_freqs` / `peak_mags`: parallel peak arrays (Hz, magnitude).
/// - `n_peaks`: leading entries to use (clamped to the shorter array).
/// - `nominal_ref_hz`: the anchor grid frequency to measure against (e.g. 440);
///   parameterised — the estimate is not forced to it.
///
/// Deviations are folded into one semitone (100 cents) and averaged on the circle,
/// so octave / semitone offsets of a peak do not bias the result — only its
/// fractional detuning does.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `nominal_ref_hz` is not positive-finite.
pub fn estimate_tuning(
    peak_freqs: &[f32],
    peak_mags: &[f32],
    n_peaks: usize,
    nominal_ref_hz: f32,
) -> Result<TuningEstimate, AudioError> {
    if !(nominal_ref_hz > 0.0) || !nominal_ref_hz.is_finite() {
        return Err(AudioError::InvalidParameter);
    }

    let np = n_peaks.min(peak_freqs.len()).min(peak_mags.len());
    let two_pi = core::f32::consts::TAU;

    // Weighted sum of unit vectors at angle = 2π * (deviation_cents / 100).
    let mut sum_cos = 0.0f32;
    let mut sum_sin = 0.0f32;
    let mut sum_w = 0.0f32;

    for k in 0..np {
        let f = peak_freqs[k];
        let m = peak_mags[k];
        if !(f > 0.0) || !f.is_finite() || !(m > 0.0) || !m.is_finite() {
            continue;
        }
        // Cents relative to the nominal grid, folded into one semitone [0,100).
        let cents = 1200.0 * (f / nominal_ref_hz).log2();
        let mut frac = cents % 100.0;
        if frac < 0.0 {
            frac += 100.0;
        }
        let ang = two_pi * (frac / 100.0);
        sum_cos += m * ang.cos();
        sum_sin += m * ang.sin();
        sum_w += m;
    }

    if sum_w <= 0.0 {
        // No usable peaks — abstain: report the nominal with zero confidence.
        return Ok(TuningEstimate {
            ref_freq_hz: nominal_ref_hz,
            deviation_cents: 0.0,
            confidence: 0.0,
        });
    }

    // Mean angle → mean fractional cents in [0,100); map to signed ±50.
    let mean_ang = sum_sin.atan2(sum_cos);
    let mut dev = 100.0 * (mean_ang / two_pi); // (-50, 50]-ish
    if dev > 50.0 {
        dev -= 100.0;
    } else if dev < -50.0 {
        dev += 100.0;
    }

    // Resultant length ∈ [0,1] = agreement/concentration of the peaks.
    let resultant = (sum_cos * sum_cos + sum_sin * sum_sin).sqrt() / sum_w;
    let confidence = resultant.clamp(0.0, 1.0);

    let ref_freq_hz = nominal_ref_hz * 2f32.powf(dev / 1200.0);
    Ok(TuningEstimate {
        ref_freq_hz,
        deviation_cents: dev,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semis(a4: f32, n: i32) -> f32 {
        a4 * 2f32.powf(n as f32 / 12.0)
    }

    /// GOLDEN: peaks synthesised on a **432 Hz** equal-tempered grid are recovered
    /// as ref ≈ 432 (a deliberately-detuned reference), measured against a 440
    /// nominal — within a couple of cents.
    #[test]
    fn recovers_432_from_440_nominal() {
        let a4 = 432.0f32;
        // A spread of notes on the 432 grid.
        let freqs = [
            semis(a4, -12),
            semis(a4, -9),
            semis(a4, -5),
            semis(a4, 0),
            semis(a4, 4),
            semis(a4, 7),
        ];
        let mags = [1.0f32; 6];
        let est = estimate_tuning(&freqs, &mags, 6, 440.0).expect("tuning");

        // 1200*log2(432/440) ≈ -31.77 cents.
        assert!(
            (est.deviation_cents - (-31.77)).abs() < 2.0,
            "deviation = {} cents",
            est.deviation_cents
        );
        assert!(
            (est.ref_freq_hz - 432.0).abs() < 1.0,
            "recovered ref = {} Hz",
            est.ref_freq_hz
        );
        // In-tune, consistent peaks → high confidence.
        assert!(est.confidence > 0.95, "confidence = {}", est.confidence);
    }

    /// Standard 440 material measured against 440 → ~0 cents deviation.
    #[test]
    fn in_tune_440_reads_zero() {
        let a4 = 440.0f32;
        let freqs = [semis(a4, -9), semis(a4, -5), semis(a4, -2)];
        let mags = [1.0f32; 3];
        let est = estimate_tuning(&freqs, &mags, 3, 440.0).expect("tuning");
        assert!(
            est.deviation_cents.abs() < 1.0,
            "dev = {}",
            est.deviation_cents
        );
        assert!((est.ref_freq_hz - 440.0).abs() < 0.5);
    }

    #[test]
    fn rejects_bad_nominal() {
        assert_eq!(
            estimate_tuning(&[440.0], &[1.0], 1, 0.0),
            Err(AudioError::InvalidParameter)
        );
    }

    /// No usable peaks → abstain with zero confidence (nominal returned).
    #[test]
    fn abstains_without_peaks() {
        let est = estimate_tuning(&[], &[], 0, 440.0).expect("tuning");
        assert_eq!(est.confidence, 0.0);
        assert_eq!(est.ref_freq_hz, 440.0);
    }
}
