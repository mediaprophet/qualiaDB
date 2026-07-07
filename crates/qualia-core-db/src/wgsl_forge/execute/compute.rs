use super::super::{schedule::Schedule, ForgeError};
use super::memory::BufferView;

/// A strictly stateless execution bridge designed to execute WGSL Forge IR operations.
/// It operates exclusively on pre-allocated, heap-free `BufferView` slices
/// provisioned by a higher-level `QualiaSlabAllocator`.
pub trait QualiaCompute {
    /// Dispatches a kernel using the supplied `Schedule` and buffer views.
    ///
    /// # Arguments
    /// * `buffers` - Lightweight pointers to the pre-allocated contiguous device memory.
    /// * `schedule` - The generated schedule bounding the workgroups.
    /// * `element_count` - The domain size for dynamic offset tracking.
    ///
    /// # Returns
    /// The hardware execution time in nanoseconds, if supported, otherwise ForgeError.
    fn dispatch(
        &self,
        buffers: &[BufferView],
        schedule: &Schedule,
        element_count: usize,
    ) -> Result<u64, ForgeError>;
}
