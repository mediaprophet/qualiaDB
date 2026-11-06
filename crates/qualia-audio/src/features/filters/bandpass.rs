//! RBJ Audio-EQ-Cookbook band-pass biquad coefficient design.

use super::biquad::BiquadCoeffs;

/// Design a 2nd-order band-pass (RBJ cookbook `BPF`, constant 0 dB peak gain).
///
/// `center_hz` is the peak frequency; `q` sets bandwidth (higher `q` = narrower).
/// Inputs are clamped so coefficients stay finite.
pub fn design_bandpass(sample_rate: f32, center_hz: f32, q: f32) -> BiquadCoeffs {
    let sr = f64::from(if sample_rate > 1.0 { sample_rate } else { 1.0 });
    let f0 = f64::from(center_hz).clamp(1.0e-4, sr * 0.5 * 0.999);
    let q64 = f64::from(if q > 1.0e-4 { q } else { 1.0e-4 });
    etalon_signal::filter::rbj_bandpass(sr, f0, q64)
        .map(BiquadCoeffs::from_etalon)
        .unwrap_or_else(|_| BiquadCoeffs::identity())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::filters::biquad::{BiquadCoeffs, BiquadState};

    fn run_sine(sr: f32, freq: f32, c: &BiquadCoeffs) -> f32 {
        let n = 16384usize;
        let skip = 4096usize;
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
    fn center_passes_edges_reject() {
        let center = 1_000.0;
        let c = design_bandpass(48_000.0, center, 2.0);
        let at_center = run_sine(48_000.0, center, &c);
        let below = run_sine(48_000.0, 100.0, &c);
        let above = run_sine(48_000.0, 10_000.0, &c);
        assert!(at_center > 0.9 && at_center < 1.1, "center {at_center}");
        assert!(below < 0.3, "below {below}");
        assert!(above < 0.3, "above {above}");
    }
}
