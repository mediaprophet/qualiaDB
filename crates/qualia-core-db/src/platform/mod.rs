//! `platform` category — consolidated from crate-root modules (reorg).

#[cfg(target_os = "android")]
pub mod jni_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod npu_ffi;
pub mod tee_ffi;
pub mod git_bridge;
pub mod kml_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod hardware_passport;
#[cfg(not(target_arch = "wasm32"))]
pub mod device_benchmark;
#[cfg(not(target_arch = "wasm32"))]
pub mod platform_scheduler;
#[cfg(not(target_arch = "wasm32"))]
pub mod local_scheduler;
