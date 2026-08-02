//! Allocation-free quantized kernels for Qualia's CPU-WASM backend.

mod q8_0;

#[cfg(target_arch = "wasm32")]
pub(crate) use q8_0::q8_0_gemv_into;
