//! Profile Asymmetry Ratio (PAR) — geometric lock against 2D rigid planes.
//!
//! # Critical trap: never trust model Z
//!
//! MediaPipe (and similar) *infer* depth from a monocular 2D image using
//! statistical priors. A flat iPad showing a face will yield a "perfect"
//! hallucinated mesh Z. Validating on model Z lets the attack pass.
//!
//! This module uses **only raw image-space \(x\) coordinates** of three
//! landmarks (MediaPipe Face Mesh indices):
//!
//! | Role | Slot | MediaPipe index |
//! |------|------|-----------------|
//! | Nose tip \(N\) | [`PadLandmarkId::NoseTip`] | **1** |
//! | Left edge \(L\) | [`PadLandmarkId::LeftCheek`] | **234** |
//! | Right edge \(R\) | [`PadLandmarkId::RightCheek`] | **454** |
//!
//! ## Math
//!
//! \[
//! d_L = \lvert x_N - x_L\rvert,\quad
//! d_R = \lvert x_R - x_N\rvert,\quad
//! PAR = d_L / d_R
//! \]
//!
//! At frontal \(t_0\): \(PAR \approx 1\). At peak yaw \(t_1\): a true 3D head
//! produces a large \(|PAR(t_1)/PAR(t_0) - 1|\) (occlusion spike). A flat mask
//! scales both sides by \(\cos\theta\), so \(\Delta PAR \approx 0\).
//!
//! Require \(|\Delta yaw| \ge 25^\circ\) so the spike is unambiguous. Baseline
//! PAR at \(t_0\) is normalized before the delta (human faces are asymmetric).

use super::landmark_types::{LandmarkFrame, PadLandmarkId};
use super::rigid_head_pose::HeadPose;

/// Default \(\tau\): minimum baseline-normalized \(|\Delta PAR|\) for live 3D.
/// Empirical; calibrate in MANIFEST for production yaw policy.
pub const DEFAULT_PAR_TAU: f32 = 0.6;

/// Minimum absolute yaw span (degrees) between frontal and peak for a valid test.
pub const MIN_YAW_SPAN_DEG: f32 = 25.0;

/// Minimum frames with valid N/L/R \(x\).
pub const MIN_PAR_SAMPLES: usize = 4;

/// Floor on \(d_R\) (and \(d_L\)) in image units to avoid division blow-up.
const MIN_HORIZ_PX: f32 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParVerdict {
    /// \(\Delta PAR\) exceeds \(\tau\) under sufficient yaw — live 3D occlusion.
    Live3d {
        delta_par: f32,
        par_t0: f32,
        par_t1: f32,
        yaw_span_deg: f32,
    },
    /// \(\Delta PAR\) too small — consistent with planar cosine compression.
    FlatSurface {
        delta_par: f32,
        par_t0: f32,
        par_t1: f32,
        yaw_span_deg: f32,
    },
    InsufficientYaw { yaw_span_deg: f32 },
    InsufficientSamples,
    MissingLandmarks,
}

/// Horizontal distances and PAR from **\(x\) only** (y and model Z ignored).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParSample {
    pub d_l: f32,
    pub d_r: f32,
    pub par: f32,
}

/// \(d_L = |x_N - x_L|\), \(d_R = |x_R - x_N|\), \(PAR = d_L / d_R\).
///
/// Returns `None` if nose / left / right edge missing or degenerate.
pub fn profile_asymmetry_ratio(frame: &LandmarkFrame) -> Option<ParSample> {
    let n = frame.get(PadLandmarkId::NoseTip)?;
    let l = frame.get(PadLandmarkId::LeftCheek)?;
    let r = frame.get(PadLandmarkId::RightCheek)?;
    // Explicitly use .x only — never any z field (Landmark2 has none by design).
    let d_l = (n.x - l.x).abs();
    let d_r = (r.x - n.x).abs();
    if d_l < MIN_HORIZ_PX || d_r < MIN_HORIZ_PX {
        return None;
    }
    Some(ParSample {
        d_l,
        d_r,
        par: d_l / d_r,
    })
}

/// Baseline-normalized absolute delta: \(|PAR(t_1)/PAR(t_0) - 1|\).
#[inline]
pub fn normalized_par_delta(par_t0: f32, par_t1: f32) -> Option<f32> {
    if !par_t0.is_finite() || !par_t1.is_finite() || par_t0.abs() < 1e-6 {
        return None;
    }
    Some((par_t1 / par_t0 - 1.0).abs())
}

