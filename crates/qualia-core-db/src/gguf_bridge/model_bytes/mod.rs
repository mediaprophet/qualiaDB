//! Internal access to legacy model backing, preserving its exact ownership.
//!
//! This partition adds no immutable lease, model admission, or new load route.
//! Public legacy fields remain available to their existing callers.
mod handle;
pub(crate) use handle::ModelByteHandle;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use super::QTensorEngine;

impl QTensorEngine {
    /// Borrow the bytes currently selected by the existing legacy field.
    pub(crate) fn model_bytes(&self) -> Option<&[u8]> {
        self.gguf_mmap.as_ref().map(|bytes| &bytes[..])
    }

    /// Retain the same object while a caller mutates other engine state.
    pub(crate) fn clone_model_bytes(&self) -> Option<ModelByteHandle> {
        self.gguf_mmap.clone().map(ModelByteHandle::from_legacy)
    }
}
