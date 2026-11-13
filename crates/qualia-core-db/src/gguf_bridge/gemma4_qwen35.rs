//! Qwen3.5 gated full attention and shared split-half RoPE.
//! Gemma execution is owned by the token-aware gemma4 child module.
//!
//! All hot-path operations are zero-heap (`assert_zero_alloc` compliant) and reuse
//! `QTensorEngine::dispatch_gemm_into` for GPU / AVX2+FMA3 matrix-vector projections.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn rope_neox_partial_inplace(
    vec: &mut [f32],
    n_heads: usize,
    head_dim: usize,
    rope_dim: usize,
    pos: u32,
    base: f32,
    scale: f32,
    freq_factors: Option<&[f32]>,
) {
    let rot = rope_dim.min(head_dim);
    let half = rot / 2;
    if half == 0 {
        return;
    }
    let scale = if scale > 0.0 && scale.is_finite() {
        scale
    } else {
        1.0
    };
    let scaled_pos = pos as f32 / scale;
    for head in 0..n_heads {
        let off = head * head_dim;
        if off + head_dim > vec.len() {
            return;
        }
        for i in 0..half {
            let factor = freq_factors
                .and_then(|f| f.get(i).copied())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or(1.0);
            let theta = (scaled_pos * base.powf(-2.0 * i as f32 / rot as f32)) / factor;
            let (s, c) = theta.sin_cos();
            let x0 = vec[off + i];
            let x1 = vec[off + half + i];
            vec[off + i] = x0 * c - x1 * s;
            vec[off + half + i] = x1 * c + x0 * s;
        }
    }
}

impl QTensorEngine {
    /// True when `tensors` is a Qwen 3.5 gated full-attention block (`q_out == 2 * n_head * head_dim` with QK-norm).
    pub(crate) fn is_qwen35_gated_attention_layer(
        index: &crate::gguf_sharder::GgufTensorIndex,
        tensors: &crate::gguf_sharder::LayerTensors,
    ) -> bool {
        if tensors.is_hybrid_ssm_layer()
            || tensors.attn_q_norm.is_none()
            || tensors.attn_k_norm.is_none()
        {
            return false;
        }
        let Some(q_info) = tensors.attn_q.as_ref() else {
            return false;
        };
        let (_, q_out) = Self::matmul_dims(q_info);
        let h = index.hyperparams;
        let q_dim = (h.n_head * h.head_dim()) as usize;
        q_dim > 0 && q_out == 2 * q_dim
    }

