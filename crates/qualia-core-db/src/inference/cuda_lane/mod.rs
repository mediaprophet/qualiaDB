//! CUDA inference lane — dense batch GEMM via persistent WMMA (mode=`cuda`).
//!
//! # Goal
//! Prefer tensor-core dense matmul for **batch prefill-shaped** GEMMs when:
//! - `InferenceMode::CudaTc` is active (`prefer_tensor_core_gemm`)
//! - dims can be padded to multiples of 16
//! - a dense f32 weight matrix is available (f16 p64 expand, or one-shot dequant)
//!
//! Weight rows are **cached on the CUDA slab** by content fingerprint so subsequent
//! chunks do not re-upload the same matrix (no host thrash for repeated layers).
//!
//! # Honest limits
//! - Not a fused Q4_K dequant-GEMV on-device (llama.cpp-class) — that remains M2b.
//! - Quantized weights must be dequantized once to dense f32 before first cache insert.
//! - Slab is finite (256 MiB); LRU-ish eviction of oldest entries when full.

#![cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]

mod attention;
mod device;
mod gemv;
mod mega_pass;
mod paged_attention;
mod q8;
mod tuning;
mod weight_cache;

pub use attention::try_q4k_soa_attention_device;
pub(crate) use device::{
    decode_graph_h2d_bytes_per_token, decode_graph_key, decode_graph_node_count,
};
pub use device::{
    device_kv_ready, ensure_device_kv_cache, preload_q4k_soa_weights, preload_resident_blob,
    q4k_device_weight_count, q4k_weight_resident, warm_cuda_context,
};
pub use gemv::{
    try_q4k_soa_ffn_block, try_q4k_soa_ffn_block_residual, try_q4k_soa_fused_swiglu,
    try_q4k_soa_gemv, try_q4k_soa_qkv,
};
pub(crate) use mega_pass::try_cuda_mega_pass_with_token;
pub use mega_pass::{
    prepare_mega_pass_kernels, try_cuda_mega_pass, MegaPassLayerDims, MegaPassLayerWeights,
    MegaPassPlanView, MegaPassWeightLayout,
};
pub use q8::{q8_0_gemv_oracle_into, try_q8_0_cuda_gemv, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS};
pub(crate) use tuning::cuda_q8_tuning;
pub use weight_cache::{
    cache_dense_weight, cache_dense_weight_direct, clear_weight_cache, dense_weight_cached,
    try_cuda_batch_gemv, try_cuda_batch_gemv_cached, try_cuda_batch_gemv_cached_only,
    weight_cache_len, weight_fingerprint, MAX_DENSE_ELEMS,
};
