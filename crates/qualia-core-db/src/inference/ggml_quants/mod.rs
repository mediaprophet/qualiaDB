//! Canonical numerical encodings and descriptor helpers, with legacy Qualia paths.
pub use etalon_model::ggml::*;
#[cfg(test)]
use crate::gguf_sharder::GgufTensorInfo;
#[cfg(test)]
mod tests;

#[cfg(test)]
fn dequant_q4_k(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    dequantize_row_into(raw, GGML_TYPE_Q4_K, n_elems, out)
}

#[cfg(test)]
fn dequant_q4_k_soa(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    dequantize_row_into(raw, GGML_TYPE_Q4_K_SOA, n_elems, out)
}

#[cfg(test)]
fn dequant_q5_k(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    dequantize_row_into(raw, GGML_TYPE_Q5_K, n_elems, out)
}

#[cfg(test)]
fn dequant_q8_0(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    dequantize_row_into(raw, GGML_TYPE_Q8_0, n_elems, out)
}

#[cfg(test)]
fn quantize_block_f32_to_q4_k_soa(src: &[f32], out: &mut [u8]) {
    quantize_f32_to_q4_k_soa_tensor(src, src.len(), 1, out)
        .expect("valid original single-block fixture");
}
