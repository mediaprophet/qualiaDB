//! Cold scalar Rust owner metadata. Registration does not create a tool/host grant.
use super::super::capability_manifests::{
    AllocationClass, Backend, DeterminismClass, Dimensionality, ExactnessClass, Maturity,
    OpManifest, ResourceLimits,
};
/// Foundation is the intended post-compilation state; this staged source has not
/// itself been compiled or qualified. General UV authoring remains broader work.
pub const UV_ATLAS_MANIFEST: OpManifest = OpManifest {
    op:"uv_atlas_explicit_disk",
    description:"Explicit source-index seams cut corner fans into validated disk charts; canonical harmonic circle mapping, area density and deterministic guttered shelf atlas. Caller-buffered output and caller-selected cold scratch/work limits. Default limits shown; cancellation is between admitted chart solves. No automatic seams, conformality or optimal packing claim.",
    backends:&[Backend::Scalar],
    determinism:DeterminismClass::Tolerance,
    limits:ResourceLimits{max_input_points:65_536,max_output_bytes:4*1024*1024,max_memory_bytes:256*1024*1024,max_time_us:0},
    topology_critical:false,
    maturity:Maturity::Foundation,
    exactness:ExactnessClass::ApproximateMetric,
    allocation:AllocationClass::ColdBounded,
    dimensionality:Dimensionality::D3,
};
