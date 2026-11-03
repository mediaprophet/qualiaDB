use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
type LegacyBytes = Arc<memmap2::Mmap>;
#[cfg(target_arch = "wasm32")]
type LegacyBytes = Arc<[u8]>;

/// Internal retained owner; only borrowed byte slices leave this handle.
#[derive(Clone)]
pub(crate) struct ModelByteHandle(LegacyBytes);

impl ModelByteHandle {
    pub(super) fn from_legacy(bytes: LegacyBytes) -> Self {
        Self(bytes)
    }
}

impl std::ops::Deref for ModelByteHandle {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsRef<[u8]> for ModelByteHandle {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
