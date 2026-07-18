//! Equal-loudness contour weighting applied to a magnitude spectrum.
//!
//! Human hearing is not flat: energy near 2–4 kHz is perceived as much louder
//! than the same energy at 50 Hz or 15 kHz. This module weights a linear
//! magnitude spectrum by an **A-weighting** curve — the standard analytic
//! approximation of the (inverted) 40-phon ISO 226 equal-loudness contour — so
//! that downstream loudness / salience measures reflect perception rather than
//! raw physical energy.
//!
//! The curve is normalised to unity (0 dB) gain at 1 kHz. It is caller-buffered
//! and allocation-free: bin `k` of an `N`-bin spectrum spanning `[0, Nyquist]`
//! is treated as frequency `k · (fs/2) / (N-1)`.

use crate::types::AudioError;

/// Weight a magnitude spectrum `mag` by the A-weighting equal-loudness curve,
/// writing `mag.len()` weighted magnitudes into `out`.
///
/// - `mag`: `N` linear magnitudes for bins `0..N`, spanning `[0, fs/2]`.
/// - `sample_rate`: source sample rate `fs` (Hz); must be non-zero.
/// - `out`: at least `N` floats; receives `mag[k] · w(f_k)`.
///
/// Returns the number of bins written (`mag.len()`), or
/// [`AudioError::InvalidParameter`] for a zero sample rate, or
/// [`AudioError::OutputBufferTooSmall`] if `out` is shorter than `mag`.
pub fn apply_equal_loudness(
    mag: &[f32],
    sample_rate: u32,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    if sample_rate == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if mag.is_empty() {
        return Ok(0);
    }
    if out.len() < mag.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let bins = mag.len();
    let nyquist = sample_rate as f64 / 2.0;
    // Bin spacing: with `bins` samples across [0, Nyquist], the last bin is at
    // Nyquist. Guard the single-bin case.
    let denom = (bins - 1).max(1) as f64;
    for k in 0..bins {
        let f = k as f64 * nyquist / denom;
        out[k] = mag[k] * a_weight_linear(f) as f32;
    }
    Ok(bins)
}

/// Linear (not dB) A-weighting gain at frequency `f` Hz, normalised to `1.0` at
/// 1 kHz. DC (`f ≤ 0`) is fully attenuated, matching the A-curve's high-pass
/// behaviour.
///
/// Uses the IEC 61672 pole frequencies:
/// `R_A(f) = c4·f⁴ / [(f²+c1)·√((f²+c2)(f²+c3))·(f²+c4)]`, then applies the
/// conventional `+2.0 dB` gain offset so `A(1000 Hz) = 0 dB`.
fn a_weight_linear(f: f64) -> f64 {
    if f <= 0.0 {
        return 0.0;
    }
    let f2 = f * f;
    let c1 = 20.598_997_f64 * 20.598_997_f64; // (20.6 Hz)²
    let c2 = 107.652_65_f64 * 107.652_65_f64; // (107.7 Hz)²
    let c3 = 737.862_23_f64 * 737.862_23_f64; // (737.9 Hz)²
    let c4 = 12194.217_f64 * 12194.217_f64; // (12194 Hz)²
    let num = c4 * f2 * f2;
    let den = (f2 + c1) * ((f2 + c2) * (f2 + c3)).sqrt() * (f2 + c4);
    let ra = num / den;
    // +2.00 dB offset → 10^(2/20) ≈ 1.258925 in linear terms; makes 1 kHz unity.
    ra * 1.258_925_4_f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_at_1khz_and_rolloff_at_low_freq() {
        // fs = 2000 Hz, 101 bins → 10 Hz spacing. Bin 100 = 1000 Hz, bin 5 = 50 Hz.
        let bins = 101;
        let mag = vec![1.0f32; bins];
        let mut out = vec![0.0f32; bins];
        let n = apply_equal_loudness(&mag, 2000, &mut out).unwrap();
        assert_eq!(n, bins);

        // 1 kHz: ~unity gain.
        assert!(
            (out[100] - 1.0).abs() < 0.05,
            "1 kHz weight expected ~1.0, got {}",
            out[100]
        );
        // 50 Hz: A-weighting attenuates by ~30 dB → linear ≪ 0.2.
        assert!(out[5] < 0.2, "50 Hz weight expected ≪ 0.2, got {}", out[5]);
        // DC fully attenuated.
        assert_eq!(out[0], 0.0);
        // Low frequencies attenuated relative to 1 kHz.
        assert!(out[5] < out[100]);
    }

    #[test]
    fn rejects_zero_rate() {
        let mag = [1.0f32; 4];
        let mut out = [0.0f32; 4];
        assert_eq!(
            apply_equal_loudness(&mag, 0, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_short_output() {
        let mag = [1.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            apply_equal_loudness(&mag, 16000, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
