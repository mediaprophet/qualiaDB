//! Canonical packed instance ABI and deterministic visibility delegation.
pub use etalon_render::instance_culling::*;
#[cfg(feature = "gpu-runtime")]
pub use crate::render::gpu::indirect_cull::{
    build_hzb_mip_chain_zero_heap, cull_and_select_lod_indirect_zero_heap,
    select_lod_tier_with_hysteresis, CullUniformsGpu, DrawIndexedIndirectCommand, HzbPyramidView,
    IndirectCullPipelineGpu, IndirectCullStreamCounts, InstanceCullDescriptor,
    RepresentationPolicy, DEFAULT_LOD_HYSTERESIS, DEFAULT_LOD_THRESHOLDS_PX,
    DEFAULT_MIN_CULL_RADIUS_PX,
};
