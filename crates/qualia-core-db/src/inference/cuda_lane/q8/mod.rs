//! Native CUDA kernels for GGML Q8_0 weights.
//!
//! This module deliberately keeps the scalar oracle, CUDA source, and host runner separate so
//! schedule work cannot silently alter the byte-layout interpretation.

mod dp4a;
#[cfg(test)]
mod dp4a_lm_head_tests;
#[cfg(test)]
mod dp4a_qkv_tests;
#[cfg(test)]
mod dp4a_resid_tests;
#[cfg(test)]
mod dp4a_swiglu_tests;
#[cfg(test)]
mod dp4a_tests;
mod embedding;
mod fused;
mod kernel;
mod oracle;
mod run;
#[cfg(test)]
mod tests;

pub use oracle::{q8_0_gemv_oracle_into, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS};
pub use run::try_q8_0_cuda_gemv;

pub(crate) use dp4a::{
    q8_dp4a_qkv_rope_source, q8_dp4a_qkv_rope_warp8_source, q8_dp4a_swiglu_source,
    Q8_0_DP4A_GEMV_ENTRY, Q8_0_DP4A_GEMV_RESID_ENTRY, Q8_0_DP4A_GEMV_RESID_SRC, Q8_0_DP4A_GEMV_SRC,
    Q8_0_DP4A_QKV_ROPE_ENTRY, Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY, Q8_0_DP4A_SWIGLU_ENTRY,
    Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
pub(crate) use embedding::{Q8_0_EMBEDDING_LOOKUP_ENTRY, Q8_0_EMBEDDING_LOOKUP_SRC};
pub(crate) use fused::{
    q8_gemv_resid_source, q8_qkv_rope_source, q8_rmsnorm_qkv_rope_source, q8_rmsnorm_swiglu_source,
    q8_swiglu_source, Q8_0_GEMV_RESID_ENTRY, Q8_0_QKV_ROPE_ENTRY, Q8_0_RMSNORM_QKV_ROPE_ENTRY,
    Q8_0_RMSNORM_SWIGLU_ENTRY, Q8_0_SWIGLU_ENTRY,
};
pub(crate) use kernel::{Q8_0_GEMV_ENTRY, Q8_0_GEMV_ROWS, Q8_0_GEMV_SRC};
