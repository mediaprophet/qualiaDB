//! Active challenge-response PAD using pose / blendshape proxies.
//!
//! Excellence path: MediaPipe mesh supplies yaw + blendshapes; this module
//! verifies the challenge without a passive RGB anti-spoof network.
//! Fails closed. Not identity proof alone — combine with 1:1 template + consent.

use super::challenge_kind::ChallengeKind;
use crate::biosense::consent::BiosenseConsent;
use crate::cv::error::CvError;

#[derive(Debug, Clone, Copy)]
pub struct MeshFrameSignals {
    /// Head yaw radians (approx); + = left of camera convention as supplied by mesh adapter.
    pub yaw: f32,
    pub pitch: f32,
    /// 0..1 smile intensity (blendshape proxy).
    pub smile: f32,
    /// 0..1 blink (both eyes).
    pub blink: f32,
    /// Mean landmark motion vs previous frame (micro-motion / anti-static).
    pub mesh_motion: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadReason {
    Pass,
    NoConsent,
    Timeout,
    WrongAction,
    StaticMesh,
    InsufficientFrames,
}

#[derive(Debug, Clone, Copy)]
pub struct PadResult {
    pub passed: bool,
    pub reason: PadReason,
    pub challenge: ChallengeKind,
}

/// Thresholds (tunable; document in MANIFEST when production-calibrated).
pub struct PadThresholds {
    pub yaw_delta: f32,
    pub smile_peak: f32,
    pub blink_peak: f32,
    pub min_mesh_motion: f32,
    pub min_frames: usize,
}

impl Default for PadThresholds {
    fn default() -> Self {
        Self {
            yaw_delta: 0.25,
            smile_peak: 0.35,
            blink_peak: 0.4,
            min_mesh_motion: 0.002,
            min_frames: 8,
        }
    }
}

/// Evaluate challenge over a short buffer of mesh signals.
pub fn evaluate_challenge_pad(
    consent: BiosenseConsent,
    challenge: ChallengeKind,
    frames: &[MeshFrameSignals],
    thr: &PadThresholds,
) -> Result<PadResult, CvError> {
    if !consent.may_process() {
        return Ok(PadResult {
            passed: false,
            reason: PadReason::NoConsent,
            challenge,
        });
    }
    if frames.len() < thr.min_frames {
        return Ok(PadResult {
            passed: false,
            reason: PadReason::InsufficientFrames,
            challenge,
        });
    }

    let yaw0 = frames[0].yaw;
    let mut max_smile = 0.0f32;
    let mut blink_events = 0u32;
    let mut prev_blink = frames[0].blink;
    let mut motion_sum = 0.0f32;
    let mut yaw_max = yaw0;
    let mut yaw_min = yaw0;

    for f in frames {
        max_smile = max_smile.max(f.smile);
        yaw_max = yaw_max.max(f.yaw);
        yaw_min = yaw_min.min(f.yaw);
        motion_sum += f.mesh_motion;
        if prev_blink < thr.blink_peak * 0.5 && f.blink >= thr.blink_peak {
            blink_events += 1;
        }
        prev_blink = f.blink;
    }
    let mean_motion = motion_sum / frames.len() as f32;
    if mean_motion < thr.min_mesh_motion {
        return Ok(PadResult {
            passed: false,
            reason: PadReason::StaticMesh,
            challenge,
        });
    }

    let action_ok = match challenge {
        ChallengeKind::YawLeft => (yaw_max - yaw0) >= thr.yaw_delta,
        ChallengeKind::YawRight => (yaw0 - yaw_min) >= thr.yaw_delta,
        ChallengeKind::Smile => max_smile >= thr.smile_peak,
        ChallengeKind::BlinkTwice => blink_events >= 2,
    };

    if action_ok {
        Ok(PadResult {
            passed: true,
            reason: PadReason::Pass,
            challenge,
        })
    } else {
        Ok(PadResult {
            passed: false,
            reason: PadReason::WrongAction,
            challenge,
        })
    }
}

/// Issue a challenge (caller tracks timeout wall-clock).
pub fn issue_challenge(seed: u64) -> ChallengeKind {
    ChallengeKind::from_seed(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};

    fn consent() -> BiosenseConsent {
        BiosenseConsent::grant_security_template(1)
    }

    #[test]
    fn smile_pass() {
        let mut frames = [MeshFrameSignals {
            yaw: 0.0,
            pitch: 0.0,
            smile: 0.0,
            blink: 0.0,
            mesh_motion: 0.01,
        }; 10];
        frames[5].smile = 0.8;
        let r = evaluate_challenge_pad(
            consent(),
            ChallengeKind::Smile,
            &frames,
            &PadThresholds::default(),
        )
        .unwrap();
        assert!(r.passed);
        assert_eq!(r.reason, PadReason::Pass);
    }

    #[test]
    fn static_mesh_fails() {
        let frames = [MeshFrameSignals {
            yaw: 0.0,
            pitch: 0.0,
            smile: 0.9,
            blink: 0.0,
            mesh_motion: 0.0,
        }; 10];
        let r = evaluate_challenge_pad(
            consent(),
            ChallengeKind::Smile,
            &frames,
            &PadThresholds::default(),
        )
        .unwrap();
        assert!(!r.passed);
        assert_eq!(r.reason, PadReason::StaticMesh);
    }

    #[test]
    fn no_consent_fails() {
        let frames = [MeshFrameSignals {
            yaw: 0.0,
            pitch: 0.0,
            smile: 0.9,
            blink: 0.0,
            mesh_motion: 0.01,
        }; 10];
        let r = evaluate_challenge_pad(
            BiosenseConsent::denied(BiosensePurpose::Security),
            ChallengeKind::Smile,
            &frames,
            &PadThresholds::default(),
        )
        .unwrap();
        assert!(!r.passed);
        assert_eq!(r.reason, PadReason::NoConsent);
    }
}