/// Evaluate PAR geometric lock over a landmark + pose trajectory.
///
/// * \(t_0\) — first valid PAR sample (challenge start / frontal).
/// * \(t_1\) — sample at peak \(|yaw - yaw_{t0}|\) (must reach ≥ [`MIN_YAW_SPAN_DEG`]).
/// * `tau` — minimum normalized \(\Delta PAR\) (default [`DEFAULT_PAR_TAU`] = 0.6).
///
/// `poses[i]` aligns with `frames[i]`. Pose yaw is used only to pick \(t_1\) and
/// enforce span — never as a substitute for PAR, and never from model Z.
pub fn evaluate_profile_asymmetry(
    frames: &[LandmarkFrame],
    poses: &[HeadPose],
    tau: f32,
) -> ParVerdict {
    if frames.len() != poses.len() || frames.is_empty() {
        return ParVerdict::InsufficientSamples;
    }

    // t0: first frame with valid PAR (frontal baseline).
    let mut i0 = None;
    for i in 0..frames.len() {
        if profile_asymmetry_ratio(&frames[i]).is_some() {
            i0 = Some(i);
            break;
        }
    }
    let Some(i0) = i0 else {
        return ParVerdict::MissingLandmarks;
    };
    let par0 = match profile_asymmetry_ratio(&frames[i0]) {
        Some(s) => s.par,
        None => return ParVerdict::MissingLandmarks,
    };
    let yaw0 = poses[i0].yaw_deg;

    // t1: peak |Δyaw| among samples with valid PAR.
    let mut i1 = i0;
    let mut best_abs = 0.0f32;
    let mut valid = 0usize;
    for i in 0..frames.len() {
        if profile_asymmetry_ratio(&frames[i]).is_none() {
            continue;
        }
        valid += 1;
        let ad = (poses[i].yaw_deg - yaw0).abs();
        if ad >= best_abs {
            best_abs = ad;
            i1 = i;
        }
    }

    if valid < MIN_PAR_SAMPLES {
        return ParVerdict::InsufficientSamples;
    }

    let yaw_span_deg = (poses[i1].yaw_deg - yaw0).abs();
    if yaw_span_deg < MIN_YAW_SPAN_DEG {
        return ParVerdict::InsufficientYaw { yaw_span_deg };
    }

    let par1 = match profile_asymmetry_ratio(&frames[i1]) {
        Some(s) => s.par,
        None => return ParVerdict::MissingLandmarks,
    };

    let Some(delta_par) = normalized_par_delta(par0, par1) else {
        return ParVerdict::MissingLandmarks;
    };

    if delta_par > tau {
        ParVerdict::Live3d {
            delta_par,
            par_t0: par0,
            par_t1: par1,
            yaw_span_deg,
        }
    } else {
        ParVerdict::FlatSurface {
            delta_par,
            par_t0: par0,
            par_t1: par1,
            yaw_span_deg,
        }
    }
}

// ── Synthetic geometry for unit tests (image x only) ─────────────────────

/// Flat plane: both sides scale by \(\cos\theta\); PAR stays ≈ constant.
/// Nose shifts so pose estimation still reports yaw (ratio remains cosine-equal).
#[cfg(test)]
pub fn synthetic_flat_par_frame(yaw_deg: f32, t_ms: u32) -> (LandmarkFrame, HeadPose) {
    use super::landmark_types::Landmark2;
    let cos = yaw_deg.to_radians().cos().abs().max(0.15);
    // Stable eye line (IOD) for rigid pose; plane width compresses with cos.
    let eye_l = 160.0f32;
    let eye_r = 240.0f32;
    let eye_c = 0.5 * (eye_l + eye_r);
    // Slight nose shift for pose; both cheek half-widths = same cos scale → PAR fixed.
    let nose_x = eye_c + yaw_deg * 0.55;
    let half = 40.0 * cos;
    let mut f = LandmarkFrame::empty(t_ms);
    f.set(PadLandmarkId::NoseTip, Landmark2::new(nose_x, 150.0));
    // Equal horizontal distances after accounting for nose offset: place edges
    // so d_L == d_R == half (PAR = 1 for all yaw).
    f.set(PadLandmarkId::LeftCheek, Landmark2::new(nose_x - half, 160.0));
    f.set(PadLandmarkId::RightCheek, Landmark2::new(nose_x + half, 160.0));
    f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(eye_l, 120.0));
    f.set(PadLandmarkId::RightEyeOuter, Landmark2::new(eye_r, 120.0));
    f.set(PadLandmarkId::Chin, Landmark2::new(nose_x, 210.0));
    (
        f,
        HeadPose {
            yaw_deg,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            scale: eye_r - eye_l,
        },
    )
}

