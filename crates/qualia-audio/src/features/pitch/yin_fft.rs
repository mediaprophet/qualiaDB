//! YinFFT: the YIN difference function computed via an FFT autocorrelation
//! rather than the `O(W · max_lag)` direct double loop.
//!
//! The YIN difference function expands as
//!
//! ```text
//! d(τ) = Σ_{j<W}(x[j] − x[j+τ])²
//!      = Σ_{j<W} x[j]²        (E0, constant)
//!      + Σ_{j<W} x[j+τ]²      (sliding energy, from a prefix sum of squares)
//!      − 2 · Σ_{j<W} x[j]x[j+τ]   (autocorrelation, from the FFT)
//! ```
//!
//! The autocorrelation term is obtained as `IFFT(conj(FFT(a)) · FFT(b))` where
//! `a` is the first `W` samples (zero-padded) and `b` is the whole frame
//! (zero-padded), giving the linear correlation at lags `0..=max_lag` with no
//! circular wrap. The two energy terms come from one prefix-sum pass. The
//! result is the *same* difference function as [`super::yin`], so it feeds the
//! identical CMND → threshold → parabolic-interpolation spine.
//!
//! Zero-heap hot path: a single caller `scratch` buffer is carved into the two
//! complex FFT work areas, the prefix-sum table, and the difference/CMND buffer.
//! Nothing is allocated per call.

use crate::features::fft::radix2::fft_radix2;
use crate::features::pitch::confidence::pitch_confidence;
use crate::features::pitch::estimate::{
    absolute_threshold, cmnd_in_place, parabolic_min, PitchEstimate,
};
use crate::types::AudioError;

/// Minimum `scratch` length (in `f32`) required for a frame of `n` samples with
/// a maximum lag of `max_lag`. `l` is the FFT size (`n` rounded up to a power of
/// two). Exposed so callers can size the buffer without guessing.
#[inline]
pub fn yin_fft_scratch_len(n: usize, max_lag: usize) -> usize {
    let l = n.max(1).next_power_of_two();
    4 * l + n + max_lag + 2
}

