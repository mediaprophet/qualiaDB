//! Cold kernel preparation for the captured CUDA decode pass.

use super::super::device::{ensure_device, multi_weight_device};
use super::super::paged_attention::{
    PAGED_GQA_SEGMENTED_MERGE_ENTRY, PAGED_GQA_SEGMENTED_PARTIAL_ENTRY,
    PAGED_GQA_SEGMENTED_SRC, PAGED_GQA_TILED_ENTRY, PAGED_GQA_TILED_SRC,
};
use super::super::q8::{
    q8_dp4a_qkv_rope_source, q8_dp4a_qkv_rope_warp8_source, q8_dp4a_swiglu_source,
    q8_gemv_resid_source, q8_qkv_rope_source, q8_rmsnorm_qkv_rope_source, q8_rmsnorm_swiglu_source,
    q8_swiglu_source, Q8_0_DP4A_GEMV_ENTRY, Q8_0_DP4A_GEMV_RESID_ENTRY, Q8_0_DP4A_GEMV_RESID_SRC,
    Q8_0_DP4A_GEMV_SRC, Q8_0_DP4A_QKV_ROPE_ENTRY, Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
    Q8_0_DP4A_SWIGLU_ENTRY, Q8_0_EMBEDDING_LOOKUP_ENTRY, Q8_0_EMBEDDING_LOOKUP_SRC,
    Q8_0_GEMV_ENTRY, Q8_0_GEMV_RESID_ENTRY, Q8_0_GEMV_SRC, Q8_0_QKV_ROPE_ENTRY,
    Q8_0_RMSNORM_QKV_ROPE_ENTRY, Q8_0_RMSNORM_SWIGLU_ENTRY, Q8_0_SWIGLU_ENTRY,
    Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use crate::wgsl_forge::emit::cuda_c::{
    ARGMAX_F32_ENTRY, ARGMAX_F32_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_RESID_ENTRY,
    Q4K_SOA_GEMV_RESID_SRC, Q4K_SOA_GEMV_SRC, Q4K_SOA_WMMA_GEMV_ENTRY,
    Q4K_SOA_WMMA_GEMV_RESID_ENTRY, Q4K_SOA_WMMA_GEMV_RESID_SRC, Q4K_SOA_WMMA_GEMV_SRC,
    RMSNORM_F32_ENTRY, RMSNORM_F32_SRC,
};
use crate::wgsl_forge::emit::cuda_c_fused::{
    KV_SLOT_WRITE_BOTH_ENTRY, KV_SLOT_WRITE_BOTH_SRC, Q4K_SOA_RMSNORM_QKV_ROPE_ENTRY,
    Q4K_SOA_RMSNORM_QKV_ROPE_SRC, Q4K_SOA_RMSNORM_SWIGLU_ENTRY, Q4K_SOA_RMSNORM_SWIGLU_SRC,
};
use crate::wgsl_forge::execute::CudaPipeline;

/// Compile/load every kernel that a prepared mega-pass may dispatch.
///
/// Called during cold plan construction so the token path only performs allocation-free module
/// cache lookups and `Arc` increments.
pub fn prepare_mega_pass_kernels() -> bool {
    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let context = &guard.as_ref().unwrap().ctx;
    let kernels: [(&str, &str, &[u32]); 25] = [
        (
            Q4K_SOA_RMSNORM_QKV_ROPE_SRC,
            Q4K_SOA_RMSNORM_QKV_ROPE_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        (
            KV_SLOT_WRITE_BOTH_SRC,
            KV_SLOT_WRITE_BOTH_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6],
        ),
        (
            PAGED_GQA_TILED_SRC,
            PAGED_GQA_TILED_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ),
        (
            PAGED_GQA_SEGMENTED_SRC,
            PAGED_GQA_SEGMENTED_PARTIAL_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ),
        (
            PAGED_GQA_SEGMENTED_SRC,
            PAGED_GQA_SEGMENTED_MERGE_ENTRY,
            &[0, 1, 2],
        ),
        (
            Q4K_SOA_WMMA_GEMV_RESID_SRC,
            Q4K_SOA_WMMA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        ),
        (
            Q4K_SOA_GEMV_RESID_SRC,
            Q4K_SOA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        ),
        (
            Q4K_SOA_RMSNORM_SWIGLU_SRC,
            Q4K_SOA_RMSNORM_SWIGLU_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        ),
        (RMSNORM_F32_SRC, RMSNORM_F32_ENTRY, &[0, 1, 2, 3]),
        (
            Q4K_SOA_WMMA_GEMV_SRC,
            Q4K_SOA_WMMA_GEMV_ENTRY,
            &[0, 1, 2, 3],
        ),
        (Q4K_SOA_GEMV_SRC, Q4K_SOA_GEMV_ENTRY, &[0, 1, 2, 3]),
        (ARGMAX_F32_SRC, ARGMAX_F32_ENTRY, &[0, 1, 2]),
        (
            Q8_0_EMBEDDING_LOOKUP_SRC,
            Q8_0_EMBEDDING_LOOKUP_ENTRY,
            &[0, 1, 2, 3],
        ),
        (
            q8_rmsnorm_qkv_rope_source(),
            Q8_0_RMSNORM_QKV_ROPE_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        (
            q8_qkv_rope_source(),
            Q8_0_QKV_ROPE_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ),
        (
            q8_rmsnorm_swiglu_source(),
            Q8_0_RMSNORM_SWIGLU_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        ),
        (q8_swiglu_source(), Q8_0_SWIGLU_ENTRY, &[0, 1, 2, 3, 4]),
        (
            Q8_ACTIVATION_QUANT_SRC,
            Q8_ACTIVATION_QUANT_ENTRY,
            &[0, 1, 2, 3],
        ),
        (
            q8_dp4a_swiglu_source(),
            Q8_0_DP4A_SWIGLU_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        ),
        (
            q8_dp4a_qkv_rope_source(),
            Q8_0_DP4A_QKV_ROPE_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        (
            q8_dp4a_qkv_rope_warp8_source(),
            Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        (
            Q8_0_DP4A_GEMV_RESID_SRC,
            Q8_0_DP4A_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        ),
        (Q8_0_DP4A_GEMV_SRC, Q8_0_DP4A_GEMV_ENTRY, &[0, 1, 2, 3, 4]),
        (
            q8_gemv_resid_source(),
            Q8_0_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        ),
        (Q8_0_GEMV_SRC, Q8_0_GEMV_ENTRY, &[0, 1, 2, 3]),
    ];
    kernels.iter().all(
        |(source, entry, bindings)| match CudaPipeline::compile_cuda_c_source_cached(
            context, source, entry, bindings,
        ) {
            Ok(_) => true,
            Err(error) => {
                log::warn!("mega_pass|prepare_kernel|entry={entry}|{error:?}");
                false
            }
        },
    )
}
