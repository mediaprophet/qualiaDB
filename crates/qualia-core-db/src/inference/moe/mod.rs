//! Mixture-of-Experts (MoE) execution and offload engine (Work Packages F9, F10).
//!
//! Provides zero-heap top-k routing, NVFP4 unpacking and GEMV, and heterogeneous
//! GPU/CPU expert offload for the Qwen3.6-35B-A3B architecture on RTX A2000 12GB.

pub mod expert_cache;
pub mod nvfp4;
pub mod routing;

pub use expert_cache::{
    ExpertCacheTelemetry, MoeOffloadManager, SlotAccessOutcome, DEFAULT_GPU_EXPERT_SLOTS,
};
pub use nvfp4::{
    dequantize_nvfp4_block, dequantize_nvfp4_row, fp8_e4m3_to_f32, nvfp4_gemv_zero_heap,
    Nvfp4Error, E2M1_TABLE, NVFP4_BLOCK_SIZE, NVFP4_BYTES_PER_BLOCK,
};
pub use routing::{
    combine_expert_outputs, route_topk, MoeError, MoeRoutingConfig, MAX_MOE_TOPK,
};
