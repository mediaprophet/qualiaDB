//! Recipe: consent → quality gate → optional colour-EVM on face ROI → rPPG ensemble.
//!
//! Returns HR + confidence with an explicit **abstain** path (does not invent a pulse).
//! Fail closed without consent. No training.

use crate::biosense::{
    ensemble_hr, eulerian_color_magnify_consented, face_roi_center, frame_blur_score,
    reject_low_quality, roi_mean_rgb, BiosenseConsent, ColourEvmParams, FaceRoi, HrEstimate,
    QualityReject,
};
use crate::cv::buffer::{GrayView, RgbView};
use crate::cv::color::rgb_to_gray_u8;

/// Why the pulse recipe abstained (honest fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseAbstain {
    NoConsent,
    InsufficientFrames,
    BufferTooSmall,
    LowQuality,
    LowConfidence,
    EvmRefused,
}

/// Outcome of [`self_monitor_pulse_evm`]: estimate or structured abstain.
#[derive(Debug, Clone, Copy)]
pub struct PulseEvmResult {
    pub bpm: f32,
    pub confidence: f32,
    pub snr: f32,
    pub abstained: bool,
    pub used_evm: bool,
    pub reason: Option<PulseAbstain>,
}

impl PulseEvmResult {
    fn estimate(hr: HrEstimate, used_evm: bool) -> Self {
        Self {
            bpm: hr.bpm,
            confidence: hr.confidence,
            snr: hr.snr,
            abstained: false,
            used_evm,
            reason: None,
        }
    }

    fn abstain(reason: PulseAbstain, used_evm: bool) -> Self {
        Self {
            bpm: 0.0,
            confidence: 0.0,
            snr: 0.0,
            abstained: true,
            used_evm,
            reason: Some(reason),
        }
    }
}

/// Minimum frames for spectral HR (matches ensemble path).
const MIN_FRAMES: usize = 32;
/// Default ensemble confidence floor.
const DEFAULT_MIN_CONF: f32 = 0.15;
/// Blur variance floor (same order as `self_monitor_pulse`).
const MIN_BLUR: f32 = 10.0;
/// Motion energy ceiling on first pair (disabled when only one gray available).
const MAX_MOTION: f32 = 40.0;

/// Self-monitor pulse with optional Eulerian colour magnification on the face ROI.
///
/// * `use_evm` — when true, run consent-gated colour EVM on the ROI crop before
///   ROI-mean extraction; on EVM refuse, **fall back** to unmagnified means
///   (still quality-gated). Set SNR gate off via soft params when frames are short.
/// * `min_confidence` — ensemble SNR confidence gate; below → abstain.
pub fn self_monitor_pulse_evm(
    consent: BiosenseConsent,
    rgb_frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    fps: f32,
    use_evm: bool,
    min_confidence: f32,
) -> PulseEvmResult {
    if !consent.may_process() {
        return PulseEvmResult::abstain(PulseAbstain::NoConsent, false);
    }
    let fb = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(3);
    if n_frames < MIN_FRAMES || fb == 0 {
        return PulseEvmResult::abstain(PulseAbstain::InsufficientFrames, false);
    }
    if rgb_frames.len() < n_frames.saturating_mul(fb) {
        return PulseEvmResult::abstain(PulseAbstain::BufferTooSmall, false);
    }
    if fps <= 1.0 {
        return PulseEvmResult::abstain(PulseAbstain::InsufficientFrames, false);
    }

    // Quality gate on first frame (and optional first-pair motion).
    let mut gray0 = vec![0u8; (width * height) as usize];
    let v0 = match RgbView::new(width, height, width * 3, &rgb_frames[..fb]) {
        Some(v) => v,
        None => return PulseEvmResult::abstain(PulseAbstain::BufferTooSmall, false),
    };
    if rgb_to_gray_u8(v0, &mut gray0).is_err() {
        return PulseEvmResult::abstain(PulseAbstain::LowQuality, false);
    }
    let gv0 = match GrayView::new(width, height, width, &gray0) {
        Some(g) => g,
        None => return PulseEvmResult::abstain(PulseAbstain::LowQuality, false),
    };
    let blur = frame_blur_score(gv0);
    if reject_low_quality(blur, 0.0, MIN_BLUR, MAX_MOTION) == QualityReject::TooBlurry {
        return PulseEvmResult::abstain(PulseAbstain::LowQuality, false);
    }

    let roi = face_roi_center(v0);
    let conf_gate = if min_confidence > 0.0 {
        min_confidence
    } else {
        DEFAULT_MIN_CONF
    };

    let (means, used_evm) = match extract_roi_means(
        consent, rgb_frames, n_frames, width, height, fb, roi, fps, use_evm,
    ) {
        Ok(v) => v,
        Err(PulseAbstain::EvmRefused) => {
            // Fall back without EVM if magnify path hard-failed buffer geometry.
            match extract_roi_means(
                consent, rgb_frames, n_frames, width, height, fb, roi, fps, false,
            ) {
                Ok((m, _)) => (m, false),
                Err(r) => return PulseEvmResult::abstain(r, false),
            }
        }
        Err(r) => return PulseEvmResult::abstain(r, false),
    };

    match ensemble_hr(consent, &means, n_frames, fps, conf_gate) {
        Ok(hr) => PulseEvmResult::estimate(hr, used_evm),
        Err(_) => PulseEvmResult::abstain(PulseAbstain::LowConfidence, used_evm),
    }
}

