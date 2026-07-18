//! Full time-domain YIN fundamental-frequency estimator
//! (de Cheveigné & Kawahara, 2002).
//!
//! Pipeline: difference function → cumulative-mean-normalised difference (CMND)
//! → absolute-threshold lag selection → **parabolic interpolation** of the
//! chosen lag → f0 in Hz plus a voicing confidence derived from the frame's
//! aperiodicity. Parabolic interpolation is what makes this recover a tone to a
//! few cents; a bare integer-lag argmin cannot.
//!
//! Zero-heap hot path: the caller supplies the frame and a `scratch` buffer
//! (≥ `max_lag + 1` floats) that holds the difference / CMND function. Nothing
//! is allocated per call.

use crate::features::pitch::confidence::pitch_confidence;
use crate::features::pitch::estimate::{
    absolute_threshold, cmnd_in_place, parabolic_min, PitchEstimate,
};
use crate::types::AudioError;

/// Estimate the fundamental frequency of `frame` with the YIN algorithm.
///
/// - `frame`: mono samples for one analysis window.
/// - `sample_rate`: sampling rate in Hz (> 0).
/// - `min_hz` / `max_hz`: the fundamental search band (`0 < min_hz < max_hz`).
/// - `threshold`: YIN absolute threshold on the CMND (typical 0.10–0.20).
/// - `scratch`: work buffer for the difference/CMND function; must hold at least
///   `floor(sample_rate / min_hz) + 1` floats (bounded by `frame.len()/2 + 1`).
///
/// Returns a [`PitchEstimate`]; `f0_hz == 0.0` with low confidence denotes an
/// unvoiced frame (no lag crossed the threshold and the global CMND minimum is
/// weak).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the rate/band is non-positive or
///   inconsistent, or `frame` is too short to form a usable lag range.
/// - [`AudioError::OutputBufferTooSmall`] if `scratch` cannot hold the CMND up
///   to `max_lag`.
pub fn yin_pitch(
    frame: &[f32],
    sample_rate: f32,
    min_hz: f32,
    max_hz: f32,
    threshold: f32,
    scratch: &mut [f32],
) -> Result<PitchEstimate, AudioError> {
    let params_ok = sample_rate.is_finite()
        && sample_rate > 0.0
        && min_hz.is_finite()
        && min_hz > 0.0
        && max_hz.is_finite()
        && max_hz > min_hz
        && threshold.is_finite();
    if !params_ok {
        return Err(AudioError::InvalidParameter);
    }
    let n = frame.len();
    if n < 8 {
        return Err(AudioError::InvalidParameter);
    }

    // Lag search band. min_lag ≥ 2 so parabolic interpolation always has a
    // left neighbour; max_lag ≤ n/2 so the integration window keeps ≥ n/2
    // samples of overlap.
    let min_lag = ((sample_rate / max_hz).floor() as usize).max(2);
    let max_lag = ((sample_rate / min_hz).floor() as usize).min(n / 2);
    if max_lag < min_lag + 2 {
        return Err(AudioError::InvalidParameter);
    }
    if scratch.len() < max_lag + 1 {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Integration window: as much overlap as the largest lag allows, so every
    // term x[j + τ] stays inside the frame for τ ≤ max_lag.
    let w = n - max_lag;

    // --- Difference function d(τ) = Σ_{j<w} (x[j] − x[j+τ])², f64 accumulate.
    scratch[0] = 0.0;
    for tau in 1..=max_lag {
        let mut acc = 0.0f64;
        for j in 0..w {
            let diff = (frame[j] - frame[j + tau]) as f64;
            acc += diff * diff;
        }
        scratch[tau] = acc as f32;
    }

    // --- Cumulative-mean normalisation (in place).
    cmnd_in_place(scratch, max_lag);

    // --- Absolute-threshold lag selection + parabolic interpolation.
    let tau = absolute_threshold(scratch, min_lag, max_lag, threshold);
    let (better_tau, aperiodicity) = parabolic_min(scratch, tau, min_lag, max_lag);

    if !(better_tau.is_finite() && better_tau > 0.0) {
        return Ok(PitchEstimate::unvoiced());
    }
    let confidence = pitch_confidence(aperiodicity);
    // Voicing gate: a lag that never approached the threshold and carries high
    // aperiodicity is not a real pitch.
    if aperiodicity > 1.0 || (scratch[tau] >= threshold && aperiodicity > 0.5) {
        return Ok(PitchEstimate { f0_hz: 0.0, confidence });
    }
    Ok(PitchEstimate {
        f0_hz: sample_rate / better_tau,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    const SR: f32 = 44_100.0;

    fn sine(freq: f32, n: usize, sr: f32) -> Vec<f32> {
        (0..n).map(|i| (TAU * freq * i as f32 / sr).sin()).collect()
    }

    fn cents(a: f32, b: f32) -> f32 {
        1200.0 * (a / b).log2().abs()
    }

    #[test]
    fn recovers_440_within_one_percent() {
        let frame = sine(440.0, 2048, SR);
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        let err = (est.f0_hz - 440.0).abs() / 440.0;
        assert!(err < 0.01, "f0={} err={err}", est.f0_hz);
        assert!(cents(est.f0_hz, 440.0) < 10.0, "cents={}", cents(est.f0_hz, 440.0));
        assert!(est.confidence > 0.9, "conf={}", est.confidence);
    }

    #[test]
    fn recovers_220_within_one_percent() {
        let frame = sine(220.0, 2048, SR);
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        let err = (est.f0_hz - 220.0).abs() / 220.0;
        assert!(err < 0.01, "f0={} err={err}", est.f0_hz);
        assert!(est.confidence > 0.9, "conf={}", est.confidence);
    }

    /// The interpolation proof: a tone whose true lag is a half-integer
    /// (sr/f = 100.5 → f ≈ 438.806 Hz). Integer-lag argmin would land on lag
    /// 100 (441.0 Hz, ~8.6 cents) or 101 (436.6 Hz, ~15 cents); only parabolic
    /// interpolation reaches within a few cents.
    #[test]
    fn parabolic_beats_integer_lag() {
        let f0 = SR / 100.5; // ≈ 438.806 Hz, deliberately fractional lag
        let frame = sine(f0, 2048, SR);
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        let c = cents(est.f0_hz, f0);
        assert!(c < 5.0, "recovered {} Hz vs {f0} Hz ({c} cents)", est.f0_hz);
        // Sanity: the nearest integer-lag frequency is ≥ 8 cents away, so a
        // pass here cannot be integer-lag luck.
        assert!(cents(SR / 100.0, f0) > 8.0);
    }

    #[test]
    fn white_noise_is_low_confidence() {
        // Deterministic LCG pseudo-noise.
        let mut s: u32 = 0x1234_5678;
        let frame: Vec<f32> = (0..2048)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (s >> 8) as f32 / (1u32 << 23) as f32 - 1.0
            })
            .collect();
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        assert!(est.confidence < 0.5, "noise conf={}", est.confidence);
    }

    #[test]
    fn rejects_short_scratch() {
        let frame = sine(440.0, 2048, SR);
        let mut scratch = vec![0.0f32; 8];
        assert_eq!(
            yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn rejects_bad_band() {
        let frame = sine(440.0, 2048, SR);
        let mut scratch = vec![0.0f32; 1200];
        assert_eq!(
            yin_pitch(&frame, SR, 1000.0, 80.0, 0.1, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
    }
}
