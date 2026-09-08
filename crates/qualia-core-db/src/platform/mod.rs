//! `platform` category (reorg).

#[cfg(not(target_arch = "wasm32"))]
pub mod compute_bridge;
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod device_benchmark;
pub mod git_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod hardware_passport;
#[cfg(target_os = "android")]
pub mod jni_bridge;
pub mod kml_bridge;
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod npu_ffi;
pub mod tee_ffi;
// Hardware dispatch / I/O — relocated here from `modalities::calculus` (they are
// platform concerns the compute layers depend on, not logic modalities).
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod gpu;
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod hetero_dispatch;
#[cfg(not(target_arch = "wasm32"))]
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod local_scheduler;
#[cfg(not(target_arch = "wasm32"))]
pub mod platform_scheduler;
