//! Respiratory rate proxy from vertical motion energy band (compat entry).
//!
//! Delegates to [`super::respiration_rate_from_motion_trace`] with default SNR gate.

use super::respiration_rate_from_motion_trace::respiration_rate_from_motion_trace;
use super::rr_estimate::RR_MIN_SNR_DEFAULT;
use crate::cv::error::CvError;

/// `vert_motion` per-frame vertical motion scalar. Estimate breaths/min in ~6–30 band.
///
/// Returns `(breaths_per_min, confidence)`. Fails closed on short/noisy traces
/// (default SNR gate inside the spectral estimator).
pub fn respiration_from_motion(vert_motion: &[f32], fps: f32) -> Result<(f32, f32), CvError> {
    let e = respiration_rate_from_motion_trace(vert_motion, fps, RR_MIN_SNR_DEFAULT)?;
    Ok((e.breaths_per_min, e.confidence))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compat_finds_roughly_15_bpm() {
        let fps = 30.0f32;
        let n = 450usize;
        let f = 15.0 / 60.0;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            t[i] = (core::f32::consts::TAU * f * i as f32 / fps).sin();
        }
        let (bpm, conf) = respiration_from_motion(&t, fps).unwrap();
        assert!((bpm - 15.0).abs() < 1.5, "bpm={}", bpm);
        assert!(conf > 0.15, "conf={}", conf);
    }
}
