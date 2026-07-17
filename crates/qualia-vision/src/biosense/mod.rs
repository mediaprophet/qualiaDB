//! Biosensing excellence: consent, quality, rPPG, magnification, affect, policy.
//! Biometrics/mindware = selfhood; fail closed without consent.

pub mod consent;
pub mod quality;
pub mod rppg;
pub mod magnification;
pub mod face;
pub mod affect;
pub mod biometrics;
pub mod policy;
pub mod respiration;
pub mod liveness;

pub use consent::{BiosenseConsent, BiosensePurpose};
pub use quality::{frame_blur_score, motion_energy, reject_low_quality, QualityReject};
pub use rppg::{ensemble_hr, spectral_hr_peak, HrEstimate};
pub use magnification::{eulerian_color_magnify, eulerian_motion_magnify};
pub use face::{face_roi_center, roi_mean_rgb, FaceRoi};
pub use affect::{
    blendshape_affect_proposal, valence_arousal_proposal, AffectProposal, BlendshapeProxy,
};
pub use biometrics::{template_hash_from_roi, templates_match, BiometricTemplate};
pub use policy::{
    cctv_stages_allowed, evaluate_processing_act, PolicyDecision, ProcessingAct,
};
pub use respiration::respiration_from_motion;
pub use liveness::{
    check_camera_stream_integrity, check_temporal_window, estimate_head_pose,
    evaluate_challenge_pad, evaluate_landmark_jitter, evaluate_landmark_pad, evaluate_non_rigid_z,
    issue_challenge, issue_rotation_challenge, ActionThresholds, CameraStreamAttestation,
    CameraStreamSource, ChallengeKind, HeadPose, Landmark2, LandmarkFrame, MeshBlendProxies,
    MeshFrameSignals, NonRigidVerdict, PadLandmarkId, PadReason, PadResult, PadThresholds,
    StreamIntegrityVerdict, TemporalGate, TemporalWindow, DEFAULT_TTC_MS, DEFAULT_TTS_MS,
    DEFAULT_YAW_THRESHOLD_DEG,
};