/// Estimate the fundamental frequency of `frame` with YinFFT.
///
/// Semantics and return values match [`super::yin::yin_pitch`]; only the
/// difference-function computation differs (FFT autocorrelation vs. direct
/// loop). `scratch` must hold at least [`yin_fft_scratch_len`] floats.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] on a non-positive/inconsistent rate or
///   band, or a frame too short to form a usable lag range.
/// - [`AudioError::OutputBufferTooSmall`] if `scratch` is undersized.
pub fn yin_fft_pitch(
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

    let min_lag = ((sample_rate / max_hz).floor() as usize).max(2);
    let max_lag = ((sample_rate / min_hz).floor() as usize).min(n / 2);
    if max_lag < min_lag + 2 {
        return Err(AudioError::InvalidParameter);
    }

    let l = n.next_power_of_two();
    if scratch.len() < yin_fft_scratch_len(n, max_lag) {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let w = n - max_lag; // integration window (a[j] support)

    // Carve the scratch into disjoint work areas.
    let (comp_a, rest) = scratch.split_at_mut(2 * l);
    let (comp_b, rest) = rest.split_at_mut(2 * l);
    let (psum, rest) = rest.split_at_mut(n + 1);
    let (diff, _) = rest.split_at_mut(max_lag + 1);

    // --- Pack a (first W samples) and b (whole frame) as zero-padded complex.
    for i in 0..l {
        let av = if i < w { frame[i] } else { 0.0 };
        let bv = if i < n { frame[i] } else { 0.0 };
        comp_a[2 * i] = av;
        comp_a[2 * i + 1] = 0.0;
        comp_b[2 * i] = bv;
        comp_b[2 * i + 1] = 0.0;
    }

    // --- Autocorrelation via FFT: IFFT(conj(A) · B).
    fft_radix2(comp_a, false)?;
    fft_radix2(comp_b, false)?;
    for k in 0..l {
        let ar = comp_a[2 * k];
        let ai = comp_a[2 * k + 1];
        let br = comp_b[2 * k];
        let bi = comp_b[2 * k + 1];
        // conj(A) · B = (ar − i·ai)(br + i·bi)
        comp_a[2 * k] = ar * br + ai * bi;
        comp_a[2 * k + 1] = ar * bi - ai * br;
    }
    fft_radix2(comp_a, true)?; // IFFT (1/L normalised); real part = correlation

    // --- Prefix sum of squares (f64 accumulate) for the energy terms.
    let mut run = 0.0f64;
    psum[0] = 0.0;
    for k in 0..n {
        let x = frame[k] as f64;
        run += x * x;
        psum[k + 1] = run as f32;
    }
    let e0 = psum[w];

    // --- Reconstruct the difference function d(τ).
    diff[0] = 0.0;
    for tau in 1..=max_lag {
        let sliding = psum[tau + w] - psum[tau];
        let corr = comp_a[2 * tau];
        let d = e0 + sliding - 2.0 * corr;
        diff[tau] = if d > 0.0 { d } else { 0.0 };
    }

    // --- Shared CMND → threshold → parabolic spine.
    cmnd_in_place(diff, max_lag);
    let tau = absolute_threshold(diff, min_lag, max_lag, threshold);
    let (better_tau, aperiodicity) = parabolic_min(diff, tau, min_lag, max_lag);

    if !(better_tau.is_finite() && better_tau > 0.0) {
        return Ok(PitchEstimate::unvoiced());
    }
    let confidence = pitch_confidence(aperiodicity);
    if aperiodicity > 1.0 || (diff[tau] >= threshold && aperiodicity > 0.5) {
        return Ok(PitchEstimate {
            f0_hz: 0.0,
            confidence,
        });
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
        let mut scratch = vec![0.0f32; yin_fft_scratch_len(2048, 551)];
        let est = yin_fft_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yinfft");
        let err = (est.f0_hz - 440.0).abs() / 440.0;
        assert!(err < 0.01, "f0={} err={err}", est.f0_hz);
        assert!(
            cents(est.f0_hz, 440.0) < 10.0,
            "cents={}",
            cents(est.f0_hz, 440.0)
        );
        assert!(est.confidence > 0.9, "conf={}", est.confidence);
    }

    #[test]
    fn recovers_220_within_one_percent() {
        let frame = sine(220.0, 2048, SR);
        let mut scratch = vec![0.0f32; yin_fft_scratch_len(2048, 551)];
        let est = yin_fft_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yinfft");
        let err = (est.f0_hz - 220.0).abs() / 220.0;
        assert!(err < 0.01, "f0={} err={err}", est.f0_hz);
        assert!(est.confidence > 0.9, "conf={}", est.confidence);
    }

    /// Half-integer true lag (sr/f = 100.5): only parabolic interpolation can
    /// reach within a few cents — proves the interpolation on the FFT path too.
    #[test]
    fn parabolic_beats_integer_lag() {
        let f0 = SR / 100.5;
        let frame = sine(f0, 2048, SR);
        let mut scratch = vec![0.0f32; yin_fft_scratch_len(2048, 551)];
        let est = yin_fft_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yinfft");
        let c = cents(est.f0_hz, f0);
        assert!(c < 5.0, "recovered {} Hz vs {f0} Hz ({c} cents)", est.f0_hz);
        assert!(cents(SR / 100.0, f0) > 8.0);
    }

    /// The FFT path must agree with the direct-loop YIN on a clean tone.
    #[test]
    fn matches_direct_yin() {
        use crate::features::pitch::yin::yin_pitch;
        let frame = sine(329.63, 2048, SR); // E4
        let mut s1 = vec![0.0f32; 1200];
        let mut s2 = vec![0.0f32; yin_fft_scratch_len(2048, 551)];
        let a = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut s1).expect("yin");
        let b = yin_fft_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut s2).expect("yinfft");
        assert!(
            cents(a.f0_hz, b.f0_hz) < 2.0,
            "yin={} yinfft={}",
            a.f0_hz,
            b.f0_hz
        );
    }

    #[test]
    fn rejects_short_scratch() {
        let frame = sine(440.0, 2048, SR);
        let mut scratch = vec![0.0f32; 16];
        assert_eq!(
            yin_fft_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
