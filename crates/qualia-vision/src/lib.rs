//! Qualia Vision — local visual intelligence.
//!
//! Plan: `docs/plans/native-visual-intelligence-and-generative-3d.md`  
//! Swarm delivery: `docs/plans/native-vision-swarm-delivery.md`
//!
//! # Design rules
//! - Hot path is caller-buffered, no hidden heap in `infer`.
//! - Model outputs are **epistemic observations**, not ground truth.
//! - Dense pixels never live in NQuins; only hashes, boxes, scores, provenance.
//! - **No Python** in this library.
//!
//! Phase 1: ABI + CPU reference detector.  
//! V1: preprocess (resize/NMS/letterbox).  
//! V2: content-addressed media store.  
//! V3: CPU vision ops (Conv/Pool/Resize) as Forge GPU oracles.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod types;
pub mod semantic;
pub mod preprocess;
pub mod media_store;
pub mod ops;
pub mod classifier;

#[cfg(feature = "cpu-reference")]
pub mod cpu_reference;

pub use semantic::{
    compile_observation_quins, media_digest, observation_quin, q_hash, MediaDigest, VisionQuin,
    MAX_OBS_QUINS, P_PROPOSES_CLASS, P_VISUAL_OBSERVATION, CTX_VISION,
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

#[cfg(feature = "cpu-reference")]
pub use cpu_reference::CpuReferenceVision;
