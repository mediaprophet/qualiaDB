//! CPU numeric kernels + pre-norm helpers used by the native/fallback LLM forward path.
//! Split out of the monolithic `gguf_bridge` module (structural refactor; no behaviour change).

use super::{RMS_NORM_EPS, VOCAB_CHUNK_ROWS};
use crate::gguf_sharder::GgufTensorInfo;

#[inline]
pub(crate) fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

#[inline]
pub(crate) fn scrub_f32_volatile(buf: &mut [f32], n: usize) {
    for v in buf.iter_mut().take(n) {
        // Prevent logits residue from surviving across decode frames.
        unsafe { core::ptr::write_volatile(v, 0.0) };
    }
}

#[inline]
pub(crate) fn update_streaming_argmax(
    chunk_logits: &[f32],
    chunk_rows: usize,
    chunk_idx: usize,
    best_token_id: &mut u32,
    max_logit: &mut f32,
) {
    update_streaming_argmax_sieved(
        chunk_logits,
        chunk_rows,
        chunk_idx,
        None,
        best_token_id,
        max_logit,
    );
}

/// Chunked argmax with optional FSM sieve mask (disallowed tokens treated as `-∞`).
#[inline]
pub(crate) fn update_streaming_argmax_sieved(
    chunk_logits: &[f32],
    chunk_rows: usize,
    chunk_idx: usize,
    sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    best_token_id: &mut u32,
    max_logit: &mut f32,
) {
    let base = chunk_idx * VOCAB_CHUNK_ROWS;
    for (local, &v) in chunk_logits.iter().take(chunk_rows).enumerate() {
        let abs_id = (base + local) as u32;
        let score = if sieve_mask.map(|m| m.allows(abs_id)).unwrap_or(true) {
            v
        } else {
            f32::NEG_INFINITY
        };
        if score > *max_logit {
            *max_logit = score;
            *best_token_id = abs_id;
        }
    }
}

#[inline]
pub(crate) fn relu_inplace(buf: &mut [f32], n: usize) {
    for v in buf.iter_mut().take(n) {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
}

/// SiLU (Swish): x * sigmoid(x) = x / (1 + e^{-x}). Llama/SmolLM2 SwiGLU gate activation.
#[inline]
pub(crate) fn silu_inplace(x: &mut [f32], n: usize) {
    for v in x.iter_mut().take(n) {
        *v = *v / (1.0 + (-*v).exp());
    }
}

#[inline]
pub(crate) fn add_residual_inplace(dst: &mut [f32], src: &[f32], n: usize) {
    for i in 0..n.min(dst.len()).min(src.len()) {
        dst[i] += src[i];
    }
}

#[inline]
pub(crate) fn rms_norm_inplace(x: &mut [f32], weight: &[f32], eps: f32) {
    let n = x.len().min(weight.len());
    if n == 0 {
        return;
    }
    let mut ss = 0.0f32;
    for i in 0..n {
        ss += x[i] * x[i];
    }
    ss /= n as f32;
    let inv_rms = 1.0 / (ss + eps).sqrt();
    for i in 0..n {
        x[i] = x[i] * inv_rms * weight[i];
    }
}

/// Dequantize a 1-D norm weight row (`attn_norm` / `ffn_norm` / `output_norm`) into `out`.
pub(crate) fn dequant_norm_row_into(
    mmap: &[u8],
    tensor_data_start: u64,
    info: &GgufTensorInfo,
    out: &mut [f32],
) -> usize {
    let n = info.dims[0] as usize;
    if n == 0 || n > out.len() {
        return 0;
    }
    let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, info) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, n, &mut out[..n]).unwrap_or(0)
}

/// Pre-norm: copy `hidden` into `h_norm`, apply RMSNorm with `norm_info` weights; return slice to use.
pub(crate) fn prepare_pre_norm_input<'a>(
    hidden: &'a [f32],
    emb_dim: usize,
    norm_info: Option<&GgufTensorInfo>,
    mmap: Option<&[u8]>,
    tensor_data_start: u64,
    h_norm: &'a mut [f32],
    norm_w: &mut [f32],
) -> &'a [f32] {
    let n_embd = emb_dim.min(hidden.len()).min(h_norm.len());
    if let (Some(mmap), Some(info)) = (mmap, norm_info) {
        if dequant_norm_row_into(mmap, tensor_data_start, info, norm_w) >= n_embd {
            h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
            rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], RMS_NORM_EPS);
            return &h_norm[..n_embd];
        }
    }
    &hidden[..n_embd]
}
