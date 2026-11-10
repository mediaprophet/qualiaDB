//! Pure-Rust execution paths for Qwen 3.5 Gated Full-Attention and Gemma 4 (E4B / 26B-A4B).
//!
//! All hot-path operations are zero-heap (`assert_zero_alloc` compliant) and reuse
//! `QTensorEngine::dispatch_gemm_into` for GPU / AVX2+FMA3 matrix-vector projections.

use super::*;

const MAX_G4_PLE_TOTAL: usize = 64 * 256;
const MAX_G4_PLE_SLOTS: usize = 64;

std::thread_local! {
    static GEMMA4_PLE_RING: core::cell::RefCell<[[f32; MAX_G4_PLE_TOTAL]; MAX_G4_PLE_SLOTS]> =
        const { core::cell::RefCell::new([[0.0f32; MAX_G4_PLE_TOTAL]; MAX_G4_PLE_SLOTS]) };
}

#[inline]
fn gelu_pytorch_tanh(x: f32) -> f32 {
    let inner = 0.797_884_56_f32 * (x + 0.044_715_f32 * x * x * x);
    0.5f32 * x * (1.0f32 + inner.tanh())
}

#[inline]
fn unit_rms_norm_inplace(x: &mut [f32], eps: f32) {
    if x.is_empty() {
        return;
    }
    let mut sum_sq = 0.0f32;
    for &v in x.iter() {
        sum_sq += v * v;
    }
    let inv = 1.0f32 / ((sum_sq / x.len() as f32) + eps).sqrt();
    for v in x.iter_mut() {
        *v *= inv;
    }
}

/// Resolve a per-block GGUF tensor by name. Pinned etalon 6ace6cc `LayerTensors`
/// omits Gemma 4 optional weights, and `find_layer_tensor` is private there.
fn named_layer_tensor(
    index: &crate::gguf_sharder::GgufTensorIndex,
    layer: u32,
    suffix: &[u8],
) -> Option<etalon_model::inference::TensorInfo> {
    let mut name = [0u8; 96];
    let n = crate::gguf_sharder::write_blk_tensor_name(layer, suffix, &mut name);
    if n == 0 {
        None
    } else {
        index.tensor_info(&name[..n])
    }
}

