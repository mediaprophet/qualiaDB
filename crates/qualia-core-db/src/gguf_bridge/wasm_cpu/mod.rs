//! Qualia's CPU-WASM LLM backend.
//!
//! This is a first-party execution floor, not a llama.cpp/wllama binding. It
//! reuses Qualia's GGUF/P64 index, tokenizer, quantized GEMV and transformer
//! mathematics while keeping WebGPU an optional accelerator.

mod forward;
#[cfg(any(test, target_arch = "wasm32"))]
mod kernels;
mod model;

#[cfg(target_arch = "wasm32")]
pub(crate) use kernels::q8_0_gemv_into;
pub use model::{CpuWasmEngine, CpuWasmError, CpuWasmStep};

/// Mobile-first default for the independent LLM working set. This is not part
/// of the 42 MiB semantic/SLG Sentinel arena; model inference owns a separate,
/// explicitly sized memory domain.
pub const CPU_WASM_DEFAULT_CONTEXT: usize = 512;
/// Explicit safety cap for one contiguous CPU-WASM KV allocation. This belongs
/// to the LLM memory policy and is unrelated to the semantic Sentinel arena.
pub const CPU_WASM_MAX_CONTEXT: usize = 4096;