    /// Execute a Qwen 3.5 gated full-attention layer (`[Q, gate]` interleaved per head, per-head QK-norm,
    /// partial split-half RoPE, causal GQA SDPA, `sigmoid(gate)`, and `attn_output` projection).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch_qwen35_gated_attention_layer(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &[f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
    ) -> Option<usize> {
        let layout = self.kv_layout?;
        let q_info = tensors.attn_q.as_ref()?;
        let k_info = tensors.attn_k.as_ref()?;
        let v_info = tensors.attn_v.as_ref()?;
        let q_norm_info = tensors.attn_q_norm.as_ref()?;
        let k_norm_info = tensors.attn_k_norm.as_ref()?;
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        let q_dim = n_head * head_dim;
        let q_gate_dim = 2 * q_dim;
        let kv_dim = n_kv * head_dim;
        if n_embd == 0
            || n_embd > MAX_HIDDEN_DIM
            || emb_dim < n_embd
            || head_dim == 0
            || head_dim > 512
            || n_head == 0
            || n_kv == 0
            || n_head % n_kv != 0
            || q_gate_dim > 8192
            || kv_dim > 4096
        {
            return None;
        }
        let eps = if h.rms_norm_eps > 0.0 {
            h.rms_norm_eps
        } else {
            RMS_NORM_EPS
        };

        // 1. Pre-norm (attn_norm.weight)
        let mut h_norm = [0.0f32; MAX_HIDDEN_DIM];
        h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
        if let Some(an_info) = tensors.attn_norm.as_ref() {
            let mmap = self.model_bytes()?;
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                an_info,
                &mut norm_w[..n_embd],
            ) >= n_embd
            {
                rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], eps);
            }
        }

        // 2. Project Q+gate, K, V
        let mut q_gate_buf = [0.0f32; 8192];
        let mut k_buf = [0.0f32; 4096];
        let mut v_buf = [0.0f32; 4096];
        if !self.dispatch_gemm_into(
            index,
            q_info,
            &h_norm[..n_embd],
            &mut q_gate_buf[..q_gate_dim],
            n_embd,
            q_gate_dim,
        ) || !self.dispatch_gemm_into(
            index,
            k_info,
            &h_norm[..n_embd],
            &mut k_buf[..kv_dim],
            n_embd,
            kv_dim,
        ) || !self.dispatch_gemm_into(
            index,
            v_info,
            &h_norm[..n_embd],
            &mut v_buf[..kv_dim],
            n_embd,
            kv_dim,
        ) {
            return None;
        }

        // 3. De-interleave [Q_h, gate_h] per head
        let mut q_buf = [0.0f32; 4096];
        let mut gate_buf = [0.0f32; 4096];
        for head in 0..n_head {
            let src = &q_gate_buf[head * 2 * head_dim..(head + 1) * 2 * head_dim];
            q_buf[head * head_dim..(head + 1) * head_dim].copy_from_slice(&src[..head_dim]);
            gate_buf[head * head_dim..(head + 1) * head_dim].copy_from_slice(&src[head_dim..]);
        }

        // 4. Per-head QK-norm
        {
            let mmap = self.model_bytes()?;
            let mut q_norm_w = [0.0f32; 512];
            let mut k_norm_w = [0.0f32; 512];
            if dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                q_norm_info,
                &mut q_norm_w[..head_dim],
            ) < head_dim
                || dequant_norm_row_into(
                    mmap,
                    index.tensor_data_start,
                    k_norm_info,
                    &mut k_norm_w[..head_dim],
                ) < head_dim
            {
                return None;
            }
            for head in 0..n_head {
                rms_norm_inplace(
                    &mut q_buf[head * head_dim..(head + 1) * head_dim],
                    &q_norm_w[..head_dim],
                    eps,
                );
            }
            for kh in 0..n_kv {
                rms_norm_inplace(
                    &mut k_buf[kh * head_dim..(kh + 1) * head_dim],
                    &k_norm_w[..head_dim],
                    eps,
                );
            }
        }

        // 5. Partial split-half (neox) RoPE
        let rope_dim = if h.rope_dim > 0 {
            h.rope_dim as usize
        } else {
            head_dim
        };
        let rope_base = h.effective_rope_freq_base();
        let rope_scale = h.effective_rope_scale();
        rope_neox_partial_inplace(
            &mut q_buf[..q_dim],
            n_head,
            head_dim,
            rope_dim,
            token_idx,
            rope_base,
            rope_scale,
            None,
        );
        rope_neox_partial_inplace(
            &mut k_buf[..kv_dim],
            n_kv,
            head_dim,
            rope_dim,
            token_idx,
            rope_base,
            rope_scale,
            None,
        );

        // 6. Write K/V to host KV cache and run causal GQA SDPA
        let mut attn_out = [0.0f32; 4096];
        {
            let kv = self.kv_cache_cpu.as_deref_mut()?;
            let max_ctx = layout.max_context.max(1);
            let slot_stride = (layout.layer_stride / max_ctx).max(2 * kv_dim as u32) as usize;
            if (layout.layer_stride as usize) < (max_ctx as usize) * slot_stride {
                return None;
            }
            let slot = (token_idx % max_ctx) as usize;
            let layer_base = layer as usize * layout.layer_stride as usize;
            let slot_base = layer_base + slot * slot_stride;
            if slot_base + 2 * kv_dim > kv.len() {
                return None;
            }
            kv[slot_base..slot_base + kv_dim].copy_from_slice(&k_buf[..kv_dim]);
            kv[slot_base + kv_dim..slot_base + 2 * kv_dim].copy_from_slice(&v_buf[..kv_dim]);

            let valid_len = (token_idx + 1).min(max_ctx) as usize;
            let gqa = n_head / n_kv;
            let scale = 1.0f32 / (head_dim as f32).sqrt();
            for qh in 0..n_head {
                let kh = qh / gqa;
                let q_head = &q_buf[qh * head_dim..(qh + 1) * head_dim];
                let out_head = &mut attn_out[qh * head_dim..(qh + 1) * head_dim];
                let mut max_score = f32::NEG_INFINITY;
                let mut sum_exp = 0.0f32;
                for t in 0..valid_len {
                    let t_base = layer_base + t * slot_stride;
                    let k_head = &kv[t_base + kh * head_dim..t_base + (kh + 1) * head_dim];
                    let v_head =
                        &kv[t_base + kv_dim + kh * head_dim..t_base + kv_dim + (kh + 1) * head_dim];
                    let mut dot = 0.0f32;
                    for d in 0..head_dim {
                        dot += q_head[d] * k_head[d];
                    }
                    let score = dot * scale;
                    if score > max_score {
                        let rescale = (max_score - score).exp();
                        for v in out_head.iter_mut() {
                            *v *= rescale;
                        }
                        sum_exp = sum_exp * rescale + 1.0f32;
                        max_score = score;
                        for d in 0..head_dim {
                            out_head[d] += v_head[d];
                        }
                    } else {
                        let w = (score - max_score).exp();
                        sum_exp += w;
                        for d in 0..head_dim {
                            out_head[d] += w * v_head[d];
                        }
                    }
                }
                if sum_exp > 0.0 {
                    let inv = 1.0f32 / sum_exp;
                    for d in 0..head_dim {
                        let gate = 1.0f32 / (1.0f32 + (-gate_buf[qh * head_dim + d]).exp());
                        out_head[d] = (out_head[d] * inv) * gate;
                    }
                }
            }
        }

        // 7. Output projection
        if let Some(out_info) = tensors.attn_output.as_ref() {
            let (o_in, o_out) = Self::matmul_dims(out_info);
            if o_in <= q_dim
                && o_out <= scratch_a.len()
                && self.dispatch_gemm_into(
                    index,
                    out_info,
                    &attn_out[..o_in],
                    &mut scratch_a[..o_out],
                    o_in,
                    o_out,
                )
            {
                return Some(o_out.min(emb_dim));
            }
            return None;
        }
        let n = q_dim.min(emb_dim).min(scratch_a.len());
        scratch_a[..n].copy_from_slice(&attn_out[..n]);
        Some(n)
    }
}

#[cfg(test)]
#[path = "gemma4_qwen35/tests.rs"]
mod tests;
