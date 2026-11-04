//! Preserved final FFN conversion and AWQ behavior.
use super::*;
/// Task #12 / STELLAR §A — like [`compile_gguf_to_p64`] but **ternary-packs the FFN projections**
/// (gate/up/down) during the compile, producing a **complete, runnable** P64: hyperparameters +
/// tokenizer are preserved (so the live loader boots it and builds the KV cache), while the FFN
/// tensors are BitNet-1.58b ternary blobs (`ternary::dequantize_blob` / the 2-bit GPU kernel).
/// Attention / norms / embeddings stay verbatim at their source precision. This is the loadable
/// container the live FFN-ternary dispatch path will run + measure against.
pub fn compile_gguf_to_p64_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, None, 0.0, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn(input, page_log2)
}

/// Target quantization for the FFN tensors in an AWQ P64 compile.
pub use etalon_model::p64::convert::FfnQuant;

/// AWQ-aware ternary FFN compile.
pub fn compile_gguf_to_p64_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware **Q4_0** FFN compile (Path A) — FFN packed to 4-bit Q4_0 (AWQ's design regime); all else
/// verbatim from the source GGUF.
pub fn compile_gguf_to_p64_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Q4_0)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_q4_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware FFN-quantized P64 compile. `quant` selects the FFN target (ternary or Q4_0). When
/// `awq_scales` is `Some` (per-layer per-input-channel salience from [`crate::llm_awq::snapshot`]) the
/// gate/up input channel `i` is scaled by `s_i^alpha` before packing and `ffn_norm` is divided by
/// `s_i^alpha` — mathematically exact in f32 (`(X·norm/s^a)·(W·s^a)=(X·norm)·W`) — moving salient
/// channels into a range the quant grid represents better. `awq_scales = None` / `alpha == 0.0`
/// reproduces the plain (un-calibrated) compile. The down projection is left un-scaled (no clean fold
/// site — a v2 item). Everything outside the FFN passes through verbatim from the source GGUF.
pub fn compile_gguf_to_p64_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    let base = compile_gguf_to_p64(input, page_log2)?;
    let source = GgufTensorIndex::from_gguf(input);
    etalon_model::p64::convert::requantize_ffn(
        input,
        base,
        source.tensor_data_start as usize,
        &source.entries,
        Default::default(),
        awq_scales,
        alpha,
        quant,
    )
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, quant)
}