#[allow(clippy::too_many_arguments)]
fn rope_neox_partial_inplace(
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
            if dequant_norm_row_into(mmap, index.tensor_data_start, an_info, &mut norm_w[..n_embd])
                >= n_embd
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

    /// Execute one Gemma 4 transformer block (`ARCH_GEMMA4`, supporting both E4B with PLE + double-wide FFN
    /// and 26B-A4B with parallel dense + 128-expert MoE).
    pub(crate) fn dispatch_gemma4_layer(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let n_layer = h.n_layer as usize;
        let n_head = h.n_head as usize;
        if n_embd == 0
            || n_embd > MAX_HIDDEN_DIM
            || emb_dim < n_embd
            || n_layer == 0
            || n_head == 0
        {
            return false;
        }
        let eps = if h.rms_norm_eps > 0.0 {
            h.rms_norm_eps
        } else {
            1e-6
        };
        let slot_ring = (token_idx as usize) & (MAX_G4_PLE_SLOTS - 1);

        // Layer 0 entry: scale raw token embedding by sqrt(n_embd) and compute PLE per-layer token inputs.
        if layer == 0 {
            let token_id = 0u32;
            let embed_scale = (n_embd as f32).sqrt();
            for v in &mut hidden[..n_embd] {
                *v *= embed_scale;
            }
            if let (Some(ple_embd_info), Some(ple_proj_info), Some(ple_norm_info)) = (
                index.ple_ngram_embedding_info(),
                index.tensor_info(b"per_layer_model_proj.weight"),
                index.tensor_info(b"per_layer_proj_norm.weight"),
            ) {
                let total_ple = ple_embd_info.dims[0] as usize;
                let ple_dim = if h.ple_embd_per_layer > 0 {
                    h.ple_embd_per_layer as usize
                } else {
                    total_ple / n_layer.max(1)
                };
                if total_ple > 0
                    && total_ple <= MAX_G4_PLE_TOTAL
                    && ple_dim > 0
                    && ple_dim <= 512
                    && n_layer * ple_dim == total_ple
                {
                    let mut tok_ple = [0.0f32; MAX_G4_PLE_TOTAL];
                    let mut proj_ple = [0.0f32; MAX_G4_PLE_TOTAL];
                    let mut proj_norm_w = [0.0f32; 512];
                    let Some(mmap) = self.model_bytes() else {
                        return false;
                    };
                    if index.dequantize_ple_row_into(
                        mmap,
                        u64::from(token_id),
                        &mut tok_ple[..total_ple],
                    ) < total_ple
                        || dequant_norm_row_into(
                            mmap,
                            index.tensor_data_start,
                            &ple_norm_info,
                            &mut proj_norm_w[..ple_dim],
                        ) < ple_dim
                    {
                        return false;
                    }
                    let ple_emb_scale = (ple_dim as f32).sqrt();
                    for v in &mut tok_ple[..total_ple] {
                        *v *= ple_emb_scale;
                    }
                    if !self.dispatch_gemm_into(
                        index,
                        &ple_proj_info,
                        &hidden[..n_embd],
                        &mut proj_ple[..total_ple],
                        n_embd,
                        total_ple,
                    ) {
                        return false;
                    }
                    let model_proj_scale = (n_embd as f32).powf(-0.5);
                    for v in &mut proj_ple[..total_ple] {
                        *v *= model_proj_scale;
                    }
                    let inv_sqrt2 = (2.0f32).powf(-0.5);
                    for l in 0..n_layer {
                        let s = l * ple_dim;
                        let e = s + ple_dim;
                        rms_norm_inplace(&mut proj_ple[s..e], &proj_norm_w[..ple_dim], eps);
                        for i in s..e {
                            proj_ple[i] = (proj_ple[i] + tok_ple[i]) * inv_sqrt2;
                        }
                    }
                    GEMMA4_PLE_RING.with(|cell| {
                        cell.borrow_mut()[slot_ring][..total_ple]
                            .copy_from_slice(&proj_ple[..total_ple]);
                    });
                }
            }
        }

        let tensors = index.get_layer_tensors(layer);
        let Some(layout) = self.kv_layout else {
            return false;
        };

        // === 1. Attention Sub-Block ===
        let Some(q_info) = tensors.attn_q.as_ref() else {
            return false;
        };
        let (_, q_out) = Self::matmul_dims(q_info);
        if q_out == 0 || q_out % n_head != 0 || q_out > 8192 {
            return false;
        }
        let head_dim_layer = q_out / n_head;
        if head_dim_layer == 0 || head_dim_layer > 512 {
            return false;
        }
        let is_swa = h.head_dim_swa > 0 && head_dim_layer == h.head_dim_swa as usize;

        let mut h_norm = [0.0f32; MAX_HIDDEN_DIM];
        h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
        if let Some(an_info) = tensors.attn_norm.as_ref() {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, an_info, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], eps);
        }

        let mut q_buf = [0.0f32; 8192];
        if !self.dispatch_gemm_into(
            index,
            q_info,
            &h_norm[..n_embd],
            &mut q_buf[..q_out],
            n_embd,
            q_out,
        ) {
            return false;
        }
        if let Some(qn_info) = tensors.attn_q_norm.as_ref() {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut q_norm_w = [0.0f32; 512];
            if dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                qn_info,
                &mut q_norm_w[..head_dim_layer],
            ) < head_dim_layer
            {
                return false;
            }
            for head in 0..n_head {
                rms_norm_inplace(
                    &mut q_buf[head * head_dim_layer..(head + 1) * head_dim_layer],
                    &q_norm_w[..head_dim_layer],
                    eps,
                );
            }
        }

        let mut rope_freqs_buf = [1.0f32; 256];
        let mut has_rope_freqs = false;
        if !is_swa {
            let mut rf_name = [0u8; 64];
            let rf_len = crate::gguf_sharder::write_blk_tensor_name(
                layer,
                b"rope_freqs.weight",
                &mut rf_name,
            );
            if let Some(rf_info) = index
                .tensor_info(&rf_name[..rf_len])
                .or_else(|| index.tensor_info(b"rope_freqs.weight"))
            {
                if let Some(mmap) = self.model_bytes() {
                    let count = (rf_info.dims[0] as usize).min(256);
                    if dequant_norm_row_into(
                        mmap,
                        index.tensor_data_start,
                        &rf_info,
                        &mut rope_freqs_buf[..count],
                    ) == count
                    {
                        has_rope_freqs = true;
                    }
                }
            }
        }
        let rope_dim = if is_swa {
            if h.rope_dim_swa > 0 {
                h.rope_dim_swa as usize
            } else {
                head_dim_layer
            }
        } else if h.rope_dim > 0 {
            h.rope_dim as usize
        } else {
            head_dim_layer
        };
        let rope_base = if is_swa {
            if h.rope_freq_base_swa > 0.0 {
                h.rope_freq_base_swa
            } else {
                10_000.0
            }
        } else {
            h.effective_rope_freq_base()
        };
        let rope_scale = h.effective_rope_scale();
        rope_neox_partial_inplace(
            &mut q_buf[..q_out],
            n_head,
            head_dim_layer,
            rope_dim,
            token_idx,
            rope_base,
            rope_scale,
            has_rope_freqs.then_some(&rope_freqs_buf[..]),
        );

        let has_own_kv = if h.shared_kv_layers > 0 && (h.shared_kv_layers as usize) < n_layer {
            (layer as usize) < n_layer - (h.shared_kv_layers as usize)
        } else {
            tensors.attn_k.is_some()
        };
        let (kv_source_layer, n_kv_layer) = if has_own_kv {
            let Some(k_info) = tensors.attn_k.as_ref() else {
                return false;
            };
            let (_, kv_dim_layer) = Self::matmul_dims(k_info);
            if kv_dim_layer == 0
                || kv_dim_layer % head_dim_layer != 0
                || kv_dim_layer > 4096
            {
                return false;
            }
            let n_kv_l = kv_dim_layer / head_dim_layer;
            if n_head % n_kv_l != 0 {
                return false;
            }
            let mut k_buf = [0.0f32; 4096];
            let mut v_buf = [0.0f32; 4096];
            if !self.dispatch_gemm_into(
                index,
                k_info,
                &h_norm[..n_embd],
                &mut k_buf[..kv_dim_layer],
                n_embd,
                kv_dim_layer,
            ) {
                return false;
            }
            if let Some(v_info) = tensors.attn_v.as_ref() {
                if !self.dispatch_gemm_into(
                    index,
                    v_info,
                    &h_norm[..n_embd],
                    &mut v_buf[..kv_dim_layer],
                    n_embd,
                    kv_dim_layer,
                ) {
                    return false;
                }
            } else {
                // Gemma 4 global attention k_eq_v: V is raw K before k_norm and RoPE.
                v_buf[..kv_dim_layer].copy_from_slice(&k_buf[..kv_dim_layer]);
            }
            if let Some(kn_info) = tensors.attn_k_norm.as_ref() {
                let Some(mmap) = self.model_bytes() else {
                    return false;
                };
                let mut k_norm_w = [0.0f32; 512];
                if dequant_norm_row_into(
                    mmap,
                    index.tensor_data_start,
                    kn_info,
                    &mut k_norm_w[..head_dim_layer],
                ) < head_dim_layer
                {
                    return false;
                }
                for kh in 0..n_kv_l {
                    rms_norm_inplace(
                        &mut k_buf[kh * head_dim_layer..(kh + 1) * head_dim_layer],
                        &k_norm_w[..head_dim_layer],
                        eps,
                    );
                }
            }
            for kh in 0..n_kv_l {
                unit_rms_norm_inplace(
                    &mut v_buf[kh * head_dim_layer..(kh + 1) * head_dim_layer],
                    eps,
                );
            }
            rope_neox_partial_inplace(
                &mut k_buf[..kv_dim_layer],
                n_kv_l,
                head_dim_layer,
                rope_dim,
                token_idx,
                rope_base,
                rope_scale,
                has_rope_freqs.then_some(&rope_freqs_buf[..]),
            );
            let Some(kv) = self.kv_cache_cpu.as_deref_mut() else {
                return false;
            };
            let max_ctx = layout.max_context.max(1);
            let slot_stride = (layout.layer_stride / max_ctx).max(2 * kv_dim_layer as u32) as usize;
            let max_slots = ((layout.layer_stride as usize) / slot_stride.max(1)).max(1) as u32;
            let slot = (token_idx % max_slots) as usize;
            let layer_base = layer as usize * layout.layer_stride as usize;
            let slot_base = layer_base + slot * slot_stride;
            if slot_base + 2 * kv_dim_layer > kv.len() {
                return false;
            }
            kv[slot_base..slot_base + kv_dim_layer].copy_from_slice(&k_buf[..kv_dim_layer]);
            kv[slot_base + kv_dim_layer..slot_base + 2 * kv_dim_layer]
                .copy_from_slice(&v_buf[..kv_dim_layer]);
            (layer, n_kv_l)
        } else {
            // Shared KV layer: reuse K/V from the last non-shared layer with the same head_dim_layer.
            let kv_from_start = if h.shared_kv_layers > 0 && (h.shared_kv_layers as usize) < n_layer
            {
                (n_layer - h.shared_kv_layers as usize) as u32
            } else {
                layer
            };
            let mut found = None;
            for prev in (0..kv_from_start).rev() {
                let prev_t = index.get_layer_tensors(prev);
                if let (Some(pq), Some(pk)) = (prev_t.attn_q.as_ref(), prev_t.attn_k.as_ref()) {
                    if Self::matmul_dims(pq).1 == q_out {
                        let kv_dim_prev = Self::matmul_dims(pk).1;
                        if kv_dim_prev > 0 && kv_dim_prev % head_dim_layer == 0 {
                            found = Some((prev, kv_dim_prev / head_dim_layer));
                            break;
                        }
                    }
                }
            }
            let Some((src_l, n_kv_l)) = found else {
                return false;
            };
            (src_l, n_kv_l)
        };

        let kv_dim_layer = n_kv_layer * head_dim_layer;
        let mut attn_out = [0.0f32; 8192];
        {
            let Some(kv) = self.kv_cache_cpu.as_deref() else {
                return false;
            };
            let max_ctx = layout.max_context.max(1);
            let slot_stride = (layout.layer_stride / max_ctx).max(2 * kv_dim_layer as u32) as usize;
            let max_slots = ((layout.layer_stride as usize) / slot_stride.max(1)).max(1) as u32;
            let layer_base = kv_source_layer as usize * layout.layer_stride as usize;
            let start_tok = if is_swa && h.sliding_window > 0 {
                (token_idx + 1).saturating_sub(h.sliding_window)
            } else {
                0
            };
            let gqa = n_head / n_kv_layer;
            for qh in 0..n_head {
                let kh = qh / gqa;
                let q_head = &q_buf[qh * head_dim_layer..(qh + 1) * head_dim_layer];
                let out_head = &mut attn_out[qh * head_dim_layer..(qh + 1) * head_dim_layer];
                let mut max_score = f32::NEG_INFINITY;
                let mut sum_exp = 0.0f32;
                for tok in start_tok..=token_idx {
                    if token_idx - tok >= max_slots {
                        continue;
                    }
                    let t_slot = (tok % max_slots) as usize;
                    let t_base = layer_base + t_slot * slot_stride;
                    let k_head = &kv
                        [t_base + kh * head_dim_layer..t_base + (kh + 1) * head_dim_layer];
                    let v_head = &kv[t_base + kv_dim_layer + kh * head_dim_layer
                        ..t_base + kv_dim_layer + (kh + 1) * head_dim_layer];
                    let mut score = 0.0f32;
                    for d in 0..head_dim_layer {
                        score += q_head[d] * k_head[d];
                    }
                    if score > max_score {
                        let rescale = (max_score - score).exp();
                        for v in out_head.iter_mut() {
                            *v *= rescale;
                        }
                        sum_exp = sum_exp * rescale + 1.0f32;
                        max_score = score;
                        for d in 0..head_dim_layer {
                            out_head[d] += v_head[d];
                        }
                    } else {
                        let w = (score - max_score).exp();
                        sum_exp += w;
                        for d in 0..head_dim_layer {
                            out_head[d] += w * v_head[d];
                        }
                    }
                }
                if sum_exp > 0.0 {
                    let inv = 1.0f32 / sum_exp;
                    for v in out_head.iter_mut() {
                        *v *= inv;
                    }
                }
            }
        }

        let Some(out_info) = tensors.attn_output.as_ref() else {
            return false;
        };
        let (o_in, o_out) = Self::matmul_dims(out_info);
        if o_in > q_out
            || o_out != n_embd
            || !self.dispatch_gemm_into(
                index,
                out_info,
                &attn_out[..o_in],
                &mut scratch_a[..n_embd],
                o_in,
                n_embd,
            )
        {
            return false;
        }
        if let Some(pan_info) = named_layer_tensor(index, layer, b"post_attention_norm.weight") {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &pan_info, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut scratch_a[..n_embd], &norm_w[..n_embd], eps);
        }
        add_residual_inplace(&mut hidden[..n_embd], &scratch_a[..n_embd], n_embd);

        // === 2. FFN Sub-Block (GEGLU dense + optional parallel MoE) ===
        h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
        if let Some(fn_info) = tensors.ffn_norm.as_ref() {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, fn_info, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], eps);
        }

        let (Some(gate_info), Some(up_info), Some(down_info)) = (
            tensors.ffn_gate.as_ref(),
            tensors.ffn_up.as_ref(),
            tensors.ffn_down.as_ref(),
        ) else {
            return false;
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in != n_embd
            || up_in != n_embd
            || up_out != n_ffn
            || dn_in != n_ffn
            || dn_out != n_embd
            || n_ffn > MAX_STACK_GEMM_DIM
        {
            return false;
        }
        let mut gate_buf = [0.0f32; MAX_STACK_GEMM_DIM];
        let mut up_buf = [0.0f32; MAX_STACK_GEMM_DIM];
        if !self.dispatch_gemm_into(
            index,
            gate_info,
            &h_norm[..n_embd],
            &mut gate_buf[..n_ffn],
            n_embd,
            n_ffn,
        ) || !self.dispatch_gemm_into(
            index,
            up_info,
            &h_norm[..n_embd],
            &mut up_buf[..n_ffn],
            n_embd,
            n_ffn,
        ) {
            return false;
        }
        for i in 0..n_ffn {
            gate_buf[i] = gelu_pytorch_tanh(gate_buf[i]) * up_buf[i];
        }
        if !self.dispatch_gemm_into(
            index,
            down_info,
            &gate_buf[..n_ffn],
            &mut scratch_b[..n_embd],
            n_ffn,
            n_embd,
        ) {
            return false;
        }

        if let Some(router_info) = tensors.moe_router.as_ref() {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            if let Some(pfn1) = named_layer_tensor(index, layer, b"post_ffw_norm_1.weight") {
                let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
                if dequant_norm_row_into(mmap, index.tensor_data_start, &pfn1, &mut norm_w[..n_embd])
                    < n_embd
                {
                    return false;
                }
                rms_norm_inplace(&mut scratch_b[..n_embd], &norm_w[..n_embd], eps);
            }
            let mut h_router = [0.0f32; MAX_HIDDEN_DIM];
            h_router[..n_embd].copy_from_slice(&hidden[..n_embd]);
            unit_rms_norm_inplace(&mut h_router[..n_embd], eps);
            if let Some(shexp_info) = tensors.moe_shared_gate_input.as_ref() {
                let mut shexp_w = [0.0f32; MAX_HIDDEN_DIM];
                if dequant_norm_row_into(
                    mmap,
                    index.tensor_data_start,
                    shexp_info,
                    &mut shexp_w[..n_embd],
                ) < n_embd
                {
                    return false;
                }
                let inv_sqrt = 1.0f32 / (n_embd as f32).sqrt();
                for i in 0..n_embd {
                    h_router[i] *= shexp_w[i] * inv_sqrt;
                }
            }
            let mut h_moe = [0.0f32; MAX_HIDDEN_DIM];
            h_moe[..n_embd].copy_from_slice(&hidden[..n_embd]);
            if let Some(pre2) = named_layer_tensor(index, layer, b"pre_ffw_norm_2.weight") {
                let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
                if dequant_norm_row_into(mmap, index.tensor_data_start, &pre2, &mut norm_w[..n_embd])
                    < n_embd
                {
                    return false;
                }
                rms_norm_inplace(&mut h_moe[..n_embd], &norm_w[..n_embd], eps);
            }
            let actual_experts = router_info.dims[1] as usize;
            let Ok(routing) = index.moe_routing.admit(actual_experts) else {
                return false;
            };
            let num_experts = routing.expert_count().min(256);
            let topk = routing.experts_per_token().clamp(1, 16);
            if num_experts == 0 {
                return false;
            }
            let Ok(router_raw) = crate::ggml_quants::fetch_tensor_bytes(
                mmap,
                index.tensor_data_start,
                router_info,
            ) else {
                return false;
            };
            let mut gate_logits = [0.0f32; 256];
            if !etalon_inference::ordinary::experts::compute_router_logits_from_gguf(
                router_raw,
                router_info,
                &h_router[..n_embd],
                num_experts,
                &mut gate_logits[..num_experts],
            ) {
                return false;
            }
            let mut expert_indices = [0u16; 16];
            let mut expert_weights = [0.0f32; 16];
            let Ok(selected) = crate::inference::moe::routing::route_topk(
                &gate_logits[..num_experts],
                topk,
                true,
                &mut expert_indices,
                &mut expert_weights,
            ) else {
                return false;
            };
            let (Some(gate_up), Some(down_exps)) = (
                tensors.moe_gate_up_exps.as_ref(),
                tensors.moe_down_exps.as_ref(),
            ) else {
                return false;
            };
            let fused_rows = gate_up.dims[1] as usize;
            let inter_dim = fused_rows / 2;
            if fused_rows == 0
                || fused_rows % 2 != 0
                || inter_dim > 4096
                || gate_up.dims[0] as usize != n_embd
                || down_exps.dims[0] as usize != inter_dim
                || down_exps.dims[1] as usize != n_embd
            {
                return false;
            }
            let (Ok(gu_raw), Ok(dn_raw)) = (
                crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_up),
                crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_exps),
            ) else {
                return false;
            };
            let mut down_exps_scale = [1.0f32; 256];
            if let Some(scale_info) = named_layer_tensor(index, layer, b"ffn_down_exps.scale") {
                let scale_len = (scale_info.dims[0] as usize).min(num_experts);
                if scale_len > 0 {
                    let _ = dequant_norm_row_into(
                        mmap,
                        index.tensor_data_start,
                        &scale_info,
                        &mut down_exps_scale[..scale_len],
                    );
                }
            }
            scratch_a[..n_embd].fill(0.0);
            let mut exp_outs = [[0.0f32; MAX_HIDDEN_DIM]; 16];
            let mut exp_ok = [true; 16];
            for sel in 0..selected {
                let out_buf = &mut exp_outs[sel];
                let ok_flag = &mut exp_ok[sel];
                let exp = expert_indices[sel] as usize;
                let w = expert_weights[sel] * down_exps_scale[exp.min(255)];
                let exp_gu_base = exp * fused_rows;
                let mut row = [0.0f32; MAX_HIDDEN_DIM];
                let mut geglu = [0.0f32; 4096];
                let mut expert_ok = true;
                for r in 0..inter_dim {
                    if crate::ggml_quants::dequant_matrix_row_into(
                        gu_raw,
                        gate_up,
                        exp_gu_base + r,
                        &mut row[..n_embd],
                    )
                    .is_err()
                    {
                        expert_ok = false;
                        break;
                    }
                    let g = etalon_inference::ordinary::experts::dot(
                        &h_moe[..n_embd],
                        &row[..n_embd],
                    );
                    if crate::ggml_quants::dequant_matrix_row_into(
                        gu_raw,
                        gate_up,
                        exp_gu_base + inter_dim + r,
                        &mut row[..n_embd],
                    )
                    .is_err()
                    {
                        expert_ok = false;
                        break;
                    }
                    let u = etalon_inference::ordinary::experts::dot(
                        &h_moe[..n_embd],
                        &row[..n_embd],
                    );
                    geglu[r] = gelu_pytorch_tanh(g) * u;
                }
                if expert_ok {
                    let exp_dn_base = exp * n_embd;
                    for r in 0..n_embd {
                        if crate::ggml_quants::dequant_matrix_row_into(
                            dn_raw,
                            down_exps,
                            exp_dn_base + r,
                            &mut row[..inter_dim],
                        )
                        .is_err()
                        {
                            expert_ok = false;
                            break;
                        }
                        out_buf[r] = w
                            * etalon_inference::ordinary::experts::dot(
                                &geglu[..inter_dim],
                                &row[..inter_dim],
                            );
                    }
                }
                *ok_flag = expert_ok;
            }
            if exp_ok[..selected].iter().any(|&ok| !ok) {
                return false;
            }
            for sel in 0..selected {
                add_residual_inplace(
                    &mut scratch_a[..n_embd],
                    &exp_outs[sel][..n_embd],
                    n_embd,
                );
            }
            if let Some(pfn2) = named_layer_tensor(index, layer, b"post_ffw_norm_2.weight") {
                let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
                if dequant_norm_row_into(mmap, index.tensor_data_start, &pfn2, &mut norm_w[..n_embd])
                    < n_embd
                {
                    return false;
                }
                rms_norm_inplace(&mut scratch_a[..n_embd], &norm_w[..n_embd], eps);
            }
            add_residual_inplace(&mut scratch_b[..n_embd], &scratch_a[..n_embd], n_embd);
        }

        if let Some(pfn_info) = named_layer_tensor(index, layer, b"post_ffw_norm.weight") {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &pfn_info, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut scratch_b[..n_embd], &norm_w[..n_embd], eps);
        }
        add_residual_inplace(&mut hidden[..n_embd], &scratch_b[..n_embd], n_embd);

        // === 3. Per-Layer Embedding (PLE) Sub-Block ===
        if let (Some(inp_gate_info), Some(proj_info), Some(post_norm_info)) = (
            named_layer_tensor(index, layer, b"inp_gate.weight"),
            named_layer_tensor(index, layer, b"proj.weight"),
            named_layer_tensor(index, layer, b"post_norm.weight"),
        ) {
            let (ig_in, ple_dim) = Self::matmul_dims(&inp_gate_info);
            let (pr_in, pr_out) = Self::matmul_dims(&proj_info);
            if ig_in != n_embd
                || ple_dim == 0
                || ple_dim > 512
                || pr_in != ple_dim
                || pr_out != n_embd
                || (layer as usize + 1) * ple_dim > MAX_G4_PLE_TOTAL
            {
                return false;
            }
            let mut ple_buf = [0.0f32; 512];
            if !self.dispatch_gemm_into(
                index,
                &inp_gate_info,
                &hidden[..n_embd],
                &mut ple_buf[..ple_dim],
                n_embd,
                ple_dim,
            ) {
                return false;
            }
            GEMMA4_PLE_RING.with(|cell| {
                let ring = cell.borrow();
                let s = layer as usize * ple_dim;
                let tok_ple = &ring[slot_ring][s..s + ple_dim];
                for i in 0..ple_dim {
                    ple_buf[i] = gelu_pytorch_tanh(ple_buf[i]) * tok_ple[i];
                }
            });
            if !self.dispatch_gemm_into(
                index,
                &proj_info,
                &ple_buf[..ple_dim],
                &mut scratch_a[..n_embd],
                ple_dim,
                n_embd,
            ) {
                return false;
            }
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                &post_norm_info,
                &mut norm_w[..n_embd],
            ) < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut scratch_a[..n_embd], &norm_w[..n_embd], eps);
            add_residual_inplace(&mut hidden[..n_embd], &scratch_a[..n_embd], n_embd);
        }

        // === 4. Layer Output Scale (`layer_output_scale.weight`) ===
        if let Some(scale_info) = named_layer_tensor(index, layer, b"layer_output_scale.weight") {
            let Some(mmap) = self.model_bytes() else {
                return false;
            };
            let mut scale = [1.0f32; 1];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &scale_info, &mut scale) == 1 {
                let s = scale[0];
                for v in &mut hidden[..n_embd] {
                    *v *= s;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gguf_sharder::GgufTokenizer;
    use std::path::Path;
    use std::time::Instant;

    fn run_gguf_smoke_decode(path: &str, prompt: &str, gen_tokens: usize) {
        if !Path::new(path).exists() {
            eprintln!("Skipping {path} (not present)");
            return;
        }
        let t0 = Instant::now();
        let mut engine = QTensorEngine::new();
        let report = engine
            .load_gguf_checked(path)
            .unwrap_or_else(|e| panic!("load_gguf_checked({path}) failed: {e}"));
        let load_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let mmap = engine.model_bytes().expect("mmap bytes").to_vec();
        let index = engine
            .tensor_index_cache
            .clone()
            .expect("tensor_index_cache");
        let tokenizer = GgufTokenizer::from_gguf(&mmap);
        let mut tokens = tokenizer.encode_prompt(prompt);
        if tokens.is_empty() {
            tokens.push(tokenizer.bos_token_id);
        }
        let emb_dim = index.emb_dim();
        assert!(emb_dim > 0 && emb_dim <= 4096);
        eprintln!(
            "[smoke] Loaded {} in {:.1} ms | layers={} emb_dim={} arch={} kv_mib={:.1} prompt_tokens={:?}",
            path,
            load_ms,
            report.n_layer,
            emb_dim,
            index.hyperparams.architecture,
            report.kv_cache_bytes as f64 / (1024.0 * 1024.0),
            tokens
        );

        let mut hidden = vec![0.0f32; emb_dim];
        let mut scratch_a = vec![0.0f32; MAX_STACK_GEMM_OUT];
        let mut scratch_b = vec![0.0f32; MAX_STACK_GEMM_OUT];
        let mut chunk_logits = vec![0.0f32; VOCAB_CHUNK_ROWS];

        let t_prefill = Instant::now();
        for (pos, &tok) in tokens.iter().enumerate() {
            let n = index.dequantize_token_embedding_into(&mmap, tok, &mut hidden);
            assert_eq!(n, emb_dim, "embedding dequant failed at pos={pos}");
            let ran = engine.dispatch_transformer_forward(
                &index,
                &mut hidden,
                emb_dim,
                &mut scratch_a,
                &mut scratch_b,
                pos as u32,
                0,
            );
            assert_eq!(
                ran, report.n_layer,
                "expected all {} layers to execute at pos={pos}, got {ran}",
                report.n_layer
            );
            assert!(
                hidden.iter().all(|v| v.is_finite()),
                "non-finite hidden after prefill token pos={pos}"
            );
        }
        let prefill_s = t_prefill.elapsed().as_secs_f64();

        assert!(engine.apply_output_norm_inplace(&index, &mut hidden, emb_dim));
        let first = engine
            .dispatch_output_argmax_chunked(&index, &hidden, emb_dim, &mut chunk_logits, 0, None)
            .expect("argmax token 0");
        assert!(first.max_logit.is_finite());

        let mut generated = vec![first.best_token_id];
        let t_decode = Instant::now();
        for step in 1..gen_tokens {
            let pos = (tokens.len() - 1 + step) as u32;
            let prev = *generated.last().unwrap();
            let n = index.dequantize_token_embedding_into(&mmap, prev, &mut hidden);
            assert_eq!(n, emb_dim);
            let ran = engine.dispatch_transformer_forward(
                &index,
                &mut hidden,
                emb_dim,
                &mut scratch_a,
                &mut scratch_b,
                pos,
                0,
            );
            assert_eq!(ran, report.n_layer);
            assert!(engine.apply_output_norm_inplace(&index, &mut hidden, emb_dim));
            let next = engine
                .dispatch_output_argmax_chunked(
                    &index,
                    &hidden,
                    emb_dim,
                    &mut chunk_logits,
                    0,
                    None,
                )
                .expect("argmax decode step");
            assert!(next.max_logit.is_finite());
            generated.push(next.best_token_id);
        }
        let decode_s = t_decode.elapsed().as_secs_f64();
        let decoded_text = tokenizer.decode(&generated);
        eprintln!(
            "[smoke] {} | prefill {:.2} tok/s ({} toks in {:.2}s) | decode {:.2} tok/s ({} toks in {:.2}s) | ids={:?} | text={:?}",
            path,
            tokens.len() as f64 / prefill_s.max(1e-9),
            tokens.len(),
            prefill_s,
            (gen_tokens.saturating_sub(1)) as f64 / decode_s.max(1e-9),
            gen_tokens.saturating_sub(1),
            decode_s,
            generated,
            decoded_text
        );
    }

    #[test]
    fn real_qwen35_0_8b_gguf_end_to_end_decode() {
        let builder = std::thread::Builder::new()
            .name("qwen35-0.8b-test".into())
            .stack_size(32 * 1024 * 1024);
        let handle = builder
            .spawn(|| {
                run_gguf_smoke_decode(
                    r"C:\LLM_files\qwen\Qwen3.5-0.8B-GGUF\Qwen3.5-0.8B-Q4_K_M.gguf",
                    "The capital of France is",
                    8,
                );
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn real_qwen35_4b_gguf_end_to_end_decode() {
        let builder = std::thread::Builder::new()
            .name("qwen35-4b-test".into())
            .stack_size(32 * 1024 * 1024);
        let handle = builder
            .spawn(|| {
                run_gguf_smoke_decode(
                    r"C:\LLM_files\qwen\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf",
                    "The capital of France is",
                    6,
                );
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn real_gemma4_e4b_gguf_end_to_end_decode() {
        let builder = std::thread::Builder::new()
            .name("gemma4-e4b-test".into())
            .stack_size(32 * 1024 * 1024);
        let handle = builder
            .spawn(|| {
                run_gguf_smoke_decode(
                    r"C:\LLM_files\gemma\gemma-4-E4B-it-GGUF\gemma-4-E4B-it-Q4_K_M.gguf",
                    "The capital of France is",
                    6,
                );
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn real_gemma4_26b_a4b_gguf_end_to_end_decode() {
        let builder = std::thread::Builder::new()
            .name("gemma4-26b-a4b-test".into())
            .stack_size(32 * 1024 * 1024);
        let handle = builder
            .spawn(|| {
                run_gguf_smoke_decode(
                    r"C:\LLM_files\gemma\gemma-4-26B-A4B-it-GGUF\gemma-4-26B-A4B-it-MXFP4_MOE.gguf",
                    "The capital of France is",
                    6,
                );
            })
            .unwrap();
        handle.join().unwrap();
    }
}
