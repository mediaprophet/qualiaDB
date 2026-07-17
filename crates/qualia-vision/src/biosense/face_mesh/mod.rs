//! MediaPipe Face Mesh layout adapters for pure-landmark PAD.
//!
//! Packs full-mesh flat buffers into the eight-slot [`LandmarkFrame`] used by
//! challenge-response PAD. **Never uses model Z** — only \(x,y\).
//!
//! No ONNX / MediaPipe runtime here (that is Track S2). Buffer → geometry only.

pub mod evaluate_pad_from_mediapipe_trace;
pub mod landmarks_from_normalized;
pub mod mediapipe_index;
pub mod pack_landmark_frame;

pub use evaluate_pad_from_mediapipe_trace::{
    evaluate_pad_from_mediapipe_trace, MAX_MEDIAPIPE_PAD_FRAMES,
};
pub use landmarks_from_normalized::landmarks_from_normalized;
pub use mediapipe_index::{
    mediapipe_index_for_pad, pad_id_for_mediapipe_index, PAD_LANDMARK_IDS, PAD_MEDIAPIPE_INDICES,
    MEDIAPIPE_FACE_MESH_COUNT,
};
pub use pack_landmark_frame::{pack_landmark_frame, LandmarkBufferLayout};

#[cfg(test)]
mod trace_tests {
    use super::*;
    use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};
    use crate::biosense::liveness::camera_stream_integrity::CameraStreamAttestation;
    use crate::biosense::liveness::challenge_kind::ChallengeKind;
    use crate::biosense::liveness::challenge_pad::{PadReason, PadThresholds};
    use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};
    use crate::biosense::liveness::non_rigid_z::DEFAULT_MIN_NONRIGID_SCORE;
    use crate::biosense::liveness::profile_asymmetry_ratio::{
        synthetic_3d_par_frame, synthetic_flat_par_frame,
    };

    fn consent() -> BiosenseConsent {
        BiosenseConsent::grant_security_template(1)
    }

    /// Scatter a PAD frame into a full MediaPipe xy flat buffer (other indices 0).
    fn frame_to_mp_xy(frame: &LandmarkFrame) -> Vec<f32> {
        let mut buf = vec![0.0f32; MEDIAPIPE_FACE_MESH_COUNT * 2];
        for id in PAD_LANDMARK_IDS {
            if let Some(p) = frame.get(id) {
                let i = id.mediapipe_index() as usize;
                buf[i * 2] = p.x;
                buf[i * 2 + 1] = p.y;
            }
        }
        buf
    }

    /// Same as xy but with a large fake Z so tests prove Z is ignored.
    fn frame_to_mp_xyz_with_poison_z(frame: &LandmarkFrame) -> Vec<f32> {
        let mut buf = vec![0.0f32; MEDIAPIPE_FACE_MESH_COUNT * 3];
        for id in PAD_LANDMARK_IDS {
            if let Some(p) = frame.get(id) {
                let i = id.mediapipe_index() as usize;
                buf[i * 3] = p.x;
                buf[i * 3 + 1] = p.y;
                buf[i * 3 + 2] = 1.0e6; // poison Z — must not affect PAD
            }
        }
        buf
    }

    fn yaw_mp_trace(live3d: bool) -> (Vec<u32>, Vec<Vec<f32>>) {
        let mut times = Vec::new();
        let mut flats = Vec::new();
        for i in 0..12 {
            let t = 40 + i as u32 * 80;
            let yaw = i as f32 * 4.0; // 0 → 44°
            let (mut f, _) = if live3d {
                synthetic_3d_par_frame(yaw, t)
            } else {
                synthetic_flat_par_frame(yaw, t)
            };
            f.t_ms = t;
            if live3d {
                let j = ((i as f32) * 2.1).sin() * 0.45;
                if let Some(n) = f.get(PadLandmarkId::NoseTip) {
                    f.set(
                        PadLandmarkId::NoseTip,
                        Landmark2::new(n.x + j, n.y + j * 0.3),
                    );
                }
            }
            times.push(t);
            flats.push(frame_to_mp_xy(&f));
        }
        (times, flats)
    }

    #[test]
    fn pack_roundtrip_preserves_par_slots() {
        let (f, _) = synthetic_3d_par_frame(20.0, 100);
        let flat = frame_to_mp_xy(&f);
        let packed =
            pack_landmark_frame(100, &flat, LandmarkBufferLayout::XyInterleaved).unwrap();
        for id in [
            PadLandmarkId::NoseTip,
            PadLandmarkId::LeftCheek,
            PadLandmarkId::RightCheek,
            PadLandmarkId::LeftEyeOuter,
            PadLandmarkId::RightEyeOuter,
            PadLandmarkId::Chin,
        ] {
            let a = f.get(id).unwrap();
            let b = packed.get(id).unwrap();
            assert!((a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5, "{:?}", id);
        }
    }

    #[test]
    fn flat_par_via_mediapipe_trace_fails() {
        let (times, flats) = yaw_mp_trace(false);
        let refs: Vec<&[f32]> = flats.iter().map(|v| v.as_slice()).collect();
        let mut thr = PadThresholds::default();
        thr.require_jitter = false;
        thr.min_nonrigid_score = DEFAULT_MIN_NONRIGID_SCORE;
        thr.action.yaw_deg = 15.0;
        let r = evaluate_pad_from_mediapipe_trace(
            consent(),
            ChallengeKind::YawLeft,
            LandmarkBufferLayout::XyInterleaved,
            &times,
            &refs,
            &[],
            CameraStreamAttestation::default(),
            &thr,
            None,
        )
        .unwrap();
        assert!(!r.passed);
        assert!(
            matches!(
                r.reason,
                PadReason::FlatSurface | PadReason::WrongAction | PadReason::StaticMesh
            ),
            "got {:?}",
            r.reason
        );
    }

    #[test]
    fn live3d_par_via_mediapipe_xyz_not_flat() {
        let (times, flats_xy) = yaw_mp_trace(true);
        // Rebuild as xyz with poison Z to prove Z is unused.
        let flats_xyz: Vec<Vec<f32>> = flats_xy
            .iter()
            .map(|xy| {
                // Re-pack through LandmarkFrame then poison-z layout.
                let f = pack_landmark_frame(0, xy, LandmarkBufferLayout::XyInterleaved).unwrap();
                frame_to_mp_xyz_with_poison_z(&f)
            })
            .collect();
        let refs: Vec<&[f32]> = flats_xyz.iter().map(|v| v.as_slice()).collect();
        let mut thr = PadThresholds::default();
        thr.jitter.min_rms = 0.00005;
        thr.min_nonrigid_score = DEFAULT_MIN_NONRIGID_SCORE;
        thr.action.yaw_deg = 18.0;
        let r = evaluate_pad_from_mediapipe_trace(
            consent(),
            ChallengeKind::YawLeft,
            LandmarkBufferLayout::XyzInterleaved,
            &times,
            &refs,
            &[],
            CameraStreamAttestation::default(),
            &thr,
            None,
        )
        .unwrap();
        assert_ne!(r.reason, PadReason::FlatSurface, "poison Z must not fake 3D");
        // Live synthetic may pass or hit soft gates; must not be flat-mask reject.
        assert!(
            r.passed
                || matches!(
                    r.reason,
                    PadReason::WrongAction
                        | PadReason::StaticMesh
                        | PadReason::JitterAnomaly
                        | PadReason::TimeToStartExceeded
                        | PadReason::TimeToCompleteExceeded
                        | PadReason::PoseUnavailable
                ),
            "unexpected {:?}",
            r.reason
        );
    }

    #[test]
    fn no_consent_fails() {
        let (times, flats) = yaw_mp_trace(true);
        let refs: Vec<&[f32]> = flats.iter().map(|v| v.as_slice()).collect();
        let r = evaluate_pad_from_mediapipe_trace(
            BiosenseConsent::denied(BiosensePurpose::Security),
            ChallengeKind::YawLeft,
            LandmarkBufferLayout::XyInterleaved,
            &times,
            &refs,
            &[],
            CameraStreamAttestation::default(),
            &PadThresholds::default(),
            None,
        )
        .unwrap();
        assert!(!r.passed);
        assert_eq!(r.reason, PadReason::NoConsent);
    }

    #[test]
    fn normalized_trace_scales_before_pad() {
        // One frontal frame in normalized coords → pack + denorm; nose near center.
        let mut buf = vec![0.0f32; MEDIAPIPE_FACE_MESH_COUNT * 2];
        let set = |buf: &mut [f32], id: PadLandmarkId, x: f32, y: f32| {
            let i = id.mediapipe_index() as usize;
            buf[i * 2] = x;
            buf[i * 2 + 1] = y;
        };
        set(&mut buf, PadLandmarkId::NoseTip, 0.5, 0.5);
        set(&mut buf, PadLandmarkId::Chin, 0.5, 0.8);
        set(&mut buf, PadLandmarkId::LeftEyeOuter, 0.35, 0.4);
        set(&mut buf, PadLandmarkId::RightEyeOuter, 0.65, 0.4);
        set(&mut buf, PadLandmarkId::LeftCheek, 0.25, 0.55);
        set(&mut buf, PadLandmarkId::RightCheek, 0.75, 0.55);
        let f = pack_landmark_frame(0, &buf, LandmarkBufferLayout::XyInterleaved).unwrap();
        let px = landmarks_from_normalized(f, 200.0, 100.0);
        let n = px.get(PadLandmarkId::NoseTip).unwrap();
        assert!((n.x - 100.0).abs() < 1e-3);
        assert!((n.y - 50.0).abs() < 1e-3);
    }
}
