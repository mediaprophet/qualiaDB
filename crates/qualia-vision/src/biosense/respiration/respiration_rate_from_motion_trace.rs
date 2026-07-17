//! Respiratory rate from a motion energy / vertical displacement trace.
//!
//! Pure spectral peak in the breath band (~0.1–0.5 Hz): demean → dense DFT
//! power scan → residual-band SNR → optional parabolic refine → fail closed.
//! No learned weights / training.

use super::rr_estimate::{
    snr_to_confidence, RrEstimate, RR_F_HI_HZ, RR_F_LO_HZ, RR_MIN_SAMPLES, RR_MIN_SNR_DEFAULT,
    RR_SPECTRAL_STEPS,
};
use crate::cv::error::CvError;

/// Core spectral peak search in the respiratory band. Fail closed on low SNR.
///
/// Shared by motion RR and rPPG low-frequency harmonic paths.
pub fn spectral_rr_peak(trace: &[f32], fps: f32, min_snr: f32) -> Result<RrEstimate, CvError> {
    let n = trace.len();
    if n < RR_MIN_SAMPLES || fps <= 1.0 || !fps.is_finite() || min_snr < 0.0 {
        return Err(CvError::InvalidParameter);
    }
    // At least ~1.5 cycles of the lowest band edge inside the window.
    let duration = n as f32 / fps;
    if duration < 1.5 / RR_F_LO_HZ {
        return Err(CvError::InvalidParameter);
    }

    let mean = trace.iter().sum::<f32>() / n as f32;
    let steps = RR_SPECTRAL_STEPS;
    let mut powers = [0.0f32; RR_SPECTRAL_STEPS];
    let mut best_k = 0usize;
    let mut best_p = 0.0f32;
    let mut total_p = 0.0f32;
    let denom = (steps as f32 - 1.0).max(1.0);
    let band = RR_F_HI_HZ - RR_F_LO_HZ;

    for k in 0..steps {
        let f = RR_F_LO_HZ + band * (k as f32 / denom);
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for (i, &v) in trace.iter().enumerate() {
            let t = i as f32 / fps;
            let ang = core::f32::consts::TAU * f * t;
            let x = v - mean;
            re += x * ang.cos();
            im += x * ang.sin();
        }
        let p = re * re + im * im;
        powers[k] = p;
        total_p += p;
        if p > best_p {
            best_p = p;
            best_k = k;
        }
    }

    if total_p < 1e-12 || best_p < 1e-12 {
        return Err(CvError::InvalidParameter);
    }

    // Residual-band SNR: peak / mean of non-peak bins (honest vs peak/mean-all inflation).
    let noise = ((total_p - best_p) / (steps as f32 - 1.0).max(1.0)).max(1e-12);
    let snr = best_p / noise;
    if snr < min_snr {
        return Err(CvError::InvalidParameter);
    }

    let f_best = refined_freq_hz(best_k, &powers, steps, denom, band);
    let conf = snr_to_confidence(snr)
        .max(snr / (snr + 8.0))
        .clamp(0.05, 0.95);

    Ok(RrEstimate {
        breaths_per_min: f_best * 60.0,
        snr,
        confidence: conf,
    })
}

fn refined_freq_hz(
    best_k: usize,
    powers: &[f32; RR_SPECTRAL_STEPS],
    steps: usize,
    denom: f32,
    band: f32,
) -> f32 {
    let f_of = |k: usize| RR_F_LO_HZ + band * (k as f32 / denom);
    if best_k == 0 || best_k + 1 >= steps {
        return f_of(best_k);
    }
    let p_l = powers[best_k - 1];
    let p_0 = powers[best_k];
    let p_r = powers[best_k + 1];
    let den = p_l - 2.0 * p_0 + p_r;
    if den.abs() < 1e-12 {
        return f_of(best_k);
    }
    let delta = (0.5 * (p_l - p_r) / den).clamp(-1.0, 1.0);
    let df = band / denom;
    (f_of(best_k) + delta * df).clamp(RR_F_LO_HZ, RR_F_HI_HZ)
}

/// Estimate respiration (breaths/min) from per-frame scalar motion
/// (chest ROI mean, vertical optical-flow energy, …).
///
/// `min_snr`: gate; pass `0.0` to use [`RR_MIN_SNR_DEFAULT`].
pub fn respiration_rate_from_motion_trace(
    trace: &[f32],
    fps: f32,
    min_snr: f32,
) -> Result<RrEstimate, CvError> {
    let gate = if min_snr > 0.0 {
        min_snr
    } else {
        RR_MIN_SNR_DEFAULT
    };
    spectral_rr_peak(trace, fps, gate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_sinusoid(bpm: f32, fps: f32, n: usize, amp: f32, noise: f32) -> Vec<f32> {
        let f = bpm / 60.0;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            let pure = amp * (core::f32::consts::TAU * f * i as f32 / fps).sin();
            let nbit = ((i.wrapping_mul(1103515245).wrapping_add(12345) >> 16) & 0x7fff) as f32
                / 32768.0
                - 0.5;
            t[i] = pure + noise * nbit;
        }
        t
    }

    #[test]
    fn finds_twelve_bpm() {
        let fps = 30.0;
        let t = synth_sinusoid(12.0, fps, 450, 1.0, 0.02);
        let e = respiration_rate_from_motion_trace(&t, fps, 2.0).unwrap();
        assert!(
            (e.breaths_per_min - 12.0).abs() < 1.5,
            "bpm={}",
            e.breaths_per_min
        );
        assert!(e.confidence > 0.2);
        assert!(e.snr > 2.0);
    }

    #[test]
    fn finds_fifteen_bpm() {
        let fps = 30.0f32;
        let t = synth_sinusoid(15.0, fps, 450, 1.0, 0.02);
        let e = respiration_rate_from_motion_trace(&t, fps, 2.0).unwrap();
        assert!(
            (e.breaths_per_min - 15.0).abs() < 1.5,
            "bpm={}",
            e.breaths_per_min
        );
        assert!(e.confidence > 0.2);
    }

    #[test]
    fn finds_eighteen_and_twenty_bpm() {
        let fps = 30.0;
        for target in [18.0f32, 20.0] {
            let t = synth_sinusoid(target, fps, 450, 1.0, 0.02);
            let e = respiration_rate_from_motion_trace(&t, fps, 2.0).unwrap();
            assert!(
                (e.breaths_per_min - target).abs() < 1.8,
                "target={} got={}",
                target,
                e.breaths_per_min
            );
        }
    }

    #[test]
    fn noise_abstains() {
        let tr = vec![0.01f32; 128];
        let r = respiration_rate_from_motion_trace(&tr, 30.0, 50.0);
        assert!(r.is_err());
    }

    #[test]
    fn pure_noise_high_gate_fails_closed() {
        let fps = 30.0;
        let n = 450;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            let nbit = ((i.wrapping_mul(1664525).wrapping_add(1013904223) >> 16) & 0x7fff) as f32
                / 32768.0
                - 0.5;
            t[i] = nbit;
        }
        assert!(respiration_rate_from_motion_trace(&t, fps, 8.0).is_err());
    }

    #[test]
    fn rejects_short_trace() {
        let t = [0.1f32; 16];
        assert!(respiration_rate_from_motion_trace(&t, 30.0, 0.0).is_err());
    }

    #[test]
    fn rejects_bad_fps() {
        let t = synth_sinusoid(15.0, 30.0, 200, 1.0, 0.0);
        assert!(respiration_rate_from_motion_trace(&t, 0.5, 2.0).is_err());
    }
}
