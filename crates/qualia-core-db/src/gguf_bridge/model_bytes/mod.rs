//! Internal access to legacy and explicitly prepared model backing.
//! Native prepared engines retain immutable lease/snapshot owners separately;
//! legacy public fields and their adoption semantics remain available.
mod handle;
#[cfg(not(target_arch = "wasm32"))]
mod prepared;
pub(crate) use handle::ModelByteHandle;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use prepared::NativeModelSources;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use super::QTensorEngine;

impl QTensorEngine {
    /// Borrow the bytes currently selected by the existing legacy field.
    pub(crate) fn model_bytes(&self) -> Option<&[u8]> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(bytes) = &self.prepared_native_bytes {
            return Some(bytes);
        }
        self.gguf_mmap.as_ref().map(|bytes| &bytes[..])
    }

    /// Retain the same object while a caller mutates other engine state.
    pub(crate) fn clone_model_bytes(&self) -> Option<ModelByteHandle> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(bytes) = &self.prepared_native_bytes {
            return Some(bytes.clone());
        }
        self.gguf_mmap.clone().map(ModelByteHandle::from_legacy)
    }
}