/// 3D head: nose occludes receding cheek — one horizontal side collapses.
/// Eyes stay on a stable IOD line so PnP-class yaw tracks the labeled turn.
#[cfg(test)]
pub fn synthetic_3d_par_frame(yaw_deg: f32, t_ms: u32) -> (LandmarkFrame, HeadPose) {
    use super::landmark_types::Landmark2;
    let mut f = LandmarkFrame::empty(t_ms);
    let eye_l = 160.0f32;
    let eye_r = 240.0f32;
    let eye_c = 0.5 * (eye_l + eye_r);
    // Nose migrates with yaw (subject look-left → +x) relative to fixed eyes.
    let nose_x = eye_c + yaw_deg * 0.9;
    // Receding side collapses toward nose; advancing side stays wide.
    // Positive yaw: left is receding → d_L shrinks, d_R stays large → PAR ↓.
    let collapse = (yaw_deg.abs() / 30.0).clamp(0.0, 1.0);
    let d_recede = 40.0 * (1.0 - 0.85 * collapse).max(0.08);
    let d_advance = 40.0 * (1.0 + 0.15 * collapse);
    let (d_l, d_r) = if yaw_deg >= 0.0 {
        (d_recede, d_advance)
    } else {
        (d_advance, d_recede)
    };
    f.set(PadLandmarkId::NoseTip, Landmark2::new(nose_x, 150.0));
    f.set(PadLandmarkId::LeftCheek, Landmark2::new(nose_x - d_l, 160.0));
    f.set(PadLandmarkId::RightCheek, Landmark2::new(nose_x + d_r, 160.0));
    f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(eye_l, 120.0));
    f.set(PadLandmarkId::RightEyeOuter, Landmark2::new(eye_r, 120.0));
    f.set(PadLandmarkId::Chin, Landmark2::new(nose_x * 0.15 + eye_c * 0.85, 210.0));
    (
        f,
        HeadPose {
            yaw_deg,
            pitch_deg: 0.0,
            roll_deg: 0.0,
            scale: eye_r - eye_l,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::Landmark2;

    #[test]
    fn par_frontal_near_one() {
        let mut f = LandmarkFrame::empty(0);
        f.set(PadLandmarkId::NoseTip, Landmark2::new(140.0, 150.0));
        f.set(PadLandmarkId::LeftCheek, Landmark2::new(100.0, 160.0));
        f.set(PadLandmarkId::RightCheek, Landmark2::new(180.0, 160.0));
        let s = profile_asymmetry_ratio(&f).unwrap();
        assert!((s.par - 1.0).abs() < 1e-5, "par={}", s.par);
        assert!((s.d_l - 40.0).abs() < 1e-5);
        assert!((s.d_r - 40.0).abs() < 1e-5);
    }

    #[test]
    fn par_uses_x_only() {
        // Same x, different y → identical PAR (y must not affect).
        let mut a = LandmarkFrame::empty(0);
        a.set(PadLandmarkId::NoseTip, Landmark2::new(50.0, 10.0));
        a.set(PadLandmarkId::LeftCheek, Landmark2::new(10.0, 999.0));
        a.set(PadLandmarkId::RightCheek, Landmark2::new(90.0, -50.0));
        let mut b = LandmarkFrame::empty(0);
        b.set(PadLandmarkId::NoseTip, Landmark2::new(50.0, 500.0));
        b.set(PadLandmarkId::LeftCheek, Landmark2::new(10.0, 0.0));
        b.set(PadLandmarkId::RightCheek, Landmark2::new(90.0, 0.0));
        assert_eq!(
            profile_asymmetry_ratio(&a).unwrap().par,
            profile_asymmetry_ratio(&b).unwrap().par
        );
    }

    #[test]
    fn flat_mask_delta_near_zero() {
        let mut frames = [LandmarkFrame::empty(0); 8];
        let mut poses = [HeadPose::default(); 8];
        for i in 0..8 {
            let yaw = i as f32 * 5.0; // 0 → 35°
            let (f, p) = synthetic_flat_par_frame(yaw, i as u32 * 100);
            frames[i] = f;
            poses[i] = p;
        }
        let v = evaluate_profile_asymmetry(&frames, &poses, DEFAULT_PAR_TAU);
        match v {
            ParVerdict::FlatSurface { delta_par, .. } => {
                assert!(delta_par < 0.15, "flat delta_par={}", delta_par);
            }
            other => panic!("expected FlatSurface, got {:?}", other),
        }
    }

    #[test]
    fn live_3d_delta_exceeds_tau() {
        let mut frames = [LandmarkFrame::empty(0); 8];
        let mut poses = [HeadPose::default(); 8];
        for i in 0..8 {
            let yaw = i as f32 * 5.0; // 0 → 35°
            let (f, p) = synthetic_3d_par_frame(yaw, i as u32 * 100);
            frames[i] = f;
            poses[i] = p;
        }
        let v = evaluate_profile_asymmetry(&frames, &poses, DEFAULT_PAR_TAU);
        match v {
            ParVerdict::Live3d { delta_par, .. } => {
                assert!(delta_par > DEFAULT_PAR_TAU, "delta_par={}", delta_par);
            }
            other => panic!("expected Live3d, got {:?}", other),
        }
    }

    #[test]
    fn insufficient_yaw_rejected() {
        let mut frames = [LandmarkFrame::empty(0); 6];
        let mut poses = [HeadPose::default(); 6];
        for i in 0..6 {
            let yaw = i as f32 * 2.0; // max 10° < 25°
            let (f, p) = synthetic_3d_par_frame(yaw, i as u32 * 100);
            frames[i] = f;
            poses[i] = p;
        }
        let v = evaluate_profile_asymmetry(&frames, &poses, DEFAULT_PAR_TAU);
        assert!(matches!(v, ParVerdict::InsufficientYaw { .. }), "got {:?}", v);
    }

    #[test]
    fn mediapipe_indices_documented() {
        assert_eq!(PadLandmarkId::NoseTip.mediapipe_index(), 1);
        assert_eq!(PadLandmarkId::LeftCheek.mediapipe_index(), 234);
        assert_eq!(PadLandmarkId::RightCheek.mediapipe_index(), 454);
    }
}
