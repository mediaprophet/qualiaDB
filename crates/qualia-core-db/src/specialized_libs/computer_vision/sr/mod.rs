//! Super-resolution library (classical B0 + tiling B1 + future learned/GPU tiers).
//!
//! Plan: `docs/plans/native-super-resolution-excellence-2026.md`.

pub mod device_policy;
pub mod super_resolve;
pub mod super_resolve_tiled;
pub mod tile_blend;
pub mod tile_extract;
pub mod tile_plan;

// Learned SR session adapters (ONNX Runtime, feature `vision-onnx`, default OFF).
pub mod cnn_light_session;
pub mod esrgan_session;
pub mod onnx_sr_session;
pub mod swin_session;

pub use device_policy::{super_resolve_tiled_with_policy, super_resolve_with_policy};
pub use super_resolve::{
    super_resolve, ClassicalKernel, EnhancementMode, SrBackend, SrReport, SrRequest,
};
pub use super_resolve_tiled::{super_resolve_tiled, super_resolve_tiled_default};
pub use tile_blend::{blend_tile_into_accum, finalize_blend};
pub use tile_extract::extract_tile_rgb8;
pub use tile_plan::{
    estimate_tile_count, plan_tiles, TilePolicy, TileRect, DEFAULT_OVERLAP, DEFAULT_TILE,
};

pub use cnn_light_session::cnn_light_super_resolve;
pub use esrgan_session::esrgan_super_resolve;
pub use onnx_sr_session::{probe_sr_asset, SrSessionError, SrSessionInfo};
pub use swin_session::swin_super_resolve;
