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
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm"))]
pub mod inference_bench_wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod kv_capture;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod kv_dict;
#[cfg(not(target_arch = "wasm32"))]
pub mod kv_dict_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub use inference_bench as llm_bench; // transitional alias
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm"))]
pub use inference_bench_wasm as llm_bench; // transitional alias
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
/// Stage-by-stage library probe tests for the inference optim toolkit.
#[cfg(test)]
pub mod toolkit_probe;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod topk;
// W2: exact CPU sampling chain (pure, wasm-safe — no GPU, no `rand`, no file I/O).
pub mod sampler;
// Multi-mode inference (portable / cuda-tc / quant-graph) — coexisting approaches.
pub mod inference_modes;
pub use inference_modes::{
    active_inference_mode, apply_mode_toggles, bootstrap_inference_mode, fast_verify_html_default,
    post_turn_verify_enabled, prefer_tensor_core_gemm, quant_graph_grounding_enabled,
    rights_mode_enabled, sentinel_mid_decode_enabled, set_inference_mode, InferenceMode,
};
// Application profiles: interactive / live-fast / batch-overnight (no Ollama).
pub mod application_profile;
pub use application_profile::{
    active_application_profile, apply_application_profile, bootstrap_application_profile,
    set_application_profile, ApplicationProfile,
};
// Inference superiority lab (plan: inference-superiority-lab-and-toolset-plan.md).
#[cfg(not(target_arch = "wasm32"))]
pub mod lab;
// Prepared native inference plan/run boundary, execution receipts, and bounded run artifacts.
#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;
// Post-turn verify / self-heal (FastVerify path).
pub mod post_turn_verify;
pub use post_turn_verify::{
    maybe_verify_turn, return_html_as_text, verify_and_heal_turn, VerifiedTurn, VerifyCheck,
};
// Device-optimal path: passport benchmark → pick dx12/vulkan/metal/cuda lane + quant.
#[cfg(not(target_arch = "wasm32"))]
pub mod inference_path_selector;
#[cfg(not(target_arch = "wasm32"))]
pub use inference_path_selector::{
    apply_inference_path_plan, bootstrap_optimal_inference_path, format_path_plan,
    last_inference_path_plan, path_auto_enabled, resolve_inference_path_plan, run_path_select_cli,
    ComputeLane, InferencePathPlan, QuantProfile,
};
// QuantGraph: selective fact grounding / repair after LLM proposal.
pub mod quant_graph_grounding;
pub use quant_graph_grounding::{
    export_fact_quins, fact_count, ground_generation, load_facts_from_tsv, lookup_capital_object,
    maybe_ground_generation, register_capital_fact, register_fact, reset_fact_store_to_defaults,
    seed_facts_from_bundled, GroundingFact, GroundingResult, CTX_GROUNDING, P_CAPITAL_OF,
};
// Qualia-unique hybrid: graph route mask + fact draft + 10D query + deontic gate.
pub mod qualia_hybrid;
pub use qualia_hybrid::{
    apply_graph_logit_bias, force_fact_tokens, graph_force_enabled, prepare_hybrid_decode,
    propose_best_draft, propose_fact_draft, publish_graph_route_from_prompt,
    publish_grounding_obligation, publish_prompt_query_tensor, GRAPH_LOGIT_BIAS,
};
// CUDA dense batch GEMM lane (mode=cuda); stub when feature off.
#[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
pub mod cuda_lane;
#[cfg(any(target_arch = "wasm32", not(feature = "cuda")))]
pub mod cuda_lane_stub;
#[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
pub use cuda_lane::{
    cache_dense_weight, clear_weight_cache, dense_weight_cached, device_kv_ready,
    ensure_device_kv_cache, preload_resident_blob, prepare_mega_pass_kernels,
    q4k_device_weight_count, q4k_weight_resident, q8_0_gemv_oracle_into, try_cuda_batch_gemv,
    try_cuda_batch_gemv_cached, try_cuda_batch_gemv_cached_only, try_cuda_mega_pass,
    try_q4k_soa_attention_device, try_q4k_soa_ffn_block, try_q4k_soa_ffn_block_residual,
    try_q4k_soa_fused_swiglu, try_q4k_soa_gemv, try_q4k_soa_qkv, try_q8_0_cuda_gemv,
    warm_cuda_context, weight_cache_len, weight_fingerprint, MegaPassLayerDims,
    MegaPassLayerWeights, MegaPassPlanView, MegaPassWeightLayout, MAX_DENSE_ELEMS,
    Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS,
};
#[cfg(any(target_arch = "wasm32", not(feature = "cuda")))]
pub use cuda_lane_stub as cuda_lane;
// W6a: prompt-lookup speculative decoding proposer (pure, wasm-safe).
pub mod prompt_lookup;
// Metal mega-pass orchestrator (Apple Silicon). Stub on non-macOS.
pub mod metal_lane;
// Paged KV cache: block-paged KV storage (vLLM-style).
pub mod paged_kv;
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

// W7: real GPU thermal/power telemetry + detect-and-recommend governor (native-only; NVML behind the
// optional `nvml` feature). The module's own inner cfg makes it empty on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub mod thermal_telemetry;
