//! `inference` category — consolidated from crate-root modules (reorg).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod llm_agent;
pub mod llm_awq;
#[cfg(not(target_arch = "wasm32"))]
pub mod llm_bench;
pub mod llm_eval;
pub mod llm_gpu_profiler;
pub mod llm_kernel_parity;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod gguf_sharder;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod ggml_quants;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod safetensor;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod tensor_roles;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod ternary;
#[cfg(not(target_arch = "wasm32"))]
pub mod ternary_gpu;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod topk;
#[cfg(not(target_arch = "wasm32"))]
pub mod topk_gpu;
#[cfg(target_os = "windows")]
pub mod directml_bridge;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod metal_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod resident_model;
#[cfg(not(target_arch = "wasm32"))]
pub mod residency_planner;
pub mod semantic_culler;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod neuro_symbolic_sieve;
pub mod spatial_sieve;
pub mod compute_universe;
pub mod agent;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod orchestrator;
#[cfg(not(target_arch = "wasm32"))]
pub mod ambient_orchestration;
