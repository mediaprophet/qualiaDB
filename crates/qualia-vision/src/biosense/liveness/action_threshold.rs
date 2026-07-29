//! Map challenges to geometric / expression thresholds.
//!
//! Head turns use rigid yaw (degrees). Expressions use Euclidean landmark
//! ratios normalized by interocular distance so camera distance cancels.

use super::challenge_kind::ChallengeKind;
use super::landmark_types::{LandmarkFrame, MeshBlendProxies, PadLandmarkId};
use super::rigid_head_pose::HeadPose;

/// Default yaw gate for "look left/right" challenges (degrees).
pub const DEFAULT_YAW_THRESHOLD_DEG: f32 = 25.0;
/// Mouth open / smile mouth height ratio (mouth gap / interocular).
pub const DEFAULT_MOUTH_OPEN_RATIO: f32 = 0.22;
/// Smile blendshape proxy peak when blendshapes available.
pub const DEFAULT_SMILE_BLEND: f32 = 0.35;
/// Blink blendshape peak.
pub const DEFAULT_BLINK_PEAK: f32 = 0.40;

#[derive(Debug, Clone, Copy)]
pub struct ActionThresholds {
    pub yaw_deg: f32,
    pub mouth_open_ratio: f32,
    pub smile_blend: f32,
    pub blink_peak: f32,
}

impl Default for ActionThresholds {
    fn default() -> Self {
        Self {
            yaw_deg: DEFAULT_YAW_THRESHOLD_DEG,
            mouth_open_ratio: DEFAULT_MOUTH_OPEN_RATIO,
            smile_blend: DEFAULT_SMILE_BLEND,
            blink_peak: DEFAULT_BLINK_PEAK,
        }
    }
}

/// Mouth aperture normalized by interocular (None if landmarks missing).
pub fn mouth_open_ratio(frame: &LandmarkFrame) -> Option<f32> {
    let upper = frame.get(PadLandmarkId::UpperLip)?;
    let lower = frame.get(PadLandmarkId::LowerLip)?;
    let iod = frame.interocular()?;
    Some(upper.dist(lower) / iod)
}

/// Whether the action criterion is met for this frame's pose + optional proxies.
pub fn action_met_at_frame(
    challenge: ChallengeKind,
    pose: HeadPose,
    pose0: HeadPose,
    frame: &LandmarkFrame,
    blend: Option<MeshBlendProxies>,
    thr: &ActionThresholds,
) -> bool {
    match challenge {
        ChallengeKind::YawLeft => (pose.yaw_deg - pose0.yaw_deg) >= thr.yaw_deg,
        ChallengeKind::YawRight => (pose0.yaw_deg - pose.yaw_deg) >= thr.yaw_deg,
        ChallengeKind::Smile => {
            let geo = mouth_open_ratio(frame).map(|r| r >= thr.mouth_open_ratio * 0.85);
            let bl = blend.map(|b| b.smile >= thr.smile_blend);
            matches!((geo, bl), (Some(true), _) | (_, Some(true)))
        }
        ChallengeKind::BlinkTwice => {
            // Per-frame blink peak; event counting is done by the orchestrator.
            blend.map(|b| b.blink >= thr.blink_peak).unwrap_or(false)
        }
        ChallengeKind::OpenMouth => mouth_open_ratio(frame)
            .map(|r| r >= thr.mouth_open_ratio)
            .unwrap_or(false),
        ChallengeKind::PitchUp => (pose0.pitch_deg - pose.pitch_deg) >= thr.yaw_deg * 0.7,
        ChallengeKind::PitchDown => (pose.pitch_deg - pose0.pitch_deg) >= thr.yaw_deg * 0.7,
    }
}

/// Onset: motion in the challenge direction exceeds a fraction of the threshold.
pub fn action_onset_at_frame(
    challenge: ChallengeKind,
    pose: HeadPose,
    pose0: HeadPose,
    thr: &ActionThresholds,
) -> bool {
    let frac = 0.15;
    match challenge {
        ChallengeKind::YawLeft => (pose.yaw_deg - pose0.yaw_deg) >= thr.yaw_deg * frac,
        ChallengeKind::YawRight => (pose0.yaw_deg - pose.yaw_deg) >= thr.yaw_deg * frac,
        ChallengeKind::PitchUp => (pose0.pitch_deg - pose.pitch_deg) >= thr.yaw_deg * frac * 0.7,
        ChallengeKind::PitchDown => (pose.pitch_deg - pose0.pitch_deg) >= thr.yaw_deg * frac * 0.7,
        ChallengeKind::Smile | ChallengeKind::OpenMouth | ChallengeKind::BlinkTwice => {
            // Expression onset handled via blend/geometry deltas in orchestrator.
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

    #[test]
    fn yaw_left_threshold() {
        let pose0 = HeadPose {
            yaw_deg: 0.0,
            ..Default::default()
        };
        let pose = HeadPose {
            yaw_deg: 30.0,
            ..Default::default()
        };
        let f = LandmarkFrame::empty(0);
        assert!(action_met_at_frame(
            ChallengeKind::YawLeft,
            pose,
            pose0,
            &f,
            None,
            &ActionThresholds::default()
        ));
    }

    #[test]
    fn mouth_ratio_normalized() {
        let mut f = LandmarkFrame::empty(0);
        f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(0.0, 0.0));
        f.set(PadLandmarkId::RightEyeOuter, Landmark2::new(100.0, 0.0));
        f.set(PadLandmarkId::UpperLip, Landmark2::new(50.0, 40.0));
        f.set(PadLandmarkId::LowerLip, Landmark2::new(50.0, 70.0));
        let r = mouth_open_ratio(&f).unwrap();
        assert!((r - 0.30).abs() < 1e-4);
    }
}
