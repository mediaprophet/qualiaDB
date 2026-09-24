//! Mixture-of-Experts (MoE) execution and offload engine (Work Packages F9, F10).
//!
//! Provides zero-heap top-k routing, NVFP4 unpacking and GEMV, and heterogeneous
//! GPU/CPU expert offload for the Qwen3.6-35B-A3B architecture on RTX A2000 12GB.

pub mod dispatch;
pub mod expert_cache;
pub mod ftw_loader;
pub mod nvfp4;
pub mod placement;
pub mod routing;

pub use dispatch::{
    dispatch_moe_step, evaluate_swiglu_expert_nvfp4, silu, ExpertWeightView, MAX_INTERMEDIATE_DIM,
    MAX_ROUTED_EXPERTS,
};
pub use expert_cache::{
    compute_principled_slot_capacity, query_dynamic_accelerator_budget,
    query_dynamic_hardware_tier, DynamicVramStatus, ExpertCacheTelemetry,
    ExpertResidencyProfile, MoeOffloadManager, PersonalHardwareTier, SlotAccessOutcome,
    DEFAULT_GPU_EXPERT_SLOTS,
};
pub use ftw_loader::{
    FtwExpertBankSlice, FtwExpertData, FtwManifest, FtwModelPackage, FtwShardEntry, FtwTensorEntry,
};
pub use nvfp4::{
    dequantize_nvfp4_block, dequantize_nvfp4_row, fp8_e4m3_to_f32, nvfp4_gemv_zero_heap,
    Nvfp4Error, E2M1_TABLE, NVFP4_BLOCK_SIZE, NVFP4_BYTES_PER_BLOCK,
};
pub use placement::{ExpertPlacementCalibration, ExpertPlacementPolicy};
pub use routing::{combine_expert_outputs, route_topk, MoeError, MoeRoutingConfig, MAX_MOE_TOPK};
