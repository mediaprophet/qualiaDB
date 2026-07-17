//! Thin recipe: MediaPipe flat landmark trace → pure-landmark PAD.
//!
//! Packs each frame with [`pack_landmark_frame`] (xy only; **never model Z**),
//! optionally denormalizes, then delegates to [`evaluate_landmark_pad`].
//! Does not load ONNX / MediaPipe runtimes — buffer → geometry only.

use super::landmarks_from_normalized::landmarks_from_normalized;
use super::pack_landmark_frame::{pack_landmark_frame, LandmarkBufferLayout};
use crate::biosense::consent::BiosenseConsent;
use crate::biosense::liveness::camera_stream_integrity::CameraStreamAttestation;
use crate::biosense::liveness::challenge_kind::ChallengeKind;
use crate::biosense::liveness::challenge_pad::{
    evaluate_landmark_pad, BlendRow, PadResult, PadThresholds,
};
use crate::biosense::liveness::landmark_types::LandmarkFrame;
use crate::cv::error::CvError;

/// Max frames packed on the stack before calling the PAD evaluator.
pub const MAX_MEDIAPIPE_PAD_FRAMES: usize = 64;

/// Evaluate pure-landmark PAD from a MediaPipe-layout landmark trajectory.
///
/// * `flat_landmarks[i]` — one flat xy or xyz buffer of length ≥ `layout.expected_len()`.
/// * `frame_times_ms[i]` — challenge-relative ms for frame \(i\) (same length as buffers).
/// * `image_size` — `Some((width, height))` when buffers are normalized \(0..1\);
///   `None` when already in pixel (or any consistent absolute) units.
/// * `blends` — optional blendshape rows aligned with frames (pass empty or all-`None`).
///
/// Z components in xyz layout are ignored at pack time.
pub fn evaluate_pad_from_mediapipe_trace(
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
    if frame_times_ms.len() != flat_landmarks.len() {
        return Err(CvError::DimensionMismatch);
    }
    if flat_landmarks.is_empty() {
        return Err(CvError::EmptyInput);
    }
    if flat_landmarks.len() > MAX_MEDIAPIPE_PAD_FRAMES {
        return Err(CvError::BufferTooSmall);
    }

    let mut packed: [LandmarkFrame; MAX_MEDIAPIPE_PAD_FRAMES] =
        [LandmarkFrame::empty(0); MAX_MEDIAPIPE_PAD_FRAMES];
    let n = flat_landmarks.len();

    for i in 0..n {
        let mut frame = pack_landmark_frame(frame_times_ms[i], flat_landmarks[i], layout)?;
        if let Some((w, h)) = image_size {
            frame = landmarks_from_normalized(frame, w, h);
        }
        packed[i] = frame;
    }

    // Pad blend rows with None when shorter than the trace.
    let mut blend_buf: [BlendRow; MAX_MEDIAPIPE_PAD_FRAMES] = [None; MAX_MEDIAPIPE_PAD_FRAMES];
    for i in 0..n {
        blend_buf[i] = blends.get(i).copied().unwrap_or(None);
    }

    evaluate_landmark_pad(
        consent,
        challenge,
        &packed[..n],
        &blend_buf[..n],
        stream,
        thr,
    )
}
