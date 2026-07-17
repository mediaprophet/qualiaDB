//! Qualia Vision — local visual intelligence + classical CV + biosense excellence.
//!
//! Plans: `native-visual-intelligence-and-generative-3d.md`,
//! `native-vision-capability-excellence-2026.md` (VX/VXB/VXP/VX3D).
//!
//! # Design rules
//! - Hot path is caller-buffered, no hidden heap in `infer`.
//! - Model outputs are **epistemic observations**, not ground truth.
//! - Dense pixels / biometric templates never in NQuins.
//! - **No Python**; no OpenCV product link.
//! - Anti-monolith: single-function files under `cv/`, `biosense/`.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod types;
pub mod semantic;
pub mod preprocess;
pub mod media_store;
pub mod ops;
pub mod classifier;
pub mod detector;
pub mod tracker;
pub mod overlay;
pub mod synthetic;
pub mod weights;
pub mod metrics;
pub mod generator;
pub mod spatial;
pub mod capability;
pub mod cv;
pub mod biosense;
pub mod recipes;

#[cfg(feature = "cpu-reference")]
pub mod cpu_reference;

pub use semantic::{
    bbox_quin, class_proposal_quin, compile_observation_quins, compile_observation_quins_full,
    human_correct_quin, human_reject_quin, media_digest, model_digest_quin, observation_quin,
    pack_bbox_u64, q_hash, query_by_frame_range, query_by_model, query_instances_in_region,
    track_quin, unpack_bbox_u64, MediaDigest, VisionQuin, CTX_HUMAN_ATTESTATION, CTX_VISION,
    MAX_OBS_QUINS, P_HAS_BBOX, P_HAS_TRACK, P_HUMAN_CORRECTS, P_HUMAN_REJECTS, P_MODEL_DIGEST,
    P_PROPOSES_CLASS, P_VISUAL_OBSERVATION,
};
pub use types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};
pub use preprocess::{
    iou_u16, letterbox_rgb8, letterbox_workspace_bytes, nms_class_agnostic,
    normalize_rgb8_to_f32_chw, resize_nearest_rgb8,
};
pub use media_store::{MediaRecord, MediaStore, RetentionClass};
pub use ops::{
    avg_pool2d_nchw_f32, conv2d_nchw_f32, max_pool2d_nchw_f32, resize_nearest_nchw_f32,
};
pub use classifier::{fit_two_class_centroids, LinearHead, LinearProbeVision};
pub use detector::{sample_frame_indices, GridMultiObjectDetector, MAX_GRID};
pub use tracker::{BoundedTracker, FLAG_TRACK_OVERFLOW, MAX_TRACKS};
pub use overlay::{
    box_css_percent, box_pixel_bounds, compose_rgb_overlay_rgba8, draw_boxes_rgba8,
    encode_bmp_rgba8,
};
pub use synthetic::{
    generate_scene_rgb8, match_accuracy, sample_id, train_test_disjoint, DatasetSplit,
    SyntheticSampleId, TEST_SEED_BASE, TRAIN_SEED_BASE,
};
pub use weights::{
    ProductionVision, VisionBackendKind, VisionWeightBundle, QVWT_MAGIC, QVWT_VERSION,
};
pub use metrics::{evaluate_real_held_out, evaluate_synthetic, mean_best_iou, MetricsReport};
pub use generator::{
    compile_generation_receipt_quins, CancelFlag, GenerationReceipt, NativeImageGenerator,
    GENERATOR_MODEL_ID, CTX_GENERATION, P_GENERATED_IMAGE, P_GEN_PROMPT, P_GEN_SEED,
};
pub use spatial::{
    assess_twin_eligibility, image_to_heightfield_mesh, mesh_ir_to_obj, mesh_ir_to_stl_binary,
    mesh_ir_triangles, print_readiness, refuse_fea_unless_eligible, validate_mesh_ir,
    AnalysisDomain, ImageTo3dReceipt, MeshIR, MeshValidationReport, MeshValidationStatus,
    PrintReadiness, TwinEligibility, MAX_INDICES, MAX_VERTICES,
};
pub use capability::{all_capabilities, by_id, count_by_status, CapabilityEntry, CapabilityStatus};
pub use cv::{
    bilateral_denoise_u8, box_blur_u8, brief_desc_u8, canny_u8, dilate_u8, draw_rect_u8,
    equalize_hist_u8, erode_u8, fast_corners_u8, find_external_blobs, gaussian_blur_u8,
    hamming_match, histogram_u8, lucas_kanade_step, median_blur_u8, rgb_to_gray_u8, sobel_mag_u8,
    warp_affine_u8, warp_perspective_u8, BlobBox, CvError, GrayView, Keypoint, Match, RgbView,
    DESC_LEN,
};
pub use biosense::{
    blendshape_affect_proposal, cctv_stages_allowed, ensemble_hr, eulerian_color_magnify,
    eulerian_motion_magnify, evaluate_challenge_pad, evaluate_landmark_pad, evaluate_processing_act,
    face_roi_center, frame_blur_score, issue_challenge, issue_rotation_challenge, motion_energy,
    reject_low_quality, respiration_from_motion, roi_mean_rgb, spectral_hr_peak,
    template_hash_from_roi, templates_match, valence_arousal_proposal, AffectProposal,
    BiometricTemplate, BiosenseConsent, BiosensePurpose, BlendshapeProxy, CameraStreamAttestation,
    ChallengeKind, FaceRoi, HeadPose, HrEstimate, Landmark2, LandmarkFrame, MeshBlendProxies,
    MeshFrameSignals, PadLandmarkId, PadReason, PadResult, PadThresholds, PolicyDecision,
    ProcessingAct, QualityReject, TemporalWindow,
};
pub use recipes::self_monitor_pulse;

#[cfg(feature = "cpu-reference")]
pub use cpu_reference::CpuReferenceVision;
