//! Qualia Vision — local visual intelligence.
//!
//! Plan: `docs/plans/native-visual-intelligence-and-generative-3d.md`  
//! Swarm delivery: `docs/plans/native-vision-swarm-delivery.md`  
//! Post-MVP: `docs/plans/native-vision-swarms-GSW.md` (W / G / S)
//!
//! # Design rules
//! - Hot path is caller-buffered, no hidden heap in `infer`.
//! - Model outputs are **epistemic observations**, not ground truth.
//! - Dense pixels never live in NQuins; only hashes, boxes, scores, provenance.
//! - **No Python** in this library.

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
pub use generator::{GenerationReceipt, NativeImageGenerator, GENERATOR_MODEL_ID};
pub use spatial::{
    image_to_heightfield_mesh, validate_mesh_ir, ImageTo3dReceipt, MeshIR, MeshValidationReport,
    MeshValidationStatus, MAX_INDICES, MAX_VERTICES,
};

#[cfg(feature = "cpu-reference")]
pub use cpu_reference::CpuReferenceVision;
