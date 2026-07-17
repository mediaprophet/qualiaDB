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
    evaluate_challenge_pad, issue_challenge, ChallengeKind, MeshFrameSignals, PadReason, PadResult,
    PadThresholds,
};
