//! Full time-domain YIN fundamental-frequency estimator
//! (de Cheveigné & Kawahara, 2002), delegated to [`etalon_signal::pitch`].

use crate::features::pitch::confidence::pitch_confidence;
use crate::features::pitch::estimate::{
    absolute_threshold, cmnd_in_place, parabolic_min, PitchEstimate,
};
use crate::types::AudioError;

/// Estimate the fundamental frequency of `frame` with the YIN algorithm.
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

    let min_lag = ((sample_rate / max_hz).floor() as usize).max(2);
    let max_lag = ((sample_rate / min_hz).floor() as usize).min(n / 2);
    if max_lag < min_lag + 2 {
        return Err(AudioError::InvalidParameter);
    }
    if scratch.len() < max_lag + 1 {
        return Err(AudioError::OutputBufferTooSmall);
    }

    etalon_signal::pitch::yin_difference_into(frame, max_lag, &mut scratch[..=max_lag]);
    cmnd_in_place(scratch, max_lag);

    let tau = absolute_threshold(scratch, min_lag, max_lag, threshold);
    let (better_tau, aperiodicity) = parabolic_min(scratch, tau, min_lag, max_lag);

    if !(better_tau.is_finite() && better_tau > 0.0) {
        return Ok(PitchEstimate::unvoiced());
    }
    let confidence = pitch_confidence(aperiodicity);
    if aperiodicity > 1.0 || (scratch[tau] >= threshold && aperiodicity > 0.5) {
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
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
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
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        let err = (est.f0_hz - 220.0).abs() / 220.0;
        assert!(err < 0.01, "f0={} err={err}", est.f0_hz);
        assert!(est.confidence > 0.9, "conf={}", est.confidence);
    }

    #[test]
    fn parabolic_beats_integer_lag() {
        let f0 = SR / 100.5;
        let frame = sine(f0, 2048, SR);
        let mut scratch = vec![0.0f32; 1200];
        let est = yin_pitch(&frame, SR, 80.0, 1000.0, 0.1, &mut scratch).expect("yin");
        let c = cents(est.f0_hz, f0);
        assert!(c < 5.0, "recovered {} Hz vs {f0} Hz ({c} cents)", est.f0_hz);
        assert!(cents(SR / 100.0, f0) > 8.0);
    }

    #[test]
    fn white_noise_is_low_confidence() {
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
