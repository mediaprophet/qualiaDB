//! Caller-buffered shoreline attributes for native open water meshes.
mod shoreline;
mod spatial;
pub use shoreline::{shore_distance_into,shore_topology_into,ShoreTopology,ShoreError,ShoreReceipt};
pub use spatial::{shore_distance_accelerated_into,ShoreSpatialScratch,spatial_working_bytes};
pub const ALLOCATION_CLASS:crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass=
    crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass::HotZeroHeap;
/// The optional spatial builder delegates to the native bounded cold BVH
/// lifecycle; nearest-edge traversal itself stays allocation-free.
pub const SPATIAL_BUILD_ALLOCATION_CLASS:crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass=
    crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass::ColdBounded;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod spatial_tests;
