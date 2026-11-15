//! Current native QAC production facade and explicit resource-registry linkage.
//! Actual PCM/current typed documents determine full-byte artifact identity.
//! Tensor previews are display data and are never synthesized into original PCM.
use crate::tensor::bake_pipeline::{audio_sidecar_relpath, PRED_HAS_SPECTRAL_SHEET};
use crate::NQuin;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::audio::qac::production::{
    produce_pcm, publish_current_signal, Error as SidecarPublishError, ProductionOptions,
};

/// Link a caller-assigned resource-registry index. It is deliberately separate
/// from the full32-byte content digest and is never derived from a preview/hash.
#[inline]
pub fn compile_spectral_sheet_quin(subject_hash: u64, sheet_index: u32) -> NQuin {
    let mut q = NQuin::default();
    q.subject = subject_hash;
    q.predicate = PRED_HAS_SPECTRAL_SHEET;
    q.object = u64::from(sheet_index);
    q
}
/// Format the entire final-byte digest into a canonical .qac relative path.
/// A short buffer returns0 without changing caller bytes.
#[inline]
pub fn format_sidecar_relpath(digest: &[u8; 32], out: &mut [u8]) -> usize {
    audio_sidecar_relpath(digest, out)
}

#[cfg(test)]
#[path = "audio_sidecar_link_tests.rs"]
mod tests;
