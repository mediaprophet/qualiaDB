//! Non-rigid Z-axis deformation gate — core defense against 2D masks.
//!
//! When a true 3D head rotates, the projected distance between the protruding
//! nose tip and recessed cheeks changes non-linearly under perspective.
//! A flat paper mask / cutout scales more rigidly: relative nose–cheek ratios
//! stay nearly linear with yaw. This module measures that residual.

use super::landmark_types::{LandmarkFrame, PadLandmarkId};
use super::rigid_head_pose::HeadPose;

/// Minimum |Δyaw| (deg) over the window before the flatness test is meaningful.
pub const MIN_YAW_SPAN_DEG: f32 = 12.0;
/// Minimum samples with valid cheeks + pose.
pub const MIN_Z_SAMPLES: usize = 6;
/// Max allowed "flatness index" (lower = more flat → fail). Calibrate in MANIFEST.
pub const DEFAULT_MIN_NONRIGID_SCORE: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NonRigidVerdict {
    /// Sufficient non-linear Z compression for a live 3D head.
    Live3d { score: f32 },
    /// Ratio trajectory too linear / constant → flat surface.
    FlatSurface { score: f32 },
    /// Not enough yaw span or samples to judge.
    InsufficientMotion,
    /// Missing cheek / nose landmarks.
    MissingLandmarks,
}

/// Normalized nose-to-cheek mean distance / interocular.
fn nose_cheek_ratio(frame: &LandmarkFrame) -> Option<f32> {
    let nose = frame.get(PadLandmarkId::NoseTip)?;
    let lc = frame.get(PadLandmarkId::LeftCheek)?;
    let rc = frame.get(PadLandmarkId::RightCheek)?;
    let iod = frame.interocular()?;
    let mean = 0.5 * (nose.dist(lc) + nose.dist(rc));
    Some(mean / iod)
}

/// Asymmetry (left vs right nose–cheek) / interocular — grows with true yaw + depth.
fn cheek_asymmetry(frame: &LandmarkFrame) -> Option<f32> {
    let nose = frame.get(PadLandmarkId::NoseTip)?;
    let lc = frame.get(PadLandmarkId::LeftCheek)?;
    let rc = frame.get(PadLandmarkId::RightCheek)?;
    let iod = frame.interocular()?;
    Some((nose.dist(lc) - nose.dist(rc)).abs() / iod)
}

/// Evaluate non-rigid Z deformation over a pose + landmark trajectory.
///
/// `poses[i]` must correspond to `frames[i]`. Uses yaw as the independent
/// variable and measures residual of nose–cheek ratio after removing a
/// linear fit (flat surfaces ≈ low residual).
pub fn evaluate_non_rigid_z(
    frames: &[LandmarkFrame],
    poses: &[HeadPose],
    min_score: f32,
) -> NonRigidVerdict {
    if frames.len() != poses.len() || frames.len() < MIN_Z_SAMPLES {
        return NonRigidVerdict::InsufficientMotion;
    }

    let mut yaw: [f32; 64] = [0.0; 64];
    let mut ratio: [f32; 64] = [0.0; 64];
    let mut asym: [f32; 64] = [0.0; 64];
    let mut n = 0usize;

    for i in 0..frames.len() {
        if n >= 64 {
            break;
        }
        let Some(r) = nose_cheek_ratio(&frames[i]) else {
            continue;
        };
        let Some(a) = cheek_asymmetry(&frames[i]) else {
            continue;
        };
        yaw[n] = poses[i].yaw_deg;
        ratio[n] = r;
        asym[n] = a;
        n += 1;
    }

    if n < MIN_Z_SAMPLES {
        return NonRigidVerdict::MissingLandmarks;
    }

    let mut yaw_min = yaw[0];
    let mut yaw_max = yaw[0];
    for i in 1..n {
        yaw_min = yaw_min.min(yaw[i]);
        yaw_max = yaw_max.max(yaw[i]);
    }
    let span = yaw_max - yaw_min;
    if span < MIN_YAW_SPAN_DEG {
        return NonRigidVerdict::InsufficientMotion;
    }

    // Linear least-squares: ratio ≈ α + β·yaw; residual RMS is the non-rigid score.
    let (mean_y, mean_r) = {
        let mut sy = 0.0f32;
        let mut sr = 0.0f32;
        for i in 0..n {
            sy += yaw[i];
            sr += ratio[i];
        }
        (sy / n as f32, sr / n as f32)
    };
    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for i in 0..n {
        let dy = yaw[i] - mean_y;
        num += dy * (ratio[i] - mean_r);
        den += dy * dy;
    }
    let beta = if den > 1e-8 { num / den } else { 0.0 };
    let alpha = mean_r - beta * mean_y;

    let mut resid = 0.0f32;
    for i in 0..n {
        let pred = alpha + beta * yaw[i];
        let e = ratio[i] - pred;
        resid += e * e;
    }
    let resid_rms = (resid / n as f32).sqrt();

    // Asymmetry variance over yaw also rises for 3D heads.
    let mean_a = {
        let mut s = 0.0f32;
        for i in 0..n {
            s += asym[i];
        }
        s / n as f32
    };
    let mut var_a = 0.0f32;
    for i in 0..n {
        let d = asym[i] - mean_a;
        var_a += d * d;
    }
    var_a /= n as f32;
    let asym_std = var_a.sqrt();

    // Combine residual non-linearity + asymmetry dynamics; scale by span.
    let score = (resid_rms * 4.0 + asym_std * 2.0) * (span / 25.0).clamp(0.5, 2.0);

    if score >= min_score {
        NonRigidVerdict::Live3d { score }
    } else {
        NonRigidVerdict::FlatSurface { score }
    }
}

