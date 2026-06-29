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
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod inference_gpu_profiler;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub use inference_gpu_profiler as llm_gpu_profiler; // transitional alias
pub mod inference_kernel_parity;
pub use inference_kernel_parity as llm_kernel_parity; // transitional alias
pub mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub mod ambient_orchestration;
pub mod compute_universe;
#[cfg(target_os = "windows")]
pub mod directml_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod ggml_quants;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod gguf_sharder;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod metal_bridge;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod neuro_symbolic_sieve;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod orchestrator;
#[cfg(not(target_arch = "wasm32"))]
pub mod residency_planner;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod resident_model;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod safetensor;
pub mod semantic_culler;
pub mod spatial_sieve;
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
// OMP sparse KV-cache decomposition builds on `crate::solvers` (dense linear
// algebra), which is itself native-or-`wasm-scientific`; mirror that gate.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod sparse_cache;
// The thermal-eviction WAL is a file-backed `memmap2` mmap — fundamentally
// native (no mmap'd files on wasm32).
#[cfg(not(target_arch = "wasm32"))]
pub mod thermal_wal;
