//! Vision compute device policy (programme A1).
//!
//! Default builds use **CPU oracles** (`ops/`). Feature `gpu` will attach
//! `shared_gpu` / Forge dispatch without inventing a second adapter.
//!
//! Today: honest `Cpu` / `Unavailable` reporting so SR and CV can log device
//! choice; GPU path lands when `gpu` feature wires Forge (A2).

pub mod dispatch;
pub mod policy;
pub mod forge_resize;

pub use dispatch::{
    avg_pool2d_dispatch, conv2d_nchw_dispatch, max_pool2d_dispatch, resize_nearest_nchw_dispatch,
    VisionComputeDevice, VisionComputeReport,
};
pub use forge_resize::try_resize_nearest_shared_gpu;
pub use policy::{thermal_allows_gpu_tiles, ThermalHint, VisionVramBudget};