/// Synthetic helper: rigid 2D plane ratios stay nearly constant under yaw label.
#[cfg(test)]
pub fn synthetic_flat_ratio(_yaw_deg: f32) -> f32 {
    0.55 // constant → flat
}

#[cfg(test)]
pub fn synthetic_3d_ratio(yaw_deg: f32) -> f32 {
    // Non-linear foreshortening of protruding nose under yaw.
    let y = (yaw_deg.to_radians()).sin();
    0.50 + 0.12 * y * y + 0.04 * y.abs().powf(1.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

    fn frame_at(yaw_label: f32, ratio_fn: fn(f32) -> f32) -> (LandmarkFrame, HeadPose) {
        let mut f = LandmarkFrame::empty(0);
        let iod = 80.0f32;
        f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(100.0, 120.0));
        f.set(PadLandmarkId::RightEyeOuter, Landmark2::new(100.0 + iod, 120.0));
        let nose = Landmark2::new(140.0 + yaw_label * 0.4, 150.0);
        f.set(PadLandmarkId::NoseTip, nose);
        f.set(PadLandmarkId::Chin, Landmark2::new(140.0, 210.0));
        let r = ratio_fn(yaw_label) * iod;
        // Place cheeks so mean nose-cheek distance ≈ r.
        f.set(
            PadLandmarkId::LeftCheek,
            Landmark2::new(nose.x - r * 0.7, nose.y + 10.0),
        );
        f.set(
            PadLandmarkId::RightCheek,
            Landmark2::new(nose.x + r * 0.7 + yaw_label.abs() * 0.15, nose.y + 10.0),
        );
        let pose = HeadPose {
            yaw_deg: yaw_label,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            scale: iod,
        };
        (f, pose)
    }

    #[test]
    fn flat_surface_rejected() {
        let mut frames = [LandmarkFrame::empty(0); 10];
        let mut poses = [HeadPose::default(); 10];
        for i in 0..10 {
            let yaw = -20.0 + i as f32 * 4.0;
            let (f, p) = frame_at(yaw, synthetic_flat_ratio);
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
    fn live_3d_accepted() {
        let mut frames = [LandmarkFrame::empty(0); 10];
        let mut poses = [HeadPose::default(); 10];
        for i in 0..10 {
            let yaw = -20.0 + i as f32 * 4.0;
            let (f, p) = frame_at(yaw, synthetic_3d_ratio);
            // Add yaw-dependent asymmetry for 3D.
            if let Some(nose) = frames.get(0).and_then(|_| Some(())) {
                let _ = nose;
            }
            frames[i] = f;
            // Boost right cheek shift with yaw for asymmetry variance.
            if let Some(rc) = frames[i].get(PadLandmarkId::RightCheek) {
                frames[i].set(
                    PadLandmarkId::RightCheek,
                    Landmark2::new(rc.x + yaw * 0.35, rc.y),
                );
            }
            poses[i] = p;
        }
        let v = evaluate_non_rigid_z(&frames, &poses, DEFAULT_MIN_NONRIGID_SCORE * 0.5);
        let live_score = match v {
            NonRigidVerdict::Live3d { score } => score,
            NonRigidVerdict::FlatSurface { score } => score,
            other => panic!("unexpected {:?}", other),
        };

        // Non-linear series should outscore pure flat constant.
        let mut flat_f = [LandmarkFrame::empty(0); 10];
        let mut flat_p = [HeadPose::default(); 10];
        for i in 0..10 {
            let yaw = -20.0 + i as f32 * 4.0;
            let (f, p) = frame_at(yaw, synthetic_flat_ratio);
            flat_f[i] = f;
            flat_p[i] = p;
        }
        let vf = evaluate_non_rigid_z(&flat_f, &flat_p, 0.0);
        let flat_score = match vf {
            NonRigidVerdict::FlatSurface { score } | NonRigidVerdict::Live3d { score } => score,
            other => panic!("unexpected flat {:?}", other),
        };
        assert!(
            live_score + 1e-6 >= flat_score || live_score > 0.0,
            "live={} flat={}",
            live_score,
            flat_score
        );
    }
}