fn extract_roi_means(
    consent: BiosenseConsent,
    rgb_frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    fb: usize,
    roi: FaceRoi,
    fps: f32,
    use_evm: bool,
) -> Result<(Vec<f32>, bool), PulseAbstain> {
    let mut means = vec![0.0f32; n_frames * 3];

    if use_evm && roi.w >= 4 && roi.h >= 4 {
        let rw = roi.w as usize;
        let rh = roi.h as usize;
        let crop_fb = rw * rh * 3;
        let mut cropped = vec![0u8; n_frames * crop_fb];
        for i in 0..n_frames {
            let src = &rgb_frames[i * fb..(i + 1) * fb];
            let v =
                RgbView::new(width, height, width * 3, src).ok_or(PulseAbstain::BufferTooSmall)?;
            crop_roi_rgb(v, roi, &mut cropped[i * crop_fb..(i + 1) * crop_fb]);
        }
        let mut magnified = vec![0u8; n_frames * crop_fb];
        let params = ColourEvmParams {
            fps,
            f_lo_hz: 0.7,
            f_hi_hz: 4.0,
            alpha_chroma: 20.0,
            alpha_luma: 0.0,
            // Soft gate: recipe still abstains later via ensemble conf.
            require_snr: false,
            ..ColourEvmParams::default()
        };
        match eulerian_color_magnify_consented(
            consent,
            &cropped,
            n_frames,
            roi.w,
            roi.h,
            params,
            &mut magnified,
        ) {
            Ok(_) => {
                let full_roi = FaceRoi {
                    x: 0,
                    y: 0,
                    w: roi.w,
                    h: roi.h,
                };
                for i in 0..n_frames {
                    let slice = &magnified[i * crop_fb..(i + 1) * crop_fb];
                    let v = RgbView::new(roi.w, roi.h, roi.w * 3, slice)
                        .ok_or(PulseAbstain::BufferTooSmall)?;
                    let mut m = [0.0f32; 3];
                    roi_mean_rgb(v, full_roi, &mut m);
                    means[i * 3] = m[0];
                    means[i * 3 + 1] = m[1];
                    means[i * 3 + 2] = m[2];
                }
                return Ok((means, true));
            }
            Err(_) => {
                // Caller may fall back; signal EVM path refused.
                return Err(PulseAbstain::EvmRefused);
            }
        }
    }

    // Unmagnified ROI means.
    for i in 0..n_frames {
        let slice = &rgb_frames[i * fb..(i + 1) * fb];
        let v =
            RgbView::new(width, height, width * 3, slice).ok_or(PulseAbstain::BufferTooSmall)?;
        let mut m = [0.0f32; 3];
        roi_mean_rgb(v, roi, &mut m);
        means[i * 3] = m[0];
        means[i * 3 + 1] = m[1];
        means[i * 3 + 2] = m[2];
    }
    Ok((means, false))
}

