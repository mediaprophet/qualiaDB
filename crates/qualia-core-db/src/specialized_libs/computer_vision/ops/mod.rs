//! CPU vision ops — certification oracles for future Forge GPU ports (V3).
//!
//! Pure Rust, caller-buffered. No Python. GPU WGSL ports land as V3b in
//! `qualia-core-db::wgsl_forge` without changing these numerical contracts.

mod conv2d;
mod pool2d;
mod resize2d;

pub use conv2d::conv2d_nchw_f32;
pub use pool2d::{avg_pool2d_nchw_f32, max_pool2d_nchw_f32};
pub use resize2d::resize_nearest_nchw_f32;
