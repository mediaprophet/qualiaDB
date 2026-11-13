//! Compatibility limits for bounded whole-file admission.
//! Shared structural validation lives in bundle::stream::index.

/// Caller-owned cold admission limits. `decoded_index_bytes` is a conservative
/// logical reservation (four times retained storage plus scratch), not measured RSS or
/// an allocator/stack guarantee. Payload bytes continue to borrow the input.
#[derive(Clone, Copy, Debug)]
pub struct BundleReadLimits {
    pub file_bytes: usize,
    pub index_bytes: usize,
    pub entries: usize,
    pub key_bytes: usize,
    pub kind_bytes: usize,
    pub meta_bytes: usize,
    pub decoded_index_bytes: usize,
}
impl Default for BundleReadLimits {
    fn default() -> Self {
        Self {
            file_bytes: 128 * 1024 * 1024,
            index_bytes: 1024 * 1024,
            entries: 4096,
            key_bytes: 4096,
            kind_bytes: 64,
            meta_bytes: 64 * 1024,
            decoded_index_bytes: 8 * 1024 * 1024,
        }
    }
}
