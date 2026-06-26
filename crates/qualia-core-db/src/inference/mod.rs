//! `inference` category (reorg).

// Inference-runtime components. These run model inference (a tensor program) — the underlying
// mathematics now lives in `crate::solvers` (GEMM, activations, softmax, normalization, attention,
// RoPE, FFN). The old `llm_*` names are kept as transitional aliases; "inference" is what these are.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod inference_agent;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub use inference_agent as llm_agent; // transitional alias
pub mod inference_awq;
pub use inference_awq as llm_awq; // transitional alias
#[cfg(not(target_arch = "wasm32"))]
pub mod inference_bench;
#[cfg(not(target_arch = "wasm32"))]
pub use inference_bench as llm_bench; // transitional alias
pub mod inference_eval;
pub use inference_eval as llm_eval; // transitional alias
pub mod inference_gpu_profiler;
pub use inference_gpu_profiler as llm_gpu_profiler; // transitional alias
pub mod inference_kernel_parity;
pub use inference_kernel_parity as llm_kernel_parity; // transitional alias
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
