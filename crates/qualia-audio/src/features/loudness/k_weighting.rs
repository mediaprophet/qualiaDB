//! ITU-R BS.1770 / EBU R128 K-weighting pre-filter, as two biquads.
//!
//! K-weighting is a cascade of two second-order stages that run through the
//! shared [`BiquadState`](crate::features::filters::biquad::BiquadState) engine:
//!
//! 1. **Stage 1 — high-frequency shelving** (a ~+4 dB high shelf) modelling the
//!    acoustic effect of the head.
//! 2. **Stage 2 — RLB high-pass** (a 2nd-order high-pass, ~38 Hz corner)
//!    removing sub-audible / rumble energy.
//!
//! The coefficients are derived analytically for the given sample rate via the
//! bilinear transform (`K = tan(pi*f0/fs)`) using the canonical BS.1770
//! parametrisation, so they reproduce the published 48 kHz reference tables
//! exactly and re-scale correctly for any rate (44.1 / 48 / 96 kHz, …).

use crate::features::filters::biquad::BiquadCoeffs;
use core::f64::consts::PI;

// --- Stage 1 (high shelf) design constants (BS.1770 canonical) ------------
const SHELF_F0: f64 = 1_681.974_450_955_533;
const SHELF_Q: f64 = 0.707_175_236_955_419_6;
const SHELF_GAIN_DB: f64 = 3.999_843_853_973_347;
const SHELF_VB_EXP: f64 = 0.499_666_774_154_541_6;

// --- Stage 2 (high pass) design constants ---------------------------------
const HP_F0: f64 = 38.135_470_876_024_44;
const HP_Q: f64 = 0.500_327_037_323_877_3;

/// Build the two BS.1770 K-weighting biquads for `sample_rate` Hz.
///
/// Returns `(stage1_high_shelf, stage2_high_pass)`; run a sample through stage 1
/// then stage 2. `sample_rate` is clamped to `>= 1.0` so coefficients stay
/// finite for degenerate inputs.
pub fn k_weighting_coeffs(sample_rate: f32) -> (BiquadCoeffs, BiquadCoeffs) {
    let fs = if sample_rate > 1.0 {
        sample_rate as f64
    } else {
        1.0
    };

    // Stage 1: high-frequency shelving filter.
    let k = (PI * SHELF_F0 / fs).tan();
    let vh = 10f64.powf(SHELF_GAIN_DB / 20.0);
    let vb = vh.powf(SHELF_VB_EXP);
    let kk = k * k;
    let kq = k / SHELF_Q;
    let a0 = 1.0 + kq + kk;
    let shelf = BiquadCoeffs {
        b0: ((vh + vb * kq + kk) / a0) as f32,
        b1: ((2.0 * (kk - vh)) / a0) as f32,
        b2: ((vh - vb * kq + kk) / a0) as f32,
        a1: ((2.0 * (kk - 1.0)) / a0) as f32,
        a2: ((1.0 - kq + kk) / a0) as f32,
    };

    // Stage 2: 2nd-order high-pass. Numerator is exactly [1, -2, 1]; only the
    // poles depend on the sample rate.
    let kb = (PI * HP_F0 / fs).tan();
    let kkb = kb * kb;
    let kqb = kb / HP_Q;
    let a0b = 1.0 + kqb + kkb;
    let hp = BiquadCoeffs {
        b0: 1.0,
        b1: -2.0,
        b2: 1.0,
        a1: ((2.0 * (kkb - 1.0)) / a0b) as f32,
        a2: ((1.0 - kqb + kkb) / a0b) as f32,
    };

    (shelf, hp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::filters::biquad::BiquadState;
    use core::f32::consts::PI as PI32;

    #[test]
    fn matches_bs1770_reference_at_48k() {
        // Published ITU-R BS.1770 coefficient tables at 48 kHz.
        let (shelf, hp) = k_weighting_coeffs(48_000.0);
        assert!(
            (shelf.b0 - 1.535_124_9).abs() < 1e-3,
            "shelf.b0 {}",
            shelf.b0
        );
        assert!(
            (shelf.b1 - (-2.691_696_2)).abs() < 1e-3,
            "shelf.b1 {}",
            shelf.b1
        );
        assert!(
            (shelf.b2 - 1.198_392_8).abs() < 1e-3,
            "shelf.b2 {}",
            shelf.b2
        );
        assert!(
            (shelf.a1 - (-1.690_659_3)).abs() < 1e-3,
            "shelf.a1 {}",
            shelf.a1
        );
        assert!(
            (shelf.a2 - 0.732_480_8).abs() < 1e-3,
            "shelf.a2 {}",
            shelf.a2
        );
        assert_eq!(hp.b0, 1.0);
        assert_eq!(hp.b1, -2.0);
        assert_eq!(hp.b2, 1.0);
        assert!((hp.a1 - (-1.990_047_5)).abs() < 1e-3, "hp.a1 {}", hp.a1);
        assert!((hp.a2 - 0.990_072_3).abs() < 1e-3, "hp.a2 {}", hp.a2);
    }

    /// Combined K-weighting amplitude gain at a frequency (steady-state RMS).
    fn k_gain(sr: f32, freq: f32) -> f32 {
        let (shelf, hp) = k_weighting_coeffs(sr);
        let (n, skip) = (16_384usize, 4_096usize);
        let mut s1 = BiquadState::new();
        let mut s2 = BiquadState::new();
        let (mut ein, mut eout, mut cnt) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let x = (2.0 * PI32 * freq * i as f32 / sr).sin();
            let y = s2.process_sample(&hp, s1.process_sample(&shelf, x));
            if i >= skip {
                ein += (x * x) as f64;
                eout += (y * y) as f64;
                cnt += 1.0;
            }
        }
        ((eout / cnt).sqrt() / (ein / cnt).sqrt()) as f32
    }

    #[test]
    fn one_khz_gain_is_slightly_positive_db() {
        // The K-curve is ~0 to +1 dB around 1 kHz — the calibration region.
        let g = k_gain(48_000.0, 1_000.0);
        let db = 20.0 * g.log10();
        assert!((-0.5..1.5).contains(&db), "1 kHz K-gain {db} dB");
    }

    #[test]
    fn sub_bass_is_attenuated() {
        // The RLB high-pass kills rumble: 20 Hz should be well below unity.
        let g = k_gain(48_000.0, 20.0);
        assert!(g < 0.6, "20 Hz gain {g}");
    }
}
