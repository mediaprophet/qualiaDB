//! Recipe: consent → MediaPipe/mesh landmark trace → pure-landmark challenge PAD.
//!
//! Orchestration only. No ONNX / MediaPipe runtime. Fail closed without consent.
//! Delegates packing + evaluation to `evaluate_pad_from_mediapipe_trace`, or
//! directly to `evaluate_landmark_pad` when frames are already packed.

use crate::biosense::{
    evaluate_landmark_pad, evaluate_pad_from_mediapipe_trace, BiosenseConsent,
    CameraStreamAttestation, ChallengeKind, LandmarkBufferLayout, LandmarkFrame, PadResult,
    PadThresholds, MAX_MEDIAPIPE_PAD_FRAMES,
};
use crate::biosense::liveness::BlendRow;
use crate::cv::error::CvError;

/// Challenge PAD from a MediaPipe-layout flat landmark trajectory.
///
/// Fail closed: denied consent yields `PadResult { passed: false, reason: NoConsent }`
/// (via the evaluator). Empty/oversized traces return `CvError`.
pub fn challenge_pad_from_mesh_trace(
    consent: BiosenseConsent,
    challenge: ChallengeKind,
    layout: LandmarkBufferLayout,
    frame_times_ms: &[u32],
    flat_landmarks: &[&[f32]],
    blends: &[BlendRow],
    stream: CameraStreamAttestation,
    thr: &PadThresholds,
    image_size: Option<(f32, f32)>,
) -> Result<PadResult, CvError> {
    if !consent.may_process() {
        // Explicit recipe-level fail-closed (evaluator also checks).
        return Ok(PadResult {
            passed: false,
            reason: crate::biosense::PadReason::NoConsent,
            challenge,
        });
    }
    if flat_landmarks.is_empty() || frame_times_ms.is_empty() {
        return Err(CvError::EmptyInput);
    }
    if flat_landmarks.len() > MAX_MEDIAPIPE_PAD_FRAMES {
        return Err(CvError::BufferTooSmall);
    }
    evaluate_pad_from_mediapipe_trace(
        consent,
        challenge,
        layout,
        frame_times_ms,
        flat_landmarks,
        blends,
        stream,
        thr,
        image_size,
    )
}

/// Challenge PAD when landmarks are already packed into [`LandmarkFrame`] slots.
pub fn challenge_pad_from_landmark_frames(
    consent: BiosenseConsent,
    challenge: ChallengeKind,
    frames: &[LandmarkFrame],
    blends: &[BlendRow],
    stream: CameraStreamAttestation,
    thr: &PadThresholds,
) -> Result<PadResult, CvError> {
    if !consent.may_process() {
        return Ok(PadResult {
            passed: false,
            reason: crate::biosense::PadReason::NoConsent,
            challenge,
        });
    }
    evaluate_landmark_pad(consent, challenge, frames, blends, stream, thr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::{
        landmarks_from_normalized, pack_landmark_frame, BiosensePurpose, MEDIAPIPE_FACE_MESH_COUNT,
        PAD_LANDMARK_IDS, PadLandmarkId, PadReason,
    };
    use crate::biosense::liveness::profile_asymmetry_ratio::{
        synthetic_3d_par_frame, synthetic_flat_par_frame,
    };
    use crate::biosense::liveness::landmark_types::Landmark2;

    fn security_consent() -> BiosenseConsent {
        BiosenseConsent::grant_security_template(42)
    }

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

    fn yaw_mp_trace(live3d: bool) -> (Vec<u32>, Vec<Vec<f32>>) {
        let mut times = Vec::new();
        let mut flats = Vec::new();
        for i in 0..12 {
            let t = 40 + i as u32 * 80;
            let yaw = i as f32 * 4.0;
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
    fn no_consent_fails_closed() {
        let (times, flats) = yaw_mp_trace(true);
        let refs: Vec<&[f32]> = flats.iter().map(|v| v.as_slice()).collect();
        let r = challenge_pad_from_mesh_trace(
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
    fn synthetic_flat_par_does_not_pass_rotation_pad() {
        let (times, flats) = yaw_mp_trace(false);
        let refs: Vec<&[f32]> = flats.iter().map(|v| v.as_slice()).collect();
        let mut thr = PadThresholds::default();
        thr.require_jitter = false;
        thr.action.yaw_deg = 15.0;
        let r = challenge_pad_from_mesh_trace(
            security_consent(),
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
    fn packed_landmark_path_respects_consent() {
        let (f, _) = synthetic_3d_par_frame(10.0, 50);
        let frames = [f; 8];
        let blends = [None; 8];
        let r = challenge_pad_from_landmark_frames(
            BiosenseConsent::denied(BiosensePurpose::Security),
            ChallengeKind::YawLeft,
            &frames,
            &blends,
            CameraStreamAttestation::default(),
            &PadThresholds::default(),
        )
        .unwrap();
        assert!(!r.passed);
        assert_eq!(r.reason, PadReason::NoConsent);
    }

    #[test]
    fn empty_trace_errors() {
        let r = challenge_pad_from_mesh_trace(
            security_consent(),
            ChallengeKind::Smile,
            LandmarkBufferLayout::XyInterleaved,
            &[],
            &[],
            &[],
            CameraStreamAttestation::default(),
            &PadThresholds::default(),
            None,
        );
        assert!(matches!(r, Err(CvError::EmptyInput)));
    }

    #[test]
    fn normalized_coords_scale_path_compiles_through_recipe() {
        // Minimal non-empty synthetic: build one frontal pad frame in normalized
        // space via pack + denorm (recipe image_size path). Full PAD still needs
        // a multi-frame trajectory; this only checks the denorm branch accepts input.
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
        let packed = pack_landmark_frame(0, &buf, LandmarkBufferLayout::XyInterleaved).unwrap();
        let px = landmarks_from_normalized(packed, 200.0, 100.0);
        let n = px.get(PadLandmarkId::NoseTip).unwrap();
        assert!((n.x - 100.0).abs() < 1e-3);
        // Recipe with single frame → insufficient frames (not NoConsent).
        let times = [0u32];
        let refs: [&[f32]; 1] = [buf.as_slice()];
        let r = challenge_pad_from_mesh_trace(
            security_consent(),
            ChallengeKind::Smile,
            LandmarkBufferLayout::XyInterleaved,
            &times,
            &refs,
            &[],
            CameraStreamAttestation::default(),
            &PadThresholds::default(),
            Some((200.0, 100.0)),
        )
        .unwrap();
        assert!(!r.passed);
        assert_eq!(r.reason, PadReason::InsufficientFrames);
    }
}
