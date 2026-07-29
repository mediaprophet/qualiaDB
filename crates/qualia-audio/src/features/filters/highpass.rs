//! RBJ Audio-EQ-Cookbook high-pass biquad coefficient design.

use super::biquad::BiquadCoeffs;
use core::f32::consts::PI;

/// Design a 2nd-order high-pass (RBJ cookbook `HPF`).
///
/// `sample_rate` Hz, `cutoff_hz` corner frequency, `q` resonance
/// (0.707 ≈ Butterworth). Inputs are clamped so coefficients stay finite.
pub fn design_highpass(sample_rate: f32, cutoff_hz: f32, q: f32) -> BiquadCoeffs {
    let (cos_w0, alpha) = rbj_params(sample_rate, cutoff_hz, q);
    let b1n = 1.0 + cos_w0;
    let b0 = b1n * 0.5;
    let b1 = -b1n;
    let b2 = b0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    BiquadCoeffs::from_raw(b0, b1, b2, a0, a1, a2)
}

/// Shared RBJ intermediate terms `(cos(w0), alpha)` with input clamping.
fn rbj_params(sample_rate: f32, cutoff_hz: f32, q: f32) -> (f32, f32) {
    let sr = if sample_rate > 1.0 { sample_rate } else { 1.0 };
    let nyq = sr * 0.5;
    let f0 = cutoff_hz.clamp(1.0e-4, nyq * 0.999);
    let q = if q > 1.0e-4 { q } else { 1.0e-4 };
    let w0 = 2.0 * PI * f0 / sr;
    let alpha = w0.sin() / (2.0 * q);
    (w0.cos(), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::filters::biquad::{BiquadCoeffs, BiquadState};

    fn run_sine(sr: f32, freq: f32, c: &BiquadCoeffs) -> f32 {
        let n = 8192usize;
        let skip = 2048usize;
        let mut st = BiquadState::new();
        let mut acc_in = 0.0f32;
        let mut acc_out = 0.0f32;
        let mut cnt = 0.0f32;
        for i in 0..n {
            let x = (2.0 * core::f32::consts::PI * freq * i as f32 / sr).sin();
            let y = st.process_sample(c, x);
            if i >= skip {
                acc_in += x * x;
                acc_out += y * y;
                cnt += 1.0;
            }
        }
        (acc_out / cnt).sqrt() / (acc_in / cnt).sqrt()
    }

    #[test]
    fn passband_preserves_rms() {
        let c = design_highpass(48_000.0, 1_000.0, 0.707);
        let ratio = run_sine(48_000.0, 12_000.0, &c); // well above cutoff
        assert!(ratio > 0.9 && ratio < 1.1, "passband ratio {ratio}");
    }

    #[test]
    fn stopband_attenuates() {
        let c = design_highpass(48_000.0, 1_000.0, 0.707);
        let ratio = run_sine(48_000.0, 100.0, &c); // well below cutoff
        assert!(ratio < 0.3, "stopband ratio {ratio}");
    }
}
