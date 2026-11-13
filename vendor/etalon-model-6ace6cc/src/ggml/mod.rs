//! Portable GGML row codecs preserving the original arithmetic and layouts.
//!
//! Enabled by the `ggml` feature, which requires std for the original float
//! operations. Descriptor lookup, tensor ownership and semantic policy remain
//! with the caller. Whole-tensor ternary encoding has no row decoder here. Q6_K retains the original
//! two-byte alignment precondition of its bytemuck block view.

mod blocks;
mod fp8;
mod iq;
mod kquants;
mod layout;
mod quantize;
mod scalar;
mod soa;
use blocks::*;
use iq::*;
use kquants::*;
pub use layout::*;
pub use quantize::quantize_f32_to_q4_k_soa_tensor;
use scalar::*;
use soa::dequant_q4_k_soa;
pub use soa::{expand_q4k_tensor_to_soa, q4k_block_to_soa};
#[cfg(test)]
mod tests;

/// Dequantize one embedding row from raw mmap bytes into `out`.
/// Returns the number of `f32` elements written (≤ `out.len()`).
pub fn dequantize_row_into(
    raw: &[u8],
    ggml_type: u32,
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    if out.len() < n_elems {
        return Err(GgmlDequantError::BufferTooSmall);
    }
    match ggml_type {
        GGML_TYPE_F32 => dequant_f32(raw, n_elems, out),
        GGML_TYPE_F16 => dequant_f16(raw, n_elems, out),
        GGML_TYPE_BF16 => dequant_bf16(raw, n_elems, out),
        GGML_TYPE_Q4_0 => dequant_q4_0(raw, n_elems, out),
        GGML_TYPE_Q5_0 => dequant_q5_0(raw, n_elems, out),
        GGML_TYPE_Q5_1 => dequant_q5_1(raw, n_elems, out),
        GGML_TYPE_MXFP4 => dequant_mxfp4(raw, n_elems, out),
        GGML_TYPE_Q8_0 => dequant_q8_0(raw, n_elems, out),
        GGML_TYPE_Q4_K => dequant_q4_k(raw, n_elems, out),
        GGML_TYPE_Q5_K => dequant_q5_k(raw, n_elems, out),
        GGML_TYPE_Q4_K_SOA => dequant_q4_k_soa(raw, n_elems, out),
        GGML_TYPE_Q6_K => dequant_q6_k(raw, n_elems, out),
        GGML_TYPE_IQ4_NL => dequant_iq4_nl(raw, n_elems, out),
        GGML_TYPE_IQ4_XS => dequant_iq4_xs(raw, n_elems, out),
        GGML_TYPE_FP8_E4M3 => dequant_fp8_e4m3(raw, n_elems, out),
        _ => Err(GgmlDequantError::UnsupportedType),
    }
}

#[cfg(feature = "inference-model")]
mod descriptor;
#[cfg(feature = "inference-model")]
pub use descriptor::*;
