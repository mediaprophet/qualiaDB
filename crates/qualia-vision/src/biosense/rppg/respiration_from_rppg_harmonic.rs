//! Secondary RR from rPPG low-frequency content (RSA / baseline wander band).
//!
//! Uses the same breath band (~0.1–0.5 Hz) as motion RR. Not a second harmonic of HR;
//! it is the low-frequency residual peak of a pulse-like trace after optional
//! HR-band energy is ignored by band-limiting the spectral search.
//! Fail closed on low SNR. No training.

use crate::biosense::respiration::respiration_rate_from_motion_trace::spectral_rr_peak;
use crate::biosense::respiration::rr_estimate::{RrEstimate, RR_MIN_SNR_DEFAULT};
use crate::cv::error::CvError;

/// Estimate respiratory rate from an rPPG (or raw green/pulse) trace by peaking
/// in the respiratory band only.
///
/// * `rppg_trace` — pulse proxy (POS/CHROM/green), one sample per frame.
/// * `fps` — sample rate.
/// * `min_snr` — SNR gate; `None` → [`RR_MIN_SNR_DEFAULT`].
pub fn respiration_from_rppg_harmonic(
    rppg_trace: &[f32],
    fps: f32,
    min_snr: Option<f32>,
) -> Result<RrEstimate, CvError> {
    // Optional mild high-pass residual: subtract a short moving average so slow
    // DC drift does not dominate, while leaving the 0.1–0.5 Hz band intact.
    let n = rppg_trace.len();
    if n < 64 || fps <= 1.0 {
        return Err(CvError::InvalidParameter);
    }
    let win = ((fps * 0.35) as usize).clamp(3, 15) | 1; // odd, ~0.35 s
    let half = win / 2;
    let mut residual = vec![0.0f32; n];
    for i in 0..n {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let mut s = 0.0f32;
        let mut c = 0u32;
        for j in lo..hi {
            s += rppg_trace[j];
            c += 1;
        }
        let ma = s / c as f32;
        residual[i] = rppg_trace[i] - ma;
    }
    spectral_rr_peak(&residual, fps, min_snr.unwrap_or(RR_MIN_SNR_DEFAULT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_pulse_with_rr(
        hr_bpm: f32,
        rr_bpm: f32,
        fps: f32,
        n: usize,
        rr_amp: f32,
    ) -> Vec<f32> {
        let f_hr = hr_bpm / 60.0;
        let f_rr = rr_bpm / 60.0;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            let ti = i as f32 / fps;
            // Cardiac carrier + respiratory AM/baseline (low-freq sinusoid).
            let pulse = (core::f32::consts::TAU * f_hr * ti).sin();
            let breath = rr_amp * (core::f32::consts::TAU * f_rr * ti).sin();
            t[i] = pulse * (1.0 + 0.15 * breath) + breath;
        }
        t
    }

    #[test]
    fn recovers_rr_from_rppg_like_trace() {
        let fps = 30.0;
        let t = synth_pulse_with_rr(72.0, 16.0, fps, 480, 0.8);
        let e = respiration_from_rppg_harmonic(&t, fps, Some(2.0)).unwrap();
        assert!(
            (e.breaths_per_min - 16.0).abs() < 2.0,
            "rr={}",
            e.breaths_per_min
        );
        assert!(e.confidence > 0.15);
    }

    #[test]
    fn fails_closed_on_flat_trace() {
        let t = vec![1.0f32; 200];
        assert!(respiration_from_rppg_harmonic(&t, 30.0, Some(4.0)).is_err());
    }
}
