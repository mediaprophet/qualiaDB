//! Pure-landmark presentation attack detection (challenge-response).
//!
//! Geometric heuristics + temporal windows + **PAR (2D x-only)** flat-mask lock.
//! Never trust model-inferred Z. No RGB texture PAD. Pair with stream attestation.

pub mod action_threshold;
pub mod camera_stream_integrity;
pub mod challenge_kind;
pub mod challenge_pad;
pub mod landmark_jitter;
pub mod landmark_types;
pub mod non_rigid_z;
pub mod profile_asymmetry_ratio;
pub mod rigid_head_pose;
pub mod temporal_window;

pub use action_threshold::{
    action_met_at_frame, mouth_open_ratio, ActionThresholds, DEFAULT_YAW_THRESHOLD_DEG,
};
pub use camera_stream_integrity::{
    check_camera_stream_integrity, CameraStreamAttestation, CameraStreamSource,
    StreamIntegrityVerdict,
};
pub use challenge_kind::ChallengeKind;
pub use challenge_pad::{
    evaluate_challenge_pad, evaluate_landmark_pad, issue_challenge, issue_rotation_challenge,
    BlendRow, MeshFrameSignals, PadReason, PadResult, PadThresholds,
};
pub use landmark_jitter::{evaluate_landmark_jitter, mean_landmark_motion, JitterThresholds};
pub use landmark_types::{Landmark2, LandmarkFrame, MeshBlendProxies, PadLandmarkId};
pub use non_rigid_z::{evaluate_non_rigid_z, NonRigidVerdict};
pub use profile_asymmetry_ratio::{
    evaluate_profile_asymmetry, normalized_par_delta, profile_asymmetry_ratio, ParSample,
    ParVerdict, DEFAULT_PAR_TAU, MIN_YAW_SPAN_DEG,
};
pub use rigid_head_pose::{estimate_head_pose, pose_delta, HeadPose};
pub use temporal_window::{
    check_temporal_window, TemporalGate, TemporalWindow, DEFAULT_TTC_MS, DEFAULT_TTS_MS,
};
