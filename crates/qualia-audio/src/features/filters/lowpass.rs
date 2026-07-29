//! RBJ Audio-EQ-Cookbook low-pass biquad coefficient design.

use super::biquad::BiquadCoeffs;
use core::f32::consts::PI;

/// Design a 2nd-order low-pass (RBJ cookbook `LPF`).
///
/// `sample_rate` Hz, `cutoff_hz` corner frequency, `q` resonance
/// (0.707 ≈ Butterworth). Inputs are clamped to a safe range so the returned
/// coefficients are always finite.
pub fn design_lowpass(sample_rate: f32, cutoff_hz: f32, q: f32) -> BiquadCoeffs {
    let (cos_w0, alpha) = rbj_params(sample_rate, cutoff_hz, q);
    let b1 = 1.0 - cos_w0;
    let b0 = b1 * 0.5;
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
    // Keep strictly inside (0, Nyquist) to avoid degenerate sin/cos.
    let f0 = cutoff_hz.clamp(1.0e-4, nyq * 0.999);
    let q = if q > 1.0e-4 { q } else { 1.0e-4 };
    let w0 = 2.0 * PI * f0 / sr;
    let alpha = w0.sin() / (2.0 * q);
    (w0.cos(), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::filters::biquad::BiquadState;

    fn rms(xs: &[f32]) -> f32 {
        let s: f32 = xs.iter().map(|v| v * v).sum();
        (s / xs.len() as f32).sqrt()
    }

    fn run_sine(sr: f32, freq: f32, c: &BiquadCoeffs) -> f32 {
        let n = 8192usize;
        let skip = 2048usize; // discard transient, measure steady state
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
        let rms_in = (acc_in / cnt).sqrt();
        let rms_out = (acc_out / cnt).sqrt();
        rms_out / rms_in
    }

    #[test]
    fn passband_preserves_rms() {
        let c = design_lowpass(48_000.0, 1_000.0, 0.707);
        let ratio = run_sine(48_000.0, 150.0, &c); // well below cutoff
        assert!(ratio > 0.9 && ratio < 1.1, "passband ratio {ratio}");
    }

    #[test]
    fn stopband_attenuates() {
        let c = design_lowpass(48_000.0, 1_000.0, 0.707);
        let ratio = run_sine(48_000.0, 12_000.0, &c); // well above cutoff
        assert!(ratio < 0.3, "stopband ratio {ratio}");
    }

    #[test]
    fn coeffs_finite() {
        let c = design_lowpass(44_100.0, 2_000.0, 0.5);
        assert!(rms(&[c.b0, c.b1, c.b2, c.a1, c.a2]).is_finite());
    }
}
