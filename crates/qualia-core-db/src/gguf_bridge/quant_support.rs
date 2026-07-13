//! GPU weight-type support gates — which ggml quant types each WGSL dequant path implements.
//! Split out of the monolithic `gguf_bridge` module (structural refactor; no behaviour change).

/// Narrow CPU-fallback / legacy GPU-quant predicate. **Native is Q4_K/Q6_K ONLY** — still used by
/// the resident-logits / top-k output-projection gate (a known limitation: it makes top-k fall back
/// to argmax for Q8_0/F16 models). The GEMM hot path uses the wider [`ggml_gpu_gemm_supported`].
#[inline]
#[allow(dead_code)]
pub(crate) fn ggml_gpu_quant_supported(ggml_type: u32) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        // WASM CPU fallback dequantizes all stack_gemm-supported types.
        matches!(
            ggml_type,
            crate::ggml_quants::GGML_TYPE_F32
                | crate::ggml_quants::GGML_TYPE_F16
                | crate::ggml_quants::GGML_TYPE_BF16
                | crate::ggml_quants::GGML_TYPE_Q4_0
                | crate::ggml_quants::GGML_TYPE_Q5_0
                | crate::ggml_quants::GGML_TYPE_Q8_0
                | crate::ggml_quants::GGML_TYPE_Q4_K
                | crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                | crate::ggml_quants::GGML_TYPE_Q6_K
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K
            || ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
            || ggml_type == crate::ggml_quants::GGML_TYPE_Q6_K
    }
}

/// Weight types implemented in `fused_attention.wgsl` / `fused_transformer.wgsl` dequant.
/// F16 added for the all-F16 Llama-3.2 family: without it `dispatch_attention_layer` returned
/// `None` and (because attn_q/k/v are present, skipping the `attn_output`-only fallback) attention
/// was silently dropped from every block — coherent-looking run, garbage logits. Mirrors the #49
/// Q8_0 widening. `fused_attention.wgsl::dequant_f16_weight` is the matching shader-side path.
#[inline]
pub(crate) fn ggml_gpu_attention_shader_supported(ggml_type: u32) -> bool {
    matches!(
        ggml_type,
        crate::ggml_quants::GGML_TYPE_F16
            | crate::ggml_quants::GGML_TYPE_BF16
            | crate::ggml_quants::GGML_TYPE_Q4_0
            | crate::ggml_quants::GGML_TYPE_Q5_0
            | crate::ggml_quants::GGML_TYPE_Q8_0
            | crate::ggml_quants::GGML_TYPE_Q4_K
            | crate::ggml_quants::GGML_TYPE_Q4_K_SOA
            | crate::ggml_quants::GGML_TYPE_Q6_K
    )
}

/// Weight types the native GEMM shader (`fused_transformer.wgsl`) actually dequantizes — WIDER than
/// `ggml_gpu_quant_supported` (Q4_K/Q6_K only). The GEMM `dequant_weight` also implements
/// Q4_0/Q5_0/Q8_0 (identical code to the attention shader), but the narrow predicate was silently
/// routing Q8_0 FFN + output-projection GEMMs to the CPU `stack_gemm_quant` fallback. Widening this
/// is the GEMM-side analogue of the #49 attention-support fix. Verified end-to-end for Q8_0
/// (SmolLM2-360M-q8_0); Q4_0/Q5_0 share the proven dequant but have no resident test model yet.
#[inline]
pub(crate) fn ggml_gpu_gemm_supported(ggml_type: u32) -> bool {
    matches!(
        ggml_type,
        crate::ggml_quants::GGML_TYPE_F16
            | crate::ggml_quants::GGML_TYPE_BF16
            | crate::ggml_quants::GGML_TYPE_Q4_0
            | crate::ggml_quants::GGML_TYPE_Q5_0
            | crate::ggml_quants::GGML_TYPE_Q8_0
            | crate::ggml_quants::GGML_TYPE_Q4_K
            | crate::ggml_quants::GGML_TYPE_Q4_K_SOA
            | crate::ggml_quants::GGML_TYPE_Q6_K
    )
}