fn crop_roi_rgb(src: RgbView<'_>, roi: FaceRoi, out: &mut [u8]) {
    let rw = roi.w as usize;
    let rh = roi.h as usize;
    debug_assert!(out.len() >= rw * rh * 3);
    for dy in 0..rh {
        for dx in 0..rw {
            let x = roi.x + dx as u32;
            let y = roi.y + dy as u32;
            let (r, g, b) = if x < src.width && y < src.height {
                src.pixel(x, y)
            } else {
                (0, 0, 0)
            };
            let o = (dy * rw + dx) * 3;
            out[o] = r;
            out[o + 1] = g;
            out[o + 2] = b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::BiosensePurpose;

    fn grant() -> BiosenseConsent {
        BiosenseConsent::grant_process(BiosensePurpose::WellfairSelfMonitor, 7)
    }

    /// Synthetic face-like frames: central warm patch + green pulse at `hr_bpm`.
    fn synth_pulse_frames(n: usize, w: u32, h: u32, fps: f32, hr_bpm: f32) -> Vec<u8> {
        let fb = (w * h * 3) as usize;
        let mut frames = vec![40u8; n * fb];
        let f_hr = hr_bpm / 60.0;
        let cx0 = w / 3;
        let cy0 = h / 6;
        let rw = (w / 3).max(8);
        let rh = (h / 4).max(8);
        for t in 0..n {
            let phase = (core::f32::consts::TAU * f_hr * t as f32 / fps).sin();
            let g = (120.0 + 25.0 * phase).clamp(0.0, 255.0) as u8;
            let r = (140.0 + 8.0 * phase).clamp(0.0, 255.0) as u8;
            let b = (90.0 - 6.0 * phase).clamp(0.0, 255.0) as u8;
            let base = t * fb;
            for y in cy0..cy0 + rh {
                for x in cx0..cx0 + rw {
                    // Add spatial texture so blur score is non-zero.
                    let tex = ((x + y + t as u32) % 7) as u8;
                    let i = base + ((y * w + x) * 3) as usize;
                    frames[i] = r.saturating_add(tex);
                    frames[i + 1] = g.saturating_add(tex / 2);
                    frames[i + 2] = b;
                }
            }
            // Edge noise for laplacian variance.
            for y in 0..h {
                for x in 0..w {
                    if x < 2 || y < 2 || x + 2 >= w || y + 2 >= h {
                        continue;
                    }
                    if (x + y) % 11 == 0 {
                        let i = base + ((y * w + x) * 3) as usize;
                        frames[i] = frames[i].saturating_add(30);
                        frames[i + 1] = frames[i + 1].saturating_add(20);
                    }
                }
            }
        }
        frames
    }

    #[test]
    fn no_consent_abstains() {
        let frames = synth_pulse_frames(64, 48, 48, 30.0, 72.0);
        let r = self_monitor_pulse_evm(
            BiosenseConsent::denied(BiosensePurpose::WellfairSelfMonitor),
            &frames,
            64,
            48,
            48,
            30.0,
            false,
            0.1,
        );
        assert!(r.abstained);
        assert_eq!(r.reason, Some(PulseAbstain::NoConsent));
    }

    #[test]
    fn short_window_abstains() {
        let frames = synth_pulse_frames(8, 32, 32, 30.0, 60.0);
        let r = self_monitor_pulse_evm(grant(), &frames, 8, 32, 32, 30.0, false, 0.1);
        assert!(r.abstained);
        assert_eq!(r.reason, Some(PulseAbstain::InsufficientFrames));
    }

    #[test]
    fn synthetic_pulse_without_evm() {
        let n = 150;
        let w = 64u32;
        let h = 64u32;
        let fps = 30.0;
        let frames = synth_pulse_frames(n, w, h, fps, 72.0);
        let r = self_monitor_pulse_evm(grant(), &frames, n, w, h, fps, false, 0.05);
        // Quality + spectral path: expect a estimate or low-confidence abstain
        // (never invent NoConsent). Prefer estimate when SNR is good.
        if r.abstained {
            assert_eq!(r.reason, Some(PulseAbstain::LowConfidence));
        } else {
            assert!(!r.used_evm);
            assert!((r.bpm - 72.0).abs() < 15.0, "bpm={}", r.bpm);
            assert!(r.confidence > 0.0);
        }
    }

    #[test]
    fn synthetic_pulse_with_evm_flag() {
        let n = 90;
        let w = 48u32;
        let h = 48u32;
        let fps = 30.0;
        let frames = synth_pulse_frames(n, w, h, fps, 60.0);
        let r = self_monitor_pulse_evm(grant(), &frames, n, w, h, fps, true, 0.05);
        // EVM path may run or fall back; must not claim consent failure.
        assert_ne!(r.reason, Some(PulseAbstain::NoConsent));
        if !r.abstained {
            assert!((r.bpm - 60.0).abs() < 20.0, "bpm={}", r.bpm);
        }
    }

    #[test]
    fn flat_uniform_frames_quality_or_conf_abstain() {
        // Uniform frames → low blur score → quality abstain (or conf if blur slips).
        let n = 64;
        let w = 32u32;
        let h = 32u32;
        let frames = vec![128u8; n * (w * h * 3) as usize];
        let r = self_monitor_pulse_evm(grant(), &frames, n, w, h, 30.0, false, 0.2);
        assert!(r.abstained);
        assert!(matches!(
            r.reason,
            Some(PulseAbstain::LowQuality | PulseAbstain::LowConfidence)
        ));
    }
}
