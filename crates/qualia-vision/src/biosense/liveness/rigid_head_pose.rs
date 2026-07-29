//! Rigid head pose from a stable landmark subset (PnP-class geometry).
//!
//! Maps nose tip, chin, and outer eye corners to a generic 3D canonical face
//! under weak-perspective alignment, then extracts pitch / yaw / roll in
//! degrees. Coordinates are scale-normalized by interocular distance so
//! camera distance does not look like structural change.
//!
//! Pure Rust — no OpenCV product ABI. Full EPnP is not required for PAD
//! thresholds; this solve is the geometric lock for challenge validation.

use super::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

/// Generic 3D face model (mm-scale), camera looking −Z, Y down.
const MODEL_NOSE: [f32; 3] = [0.0, 0.0, 0.0];
const MODEL_CHIN: [f32; 3] = [0.0, 63.0, -12.0];
const MODEL_LEFT_EYE: [f32; 3] = [-35.0, -32.0, -26.0];
const MODEL_RIGHT_EYE: [f32; 3] = [35.0, -32.0, -26.0];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeadPose {
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub roll_deg: f32,
    /// Interocular distance used for scale (pixels or same units as landmarks).
    pub scale: f32,
}

/// Estimate rigid head pose from pose-core landmarks.
///
/// Returns `None` if the pose core is incomplete or degenerate.
pub fn estimate_head_pose(frame: &LandmarkFrame) -> Option<HeadPose> {
    let nose = frame.get(PadLandmarkId::NoseTip)?;
    let chin = frame.get(PadLandmarkId::Chin)?;
    let le = frame.get(PadLandmarkId::LeftEyeOuter)?;
    let re = frame.get(PadLandmarkId::RightEyeOuter)?;
    let iod = le.dist(re);
    if iod < 1e-4 {
        return None;
    }

    // Weak-perspective: center 2D/3D, scale 2D by IOD and 3D by model IOD.
    let eye_c = le.midpoint(re);
    let model_eye_c = [
        0.5 * (MODEL_LEFT_EYE[0] + MODEL_RIGHT_EYE[0]),
        0.5 * (MODEL_LEFT_EYE[1] + MODEL_RIGHT_EYE[1]),
        0.5 * (MODEL_LEFT_EYE[2] + MODEL_RIGHT_EYE[2]),
    ];
    let model_iod = {
        let dx = MODEL_LEFT_EYE[0] - MODEL_RIGHT_EYE[0];
        let dy = MODEL_LEFT_EYE[1] - MODEL_RIGHT_EYE[1];
        let dz = MODEL_LEFT_EYE[2] - MODEL_RIGHT_EYE[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    if model_iod < 1e-4 {
        return None;
    }

    // Geometric euler proxies (normalized, distance-invariant).
    // Yaw: horizontal nose offset from eye midline relative to IOD.
    // Positive yaw = subject looking left (nose shifts toward image-right when facing camera).
    let yaw_n = (nose.x - eye_c.x) / iod;
    // Pitch: nose vs eye-chin vertical structure.
    let face_h = (chin.y - eye_c.y).abs().max(1e-4);
    let pitch_n = ((nose.y - eye_c.y) / face_h) - 0.35; // frontal bias ≈ 0
                                                        // Roll: eye-line angle.
    let roll_rad = (re.y - le.y).atan2(re.x - le.x);

    // Scale to degrees with stable gains (production-calibrate in MANIFEST).
    let mut pose = HeadPose {
        pitch_deg: (pitch_n * 55.0).clamp(-90.0, 90.0),
        yaw_deg: (yaw_n * 70.0).clamp(-90.0, 90.0),
        roll_deg: roll_rad.to_degrees().clamp(-90.0, 90.0),
        scale: iod,
    };

    // Refine with weak-perspective residual on canonical model (depth cue).
    if let Some(refined) = refine_with_model(nose, chin, le, re, iod, model_eye_c, model_iod) {
        // Blend geometric (stable) with model residual (non-rigid sensitive).
        pose.yaw_deg = 0.65 * pose.yaw_deg + 0.35 * refined.yaw_deg;
        pose.pitch_deg = 0.65 * pose.pitch_deg + 0.35 * refined.pitch_deg;
    }

    Some(pose)
}

fn refine_with_model(
    nose: Landmark2,
    chin: Landmark2,
    le: Landmark2,
    re: Landmark2,
    iod: f32,
    model_eye_c: [f32; 3],
    model_iod: f32,
) -> Option<HeadPose> {
    // Project model XY after centering; residual of nose vs expected frontal
    // projects to yaw/pitch corrections.
    let s = iod / model_iod;
    let eye_c = le.midpoint(re);
    let pred_nose = Landmark2 {
        x: eye_c.x + s * (MODEL_NOSE[0] - model_eye_c[0]),
        y: eye_c.y + s * (MODEL_NOSE[1] - model_eye_c[1]),
    };
    let pred_chin = Landmark2 {
        x: eye_c.x + s * (MODEL_CHIN[0] - model_eye_c[0]),
        y: eye_c.y + s * (MODEL_CHIN[1] - model_eye_c[1]),
    };
    let dn = Landmark2 {
        x: (nose.x - pred_nose.x) / iod,
        y: (nose.y - pred_nose.y) / iod,
    };
    let dc = Landmark2 {
        x: (chin.x - pred_chin.x) / iod,
        y: (chin.y - pred_chin.y) / iod,
    };
    // Average residual → angle proxies.
    Some(HeadPose {
        yaw_deg: ((dn.x + 0.5 * dc.x) * 80.0).clamp(-90.0, 90.0),
        pitch_deg: ((dn.y + 0.5 * dc.y) * 70.0).clamp(-90.0, 90.0),
        roll_deg: 0.0,
        scale: iod,
    })
}

/// Normalize pose deltas so absolute camera distance does not dominate.
pub fn pose_delta(a: HeadPose, b: HeadPose) -> HeadPose {
    HeadPose {
        pitch_deg: b.pitch_deg - a.pitch_deg,
        yaw_deg: b.yaw_deg - a.yaw_deg,
        roll_deg: b.roll_deg - a.roll_deg,
        scale: b.scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

    fn frontal() -> LandmarkFrame {
        let mut f = LandmarkFrame::empty(0);
        f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(100.0, 120.0));
        f.set(PadLandmarkId::RightEyeOuter, Landmark2::new(180.0, 120.0));
        f.set(PadLandmarkId::NoseTip, Landmark2::new(140.0, 150.0));
        f.set(PadLandmarkId::Chin, Landmark2::new(140.0, 210.0));
        f
    }

    #[test]
    fn frontal_near_zero_yaw() {
        let p = estimate_head_pose(&frontal()).unwrap();
        assert!(p.yaw_deg.abs() < 8.0, "yaw={}", p.yaw_deg);
        assert!(p.scale > 0.0);
    }

    #[test]
    fn left_yaw_positive() {
        let mut f = frontal();
        // Subject looks left → nose toward image-right (facing camera).
        f.set(PadLandmarkId::NoseTip, Landmark2::new(170.0, 150.0));
        let p = estimate_head_pose(&f).unwrap();
        assert!(
            p.yaw_deg > 5.0,
            "expected positive left yaw, got {}",
            p.yaw_deg
        );
    }
}
