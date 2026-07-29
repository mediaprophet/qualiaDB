//! Music structure analysis (AU-STRUCT) — self-similarity matrix, Foote novelty
//! curve, and novelty-peak segmentation. This replaces the energy-hysteresis
//! "structure" heuristic in `music.rs` with a real SSM + novelty approach.
//!
//! The caller supplies a per-frame feature matrix (e.g. from
//! [`crate::features::mel::mfcc`] or chroma / tonal features); these modules
//! never recompute features. The frames²-sized SSM is written into a
//! caller-provided buffer — see each function's caller-buffer contract.
//!
//! Re-exports only; each leaf module owns exactly one public function.

pub mod novelty;
pub mod segmentation;
pub mod ssm;

pub use novelty::novelty_curve;
pub use segmentation::segment_boundaries;
pub use ssm::self_similarity;
