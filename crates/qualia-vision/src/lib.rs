//! Qualia Vision — local visual intelligence (Phase 1).
//!
//! Plan: `docs/plans/native-visual-intelligence-and-generative-3d.md`.
//!
//! # Design rules (human-rights / consumer edge)
//! - Hot path is caller-buffered, no hidden heap in `infer`.
//! - Model outputs are **epistemic observations**, not ground truth.
//! - Dense pixels never live in NQuins; only hashes, boxes, scores, provenance.
//!
//! Phase 1 delivers the stable ABI + a deterministic **CPU reference** detector
//! (colour-channel + edge energy classes) so desktop/CLI can integrate without
//! waiting for a full P64 vision backbone. Replace the reference backend with a
//! real encoder when P64 vision weights land — same `VisualModel` trait.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod types;
pub mod semantic;
#[cfg(feature = "cpu-reference")]
pub mod cpu_reference;

pub use semantic::{
    compile_observation_quins, observation_quin, MediaDigest, MAX_OBS_QUINS, P_PROPOSES_CLASS,
    P_VISUAL_OBSERVATION, CTX_VISION,
};
pub use types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};

#[cfg(feature = "cpu-reference")]
pub use cpu_reference::CpuReferenceVision;
