//! Biosensing excellence: consent, quality, rPPG, magnification, affect, policy.
//! Biometrics/mindware = selfhood; fail closed without consent.

pub mod consent;
pub mod quality;
pub mod rppg;
pub mod magnification;
pub mod face;
pub mod face_mesh;
pub mod affect;
pub mod biometrics;
pub mod policy;
pub mod respiration;
pub mod liveness;
pub mod pose;

pub use consent::{BiosenseConsent, BiosensePurpose};
pub use quality::{frame_blur_score, motion_energy, reject_low_quality, QualityReject};
pub use rppg::{
    ensemble_hr, pos_rppg_trace, respiration_from_rppg_harmonic, spectral_hr_peak, HrEstimate,
};
pub use magnification::{
    band_energy_snr, colour_evm_yiq, design_bandpass_iir, energy_ms, eulerian_color_magnify,
    eulerian_color_magnify_consented, eulerian_color_magnify_ex, eulerian_color_magnify_hz,
    eulerian_motion_magnify, eulerian_motion_magnify_consented, eulerian_motion_magnify_ex,
    eulerian_motion_magnify_hz, evm_snr_gate, evm_snr_gate_trace, gaussian_pyramid_build,
    gaussian_pyramid_down_u8, laplacian_pyramid_build, pyramid_reconstruct, temporal_bandpass_iir,
    temporal_bandpass_series, BandpassIir, BandpassState, ColourEvmParams, EvmRefuse, EvmSnrVerdict,
    MotionEvmParams, PyramidLevelMeta, DEFAULT_EVM_MIN_SNR, MAX_PYRAMID_LEVELS,
};
pub use face::{face_nms, face_roi_center, roi_mean_rgb, yunet_decode_detections, FaceBox, FaceRoi};
pub use face_mesh::{
    evaluate_pad_from_mediapipe_trace, landmarks_from_normalized, mediapipe_index_for_pad,
    pack_landmark_frame, pad_id_for_mediapipe_index, LandmarkBufferLayout, MAX_MEDIAPIPE_PAD_FRAMES,
    MEDIAPIPE_FACE_MESH_COUNT, PAD_LANDMARK_IDS, PAD_MEDIAPIPE_INDICES,
};
pub use affect::{
    blendshape_affect_proposal, valence_arousal_proposal, AffectProposal, BlendshapeProxy,
};
pub use biometrics::{
    sface_cosine, sface_embed_from_tensor, template_hash_from_roi, templates_match,
    BiometricTemplate, SFACE_EMBED_DIM,
};
pub use pose::{
    pack_hand_landmarks_xy, pack_pose_landmarks_xy, HandLandmark, PoseLandmark, MAX_HAND_LANDMARKS,
    MAX_POSE_LANDMARKS,
};
pub use policy::{
    cctv_stages_allowed, evaluate_processing_act, PolicyDecision, ProcessingAct,
};
pub use respiration::{
    ensemble_respiration, respiration_from_motion, respiration_rate_from_motion_trace, RrEstimate,
    RR_F_HI_HZ, RR_F_LO_HZ, RR_MIN_SNR_DEFAULT,
};
pub use liveness::{
    check_camera_stream_integrity, check_temporal_window, estimate_head_pose,
    evaluate_challenge_pad, evaluate_landmark_jitter, evaluate_landmark_pad, evaluate_non_rigid_z,
    evaluate_profile_asymmetry, issue_challenge, issue_rotation_challenge, profile_asymmetry_ratio,
    ActionThresholds, CameraStreamAttestation, CameraStreamSource, ChallengeKind, HeadPose,
    Landmark2, LandmarkFrame, MeshBlendProxies, MeshFrameSignals, NonRigidVerdict, PadLandmarkId,
    PadReason, PadResult, PadThresholds, ParSample, ParVerdict, StreamIntegrityVerdict,
    TemporalGate, TemporalWindow, DEFAULT_PAR_TAU, DEFAULT_TTC_MS, DEFAULT_TTS_MS,
    DEFAULT_YAW_THRESHOLD_DEG, MIN_YAW_SPAN_DEG,
};
