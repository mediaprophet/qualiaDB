//! Resident index budgets are separate from remote logical extent.
use crate::bundle::BundleReadLimits;

#[derive(Clone, Copy, Debug)]
pub struct IndexReadLimits {
    pub index_bytes: usize,
    pub entries: usize,
    pub key_bytes: usize,
    pub kind_bytes: usize,
    pub meta_bytes: usize,
    /// Conservative aggregate reservation for retained rows and live scratch.
    /// This bounds requested storage, not allocator overhead or measured RSS.
    pub decoded_index_bytes: usize,
}
impl From<BundleReadLimits> for IndexReadLimits {
    fn from(l: BundleReadLimits) -> Self {
        Self {
            index_bytes: l.index_bytes,
            entries: l.entries,
            key_bytes: l.key_bytes,
            kind_bytes: l.kind_bytes,
            meta_bytes: l.meta_bytes,
            decoded_index_bytes: l.decoded_index_bytes,
        }
    }
}
impl Default for IndexReadLimits {
    fn default() -> Self {
        BundleReadLimits::default().into()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExtentLimit {
    pub max_logical_bytes: u64,
}
impl Default for ExtentLimit {
    fn default() -> Self {
        Self {
            max_logical_bytes: 1 << 40,
        }
    }
}
