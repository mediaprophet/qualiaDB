use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
type LegacyBytes = Arc<memmap2::Mmap>;
#[cfg(target_arch = "wasm32")]
type LegacyBytes = Arc<[u8]>;

/// Internal retained owner; only borrowed byte slices leave this handle. Lease
/// exclusion cannot be separated from its bytes by cloning a bare mmap.
#[derive(Clone)]
pub(crate) enum ModelByteHandle {
    Legacy(LegacyBytes),
    #[cfg(not(target_arch = "wasm32"))]
    Leased(Arc<etalon_model_io::lease::ModelLease>),
    #[cfg(not(target_arch = "wasm32"))]
    Snapshot(Arc<memmap2::Mmap>),
}

impl ModelByteHandle {
    pub(super) fn from_legacy(bytes: LegacyBytes) -> Self {
        Self::Legacy(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn leased(bytes: Arc<etalon_model_io::lease::ModelLease>) -> Self {
        Self::Leased(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn snapshot(bytes: Arc<memmap2::Mmap>) -> Self {
        Self::Snapshot(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Legacy(a), Self::Legacy(b)) | (Self::Snapshot(a), Self::Snapshot(b)) => {
                Arc::ptr_eq(a, b)
            }
            (Self::Leased(a), Self::Leased(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl std::ops::Deref for ModelByteHandle {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            Self::Legacy(bytes) => &bytes[..],
            #[cfg(not(target_arch = "wasm32"))]
            Self::Leased(bytes) => bytes.bytes(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Snapshot(bytes) => &bytes[..],
        }
    }
}

impl AsRef<[u8]> for ModelByteHandle {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
