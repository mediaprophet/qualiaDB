//! Non-rigid occlusion gate for 2D mask rejection.
//!
//! **Does not use model-inferred Z.** MediaPipe depth is a statistical prior
//! and will pass flat-screen attacks. The real lock is
//! [`profile_asymmetry_ratio`](crate::biosense::liveness::profile_asymmetry_ratio)
//! — Profile Asymmetry Ratio on raw image \(x\) of landmarks 1 / 234 / 454.
//!
//! This module is a thin compatibility façade over PAR so existing call sites
//! (`evaluate_non_rigid_z`, `NonRigidVerdict`) keep working.

use super::landmark_types::LandmarkFrame;
use super::profile_asymmetry_ratio::{
    evaluate_profile_asymmetry, ParVerdict, DEFAULT_PAR_TAU, MIN_YAW_SPAN_DEG,
};
use super::rigid_head_pose::HeadPose;

/// Default τ for baseline-normalized ΔPAR (see PAR module).
pub const DEFAULT_MIN_NONRIGID_SCORE: f32 = DEFAULT_PAR_TAU;

pub use super::profile_asymmetry_ratio::MIN_YAW_SPAN_DEG as MIN_Z_YAW_SPAN_DEG;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NonRigidVerdict {
    /// PAR occlusion spike — live 3D head in image space.
    Live3d { score: f32 },
    /// ΔPAR ≈ 0 under yaw — planar / flat surface.
    FlatSurface { score: f32 },
    /// Yaw span &lt; 25° or too few samples.
    InsufficientMotion,
    /// Missing nose / left / right edge landmarks.
    MissingLandmarks,
}

/// Evaluate the flat-mask lock via **PAR only** (raw 2D \(x\)).
///
/// `min_score` is τ for `|PAR(t1)/PAR(t0) - 1|` (default 0.6).
/// `poses` select peak yaw and enforce ≥ 25° span — not a Z source.
pub fn evaluate_non_rigid_z(
    frames: &[LandmarkFrame],
    poses: &[HeadPose],
    min_score: f32,
) -> NonRigidVerdict {
    match evaluate_profile_asymmetry(frames, poses, min_score) {
        ParVerdict::Live3d { delta_par, .. } => NonRigidVerdict::Live3d { score: delta_par },
        ParVerdict::FlatSurface { delta_par, .. } => {
            NonRigidVerdict::FlatSurface { score: delta_par }
        }
        ParVerdict::InsufficientYaw { .. } | ParVerdict::InsufficientSamples => {
            NonRigidVerdict::InsufficientMotion
        }
        ParVerdict::MissingLandmarks => NonRigidVerdict::MissingLandmarks,
    }
}

/// Re-export span constant for orchestrator docs.
pub const REQUIRED_YAW_FOR_PAR: f32 = MIN_YAW_SPAN_DEG;

#[cfg(test)]
pub use super::profile_asymmetry_ratio::{synthetic_3d_par_frame, synthetic_flat_par_frame};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::profile_asymmetry_ratio::{
        synthetic_3d_par_frame, synthetic_flat_par_frame,
    };

    #[test]
    fn flat_surface_rejected_via_par() {
        let mut frames = [LandmarkFrame::empty(0); 8];
        let mut poses = [HeadPose::default(); 8];
        for i in 0..8 {
            let (f, p) = synthetic_flat_par_frame(i as f32 * 5.0, i as u32 * 100);
            frames[i] = f;
            poses[i] = p;
        }
        let v = evaluate_non_rigid_z(&frames, &poses, DEFAULT_MIN_NONRIGID_SCORE);
        assert!(
            matches!(v, NonRigidVerdict::FlatSurface { .. }),
            "got {:?}",
            v
        );
    }

    #[test]
    fn live_3d_accepted_via_par() {
        let mut frames = [LandmarkFrame::empty(0); 8];
        let mut poses = [HeadPose::default(); 8];
        for i in 0..8 {
            let (f, p) = synthetic_3d_par_frame(i as f32 * 5.0, i as u32 * 100);
            frames[i] = f;
            poses[i] = p;
        }
        let v = evaluate_non_rigid_z(&frames, &poses, DEFAULT_MIN_NONRIGID_SCORE);
        assert!(matches!(v, NonRigidVerdict::Live3d { score } if score > 0.6), "got {:?}", v);
    }
}
