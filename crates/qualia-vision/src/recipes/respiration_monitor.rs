//! Recipe: consent → motion RR ± rPPG-harmonic RR → ensemble respiration.
//!
//! Orchestration only. Fail closed without consent. No training.

use crate::biosense::{
    ensemble_respiration, pos_rppg_trace, respiration_from_rppg_harmonic,
    respiration_rate_from_motion_trace, BiosenseConsent, RrEstimate, RR_MIN_SNR_DEFAULT,
};
use crate::cv::error::CvError;

/// Default ensemble confidence floor (matches HR ensemble scale).
const DEFAULT_MIN_CONF: f32 = 0.15;

/// Respiratory rate from a vertical-motion trace and optional RGB-mean series.
///
/// * `vert_motion` — per-frame chest / torso vertical motion scalar (length `n_frames`).
/// * `rgb_means` — optional interleaved `[r,g,b,…]` (length `3 * n_frames`) for the
///   rPPG low-frequency harmonic path. When `None`, motion-only ensemble (scaled conf).
/// * `min_confidence` — fused confidence gate; `0.0` → [`DEFAULT_MIN_CONF`].
/// * `min_snr` — spectral SNR gate for each branch; `None` → [`RR_MIN_SNR_DEFAULT`].
pub fn respiration_monitor(
    consent: BiosenseConsent,
    vert_motion: &[f32],
    rgb_means: Option<&[f32]>,
    n_frames: usize,
    fps: f32,
    min_confidence: f32,
    min_snr: Option<f32>,
) -> Result<RrEstimate, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    if n_frames == 0 || vert_motion.len() < n_frames {
        return Err(CvError::BufferTooSmall);
    }
    if fps <= 1.0 {
        return Err(CvError::InvalidParameter);
    }

    let snr_gate = min_snr.unwrap_or(RR_MIN_SNR_DEFAULT);
    let conf_gate = if min_confidence > 0.0 {
        min_confidence
    } else {
        DEFAULT_MIN_CONF
    };

    let motion = respiration_rate_from_motion_trace(&vert_motion[..n_frames], fps, snr_gate).ok();

    let harmonic = if let Some(means) = rgb_means {
        if means.len() < n_frames * 3 {
            return Err(CvError::BufferTooSmall);
        }
        let mut pulse = vec![0.0f32; n_frames];
        pos_rppg_trace(means, n_frames, &mut pulse)?;
        respiration_from_rppg_harmonic(&pulse, fps, Some(snr_gate)).ok()
    } else {
        None
    };

    ensemble_respiration(motion, harmonic, conf_gate)
}

/// Convenience: motion-only respiratory monitor (no rPPG branch).
pub fn respiration_monitor_motion_only(
    consent: BiosenseConsent,
    vert_motion: &[f32],
    fps: f32,
) -> Result<RrEstimate, CvError> {
    respiration_monitor(
        consent,
        vert_motion,
        None,
        vert_motion.len(),
        fps,
        DEFAULT_MIN_CONF,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::BiosensePurpose;

    fn grant() -> BiosenseConsent {
        BiosenseConsent::grant_process(BiosensePurpose::WellfairSelfMonitor, 3)
    }

    fn synth_motion(bpm: f32, fps: f32, n: usize) -> Vec<f32> {
        let f = bpm / 60.0;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            t[i] = (core::f32::consts::TAU * f * i as f32 / fps).sin();
        }
        t
    }

    fn synth_rgb_with_rr(hr_bpm: f32, rr_bpm: f32, fps: f32, n: usize) -> Vec<f32> {
        let f_hr = hr_bpm / 60.0;
        let f_rr = rr_bpm / 60.0;
        let mut m = vec![0.0f32; n * 3];
        for i in 0..n {
            let ti = i as f32 / fps;
            let pulse = (core::f32::consts::TAU * f_hr * ti).sin();
            let breath = 0.8 * (core::f32::consts::TAU * f_rr * ti).sin();
            let g = 120.0 + 20.0 * (pulse * (1.0 + 0.15 * breath) + breath);
            let r = 110.0 + 5.0 * pulse;
            let b = 90.0 - 4.0 * pulse;
            m[i * 3] = r;
            m[i * 3 + 1] = g;
            m[i * 3 + 2] = b;
        }
        m
    }

    #[test]
    fn no_consent_fails_closed() {
        let m = synth_motion(15.0, 30.0, 450);
        let r = respiration_monitor(
            BiosenseConsent::denied(BiosensePurpose::WellfairSelfMonitor),
            &m,
            None,
            m.len(),
            30.0,
            0.1,
            Some(2.0),
        );
        assert!(r.is_err());
    }

    #[test]
    fn motion_only_finds_fifteen_bpm() {
        let fps = 30.0;
        let m = synth_motion(15.0, fps, 450);
        let e = respiration_monitor_motion_only(grant(), &m, fps).unwrap();
        assert!(
            (e.breaths_per_min - 15.0).abs() < 1.5,
            "bpm={}",
            e.breaths_per_min
        );
        assert!(e.confidence > 0.1);
    }

    #[test]
    fn dual_source_ensemble() {
        let fps = 30.0;
        let n = 480;
        let target = 16.0f32;
        let motion = synth_motion(target, fps, n);
        let rgb = synth_rgb_with_rr(72.0, target, fps, n);
        let e = respiration_monitor(
            grant(),
            &motion,
            Some(&rgb),
            n,
            fps,
            0.1,
            Some(2.0),
        )
        .unwrap();
        assert!(
            (e.breaths_per_min - target).abs() < 2.5,
            "bpm={}",
            e.breaths_per_min
        );
        assert!(e.confidence > 0.1);
    }

    #[test]
    fn flat_motion_fails_closed() {
        let m = vec![0.0f32; 200];
        let r = respiration_monitor(grant(), &m, None, m.len(), 30.0, 0.2, Some(8.0));
        assert!(r.is_err());
    }

    #[test]
    fn short_buffer_errors() {
        let m = [0.1f32; 8];
        assert!(respiration_monitor(grant(), &m, None, 8, 30.0, 0.1, None).is_err());
    }
}
