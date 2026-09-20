//! Per-step hybrid SSM forward pass (Granite Hybrid / Qwen GatedDeltaNet).
//!
//! This module wires the recurrent-state primitives in
//! `crates/qualia-core-db/src/inference/qwen/gated_deltanet.rs` into the
//! `QTensorEngine` decode loop.  Each hybrid layer either executes a
//! GatedDeltaNet recurrence (Qwen) or a Mamba-style causal-conv +
//! recurrent projection (Granite) instead of scaled-dot-product attention.
//!
//! Memory contract (zero-heap hot path):
//! The SSM recurrent state is a single heap allocation made once at model-
//! load time (`QTensorEngine::init_ssm_state`).  During the hot decode loop
//! the state is accessed as a mutable `&mut [f32]` slice -- no Vec::push, no
//! per-token allocation.  This keeps the hot path in Tier 1 (zero-heap) while
//! the one-shot construction is Tier 2 (cold bounded) per the AGENTS.md
//! two-tier contract.

use super::{
    cpu_ops::{add_residual_inplace, dequant_norm_row_into, rms_norm_inplace},
    RMS_NORM_EPS,
};
use crate::{
    gguf_sharder::{GgufTensorIndex, LayerTensors},
    inference::qwen::gated_deltanet::{step_causal_conv1d, step_gated_deltanet},
};

/// Maximum supported SSM state width (d_state).
pub(crate) const MAX_SSM_STATE: usize = 256;
/// Maximum supported SSM head dimension (d_head).
pub(crate) const MAX_SSM_HEAD: usize = 256;
/// Maximum supported SSM inner (expanded) dimension.
pub(crate) const MAX_SSM_INNER: usize = 4096;
/// Causal convolution history depth (CAUSAL_CONV_KERNEL - 1 = 3).
pub(crate) const CONV_HISTORY_DEPTH: usize = 3;

/// Per-layer recurrent state layout stored contiguously in ssm_recurrent_state:
/// [conv_history: CONV_HISTORY_DEPTH * inner_dim f32]
/// [delta_state : d_state * d_head * n_heads f32   ]
#[derive(Clone, Copy, Debug)]
pub(crate) struct SsmLayerStateLayout {
    /// Layer stride in f32 elements.
    pub(crate) stride: usize,
    /// Offset of conv history region within the stride (always 0).
    #[allow(dead_code)]
    pub(crate) conv_offset: usize,
    /// Length of conv history region.
    pub(crate) conv_len: usize,
    /// Offset of GatedDeltaNet state matrix.
    pub(crate) delta_offset: usize,
    /// Length of GatedDeltaNet state matrix.
    pub(crate) delta_len: usize,
}

impl SsmLayerStateLayout {
    pub(crate) fn new(inner_dim: usize, d_state: usize, d_head: usize, n_heads: usize) -> Self {
        let conv_len = CONV_HISTORY_DEPTH * inner_dim;
        let delta_len = d_state * d_head * n_heads;
        let delta_offset = conv_len;
        let stride = conv_len + delta_len;
        Self { stride, conv_offset: 0, conv_len, delta_offset, delta_len }
    }
}

/// Derive the SsmLayerStateLayout from a layer's tensor metadata.
pub(crate) fn ssm_layout_for(
    tensors: &LayerTensors,
    h: &crate::gguf_sharder::GgufHyperparams,
    n_embd: usize,
) -> SsmLayerStateLayout {
    let inner_dim = if h.ssm_inner_size > 0 {
        h.ssm_inner_size as usize
    } else if let Some(info) = tensors.ssm_in.as_ref().or(tensors.attn_qkv.as_ref()) {
        let rows = info.dims[0] as usize;
        if tensors.attn_qkv.is_some() { rows / 3 } else { rows }
    } else {
        n_embd * 2
    };
    let inner_dim = inner_dim.min(MAX_SSM_INNER).max(1);

    let d_state = if h.ssm_state_size > 0 { h.ssm_state_size as usize } else { MAX_SSM_STATE / 2 }
        .min(MAX_SSM_STATE).max(1);

    let d_head = if h.head_dim() > 0 { h.head_dim() as usize } else { d_state }
        .min(MAX_SSM_HEAD).max(1);

    let n_heads = if h.ssm_group_count > 0 {
        h.ssm_group_count as usize
    } else {
        let alpha_dim = tensors.ssm_alpha.map(|i| i.dims[0] as usize).unwrap_or(0);
        if d_head > 0 && alpha_dim > 0 { alpha_dim / d_head } else { 1 }
    }.max(1);

    SsmLayerStateLayout::new(inner_dim, d_state, d_head, n_heads)
}

impl super::QTensorEngine {
    /// Allocate (once) the flat recurrent-state arena for all hybrid SSM layers.
    ///
    /// Called after the GGUF index is parsed. Returns the number of f32 elements
    /// allocated (zero if the model has no hybrid SSM layers).
    pub fn init_ssm_state(&mut self, index: &GgufTensorIndex) -> usize {
        let h = &index.hyperparams;
        let n_layer = h.n_layer as usize;
        let n_embd = h.n_embd as usize;
        let mut total_elems = 0usize;
        let mut any_ssm = false;
        for layer in 0..n_layer as u32 {
            let tensors = index.get_layer_tensors(layer);
            if tensors.is_hybrid_ssm_layer() {
                any_ssm = true;
                let layout = ssm_layout_for(&tensors, h, n_embd);
                total_elems += layout.stride;
            }
        }
        if !any_ssm || total_elems == 0 {
            return 0;
        }
        // Sentinel budget check (8 MB).
        const MAX_SSM_BYTES: usize = 8 * 1024 * 1024;
        let byte_budget = total_elems * 4;
        if byte_budget > MAX_SSM_BYTES {
            super::wlog(&format!(
                "[ssm_init] SSM state too large: {byte_budget} bytes > {MAX_SSM_BYTES} ceiling -- SSM layers skipped"
            ));
            return 0;
        }
        // Tier-2 cold allocation: single Box<[f32]> created once at boot.
        let state = vec![0.0f32; total_elems].into_boxed_slice();
        self.ssm_recurrent_state = state;
        super::wlog(&format!(
            "[ssm_init] allocated SSM state: {total_elems} f32 ({} kB) for {n_layer} layers",
            byte_budget / 1024,
        ));
        total_elems
    }

    /// Execute one decode step for a hybrid SSM layer.
    pub(crate) fn dispatch_hybrid_ssm_layer(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if self.ssm_recurrent_state.is_empty() {
            super::wlog(&format!("[ssm] layer={layer} skipped: state not allocated"));
            return false;
        }
        let state_offset = self.ssm_state_offset_for_layer(index, layer);
        if tensors.ssm_in.is_some() {
            self.dispatch_granite_ssm_layer(index, layer, hidden, emb_dim, tensors, scratch_a, scratch_b, state_offset)
        } else if tensors.attn_qkv.is_some() && tensors.ssm_alpha.is_some() {
            self.dispatch_qwen_ssm_layer(index, layer, hidden, emb_dim, tensors, scratch_a, scratch_b, state_offset)
        } else {
            super::wlog(&format!("[ssm] layer={layer} unrecognised SSM flavour"));
            false
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_granite_ssm_layer(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        state_offset: usize,
    ) -> bool {
        if self.gguf_mmap.is_none() {
            super::wlog(&format!("[granite_ssm] layer={layer} mmap None"));
            return false;
        }
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let tds = index.tensor_data_start;

        let ssm_in_info = match tensors.ssm_in.as_ref() { Some(i) => *i, None => return false };
        let ssm_conv1d_info = match tensors.ssm_conv1d.as_ref() { Some(i) => *i, None => return false };
        let ssm_out_info = match tensors.ssm_out.as_ref() { Some(i) => *i, None => return false };

        let (inner_dim, proj_in) = Self::matmul_dims(&ssm_in_info);
        if proj_in == 0 || inner_dim == 0 || inner_dim > MAX_SSM_INNER { return false; }

        // 1. Pre-norm: inline mmap access; borrow released after block.
        let ssm_input_is_norm = {
            if let Some(ni) = tensors.attn_norm.as_ref() {
                let mmap = self.gguf_mmap.as_deref().unwrap();
                let mut norm_w = [0f32; MAX_SSM_INNER];
                let n = dequant_norm_row_into(mmap, tds, ni, &mut norm_w);
                if n >= n_embd {
                    scratch_b[..n_embd].copy_from_slice(&hidden[..n_embd]);
                    rms_norm_inplace(&mut scratch_b[..n_embd], &norm_w[..n_embd], RMS_NORM_EPS);
                    true
                } else { false }
            } else { false }
        };
        // scratch_b[..n_embd] holds the normalised input if ssm_input_is_norm.

        // 2. Input projection: scratch_b → scratch_a via ssm_in weight.
        //    dispatch_gemm_into takes &self — only works if no &mut self field is live.
        if ssm_input_is_norm {
            let snap_in: [f32; MAX_SSM_INNER] = {
                let mut buf = [0f32; MAX_SSM_INNER];
                buf[..n_embd].copy_from_slice(&scratch_b[..n_embd]);
                buf
            };
            if !self.dispatch_gemm_into(index, &ssm_in_info, &snap_in[..proj_in], scratch_a, proj_in, inner_dim) {
                super::wlog(&format!("[granite_ssm] layer={layer} ssm_in GEMM fail"));
                return false;
            }
        } else {
            let snap_in: [f32; MAX_SSM_INNER] = {
                let mut buf = [0f32; MAX_SSM_INNER];
                buf[..n_embd.min(proj_in)].copy_from_slice(&hidden[..n_embd.min(proj_in)]);
                buf
            };
            if !self.dispatch_gemm_into(index, &ssm_in_info, &snap_in[..proj_in], scratch_a, proj_in, inner_dim) {
                super::wlog(&format!("[granite_ssm] layer={layer} ssm_in GEMM fail"));
                return false;
            }
        }
        // scratch_a[..inner_dim] = projected input x

        // 3. Causal conv1d on projected input.
        //    mmap borrow ends after its last use (dequantize_row_into) — NLL allows
        //    ssm_recurrent_state mutable borrow to start in the same block afterwards.
        {
            let layout = SsmLayerStateLayout::new(inner_dim, MAX_SSM_STATE, MAX_SSM_HEAD, 1);
            let avail = self.ssm_recurrent_state.len().saturating_sub(state_offset);
            if avail < layout.stride { return false; }
            let mut conv_w = [0f32; MAX_SSM_INNER * 4];
            {
                let mmap = self.gguf_mmap.as_deref().unwrap();
                let conv_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, tds, &ssm_conv1d_info) {
                    Ok(s) => s,
                    Err(_) => { super::wlog(&format!("[granite_ssm] layer={layer} conv fetch fail")); return false; }
                };
                let _ = crate::ggml_quants::dequantize_row_into(
                    conv_raw, ssm_conv1d_info.ggml_type, inner_dim * 4, &mut conv_w[..inner_dim * 4],
                );
            } // mmap and conv_raw dropped here
            let mut conv_inp = [0f32; MAX_SSM_INNER];
            conv_inp[..inner_dim].copy_from_slice(&scratch_a[..inner_dim]);
            let layer_state = &mut self.ssm_recurrent_state[state_offset..state_offset + layout.stride];
            let (conv_region, _) = layer_state.split_at_mut(layout.conv_len);
            let _ = step_causal_conv1d(conv_region, &conv_w[..inner_dim * 4], &conv_inp[..inner_dim], &mut scratch_b[..inner_dim]);
        }

        // 4. dt_bias softplus gate.
        if let Some(dt_info) = tensors.ssm_dt_bias.as_ref() {
            let mmap = self.gguf_mmap.as_deref().unwrap();
            let mut dt_w = [0f32; MAX_SSM_INNER];
            let dt_n = dequant_norm_row_into(mmap, tds, dt_info, &mut dt_w);
            for i in 0..dt_n.min(inner_dim) {
                let v = scratch_b[i] + dt_w[i];
                scratch_b[i] = if v >= 20.0 { v } else { (1.0f32 + v.exp()).ln() };
            }
        }

        // 5. SSM norm.
        if let Some(norm_info) = tensors.ssm_norm.as_ref() {
            let mmap = self.gguf_mmap.as_deref().unwrap();
            let mut ssm_norm_w = [0f32; MAX_SSM_INNER];
            let n = dequant_norm_row_into(mmap, tds, norm_info, &mut ssm_norm_w);
            if n >= inner_dim {
                rms_norm_inplace(&mut scratch_b[..inner_dim], &ssm_norm_w[..inner_dim], RMS_NORM_EPS);
            }
        }

        // 6. Output projection (snap scratch_b to avoid conflict with dispatch_gemm_into).
        let (out_in, out_out) = Self::matmul_dims(&ssm_out_info);
        if out_in > inner_dim || out_out == 0 { return false; }
        let snap_b: [f32; MAX_SSM_INNER] = {
            let mut buf = [0f32; MAX_SSM_INNER];
            buf[..out_in].copy_from_slice(&scratch_b[..out_in]);
            buf
        };
        if !self.dispatch_gemm_into(index, &ssm_out_info, &snap_b[..out_in], scratch_a, out_in, out_out) {
            return false;
        }

        // 7. Residual add.
        add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..out_out], emb_dim.min(out_out));
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_qwen_ssm_layer(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        state_offset: usize,
    ) -> bool {
        if self.gguf_mmap.is_none() {
            super::wlog(&format!("[qwen_ssm] layer={layer} mmap None"));
            return false;
        }
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let tds = index.tensor_data_start;

        let qkv_info = match tensors.attn_qkv.as_ref() { Some(i) => *i, None => return false };
        let alpha_info = match tensors.ssm_alpha.as_ref() { Some(i) => *i, None => return false };
        let beta_info = match tensors.ssm_beta.as_ref() { Some(i) => *i, None => return false };
        let conv_info = match tensors.ssm_conv1d.as_ref() { Some(i) => *i, None => return false };
        let out_info = match tensors.ssm_out.as_ref() { Some(i) => *i, None => return false };

        let alpha_dim = alpha_info.dims[0] as usize;
        let d_head_ssm = if h.head_dim() > 0 { h.head_dim() as usize } else { MAX_SSM_HEAD };
        let n_heads_ssm = (if d_head_ssm > 0 { alpha_dim / d_head_ssm } else { 1 }).max(1);

        let (qkv_rows, qkv_cols) = Self::matmul_dims(&qkv_info);
        let inner_dim = qkv_rows / 3;
        if inner_dim == 0 || inner_dim > MAX_SSM_INNER { return false; }
        if inner_dim * 3 > scratch_a.len() { return false; }

        // 1. Pre-norm: snap normalised input into a stack buffer.
        let qkv_input_snap: [f32; MAX_SSM_INNER] = {
            let mut buf = [0f32; MAX_SSM_INNER];
            if let Some(ni) = tensors.attn_norm.as_ref() {
                let mmap = self.gguf_mmap.as_deref().unwrap();
                let mut norm_w = [0f32; MAX_SSM_INNER];
                let n = dequant_norm_row_into(mmap, tds, ni, &mut norm_w);
                if n >= n_embd {
                    buf[..n_embd].copy_from_slice(&hidden[..n_embd]);
                    rms_norm_inplace(&mut buf[..n_embd], &norm_w[..n_embd], RMS_NORM_EPS);
                } else {
                    buf[..n_embd].copy_from_slice(&hidden[..n_embd]);
                }
            } else {
                buf[..n_embd].copy_from_slice(&hidden[..n_embd]);
            }
            buf
        };

        // 2. Fused QKV projection → scratch_a[0..3*inner_dim].
        if !self.dispatch_gemm_into(index, &qkv_info, &qkv_input_snap[..qkv_cols.min(n_embd)], scratch_a, qkv_cols, qkv_rows) {
            super::wlog(&format!("[qwen_ssm] layer={layer} QKV GEMM fail"));
            return false;
        }

        // 3. Causal conv on k; decode conv weights + snap k before mutable state borrow.
        let mut k_snap = [0f32; MAX_SSM_INNER];
        let mut conv_w = [0f32; MAX_SSM_INNER * 4];
        {
            let mmap = self.gguf_mmap.as_deref().unwrap();
            let conv_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, tds, &conv_info) {
                Ok(s) => s,
                Err(_) => { return false; }
            };
            let _ = crate::ggml_quants::dequantize_row_into(
                conv_raw, conv_info.ggml_type, inner_dim * 4, &mut conv_w[..inner_dim * 4],
            );
            k_snap[..inner_dim].copy_from_slice(&scratch_a[inner_dim..2 * inner_dim]);
        } // mmap borrow released
        {
            let layout = SsmLayerStateLayout::new(inner_dim, d_head_ssm, d_head_ssm, n_heads_ssm);
            let avail = self.ssm_recurrent_state.len().saturating_sub(state_offset);
            if avail < layout.stride { return false; }
            let layer_state = &mut self.ssm_recurrent_state[state_offset..state_offset + layout.stride];
            let (conv_region, _) = layer_state.split_at_mut(layout.conv_len);
            let _ = step_causal_conv1d(conv_region, &conv_w[..inner_dim * 4], &k_snap[..inner_dim], &mut scratch_b[..inner_dim]);
        }

        // 4. Alpha / beta gates.
        let mut alpha_buf = [1.0f32; MAX_SSM_HEAD];
        let mut beta_buf = [1.0f32; MAX_SSM_HEAD];
        {
            let mmap = self.gguf_mmap.as_deref().unwrap();
            let _ = dequant_norm_row_into(mmap, tds, &alpha_info, &mut alpha_buf[..d_head_ssm.min(MAX_SSM_HEAD)]);
            let _ = dequant_norm_row_into(mmap, tds, &beta_info, &mut beta_buf[..d_head_ssm.min(MAX_SSM_HEAD)]);
        }

        // 5. GatedDeltaNet recurrence: scoped state borrow.
        let mut ssm_out_buf = [0f32; MAX_SSM_INNER];
        let ssm_len = n_heads_ssm * d_head_ssm;
        {
            let layout2 = SsmLayerStateLayout::new(inner_dim, d_head_ssm, d_head_ssm, n_heads_ssm);
            let layer_state2 = &mut self.ssm_recurrent_state[state_offset..state_offset + layout2.stride];
            let delta_state = &mut layer_state2[layout2.delta_offset..layout2.delta_offset + layout2.delta_len];
            let d_state = d_head_ssm;
            for head in 0..n_heads_ssm {
                let hs = head * d_head_ssm;
                let he = hs + d_head_ssm;
                let vs = 2 * inner_dim + hs;
                let ve = 2 * inner_dim + he;
                let ss = head * d_state * d_head_ssm;
                let se = ss + d_state * d_head_ssm;
                if ve > scratch_a.len() || se > delta_state.len() { break; }
                let _ = step_gated_deltanet(
                    &mut delta_state[ss..se],
                    &scratch_a[hs..he],
                    &scratch_b[hs..he],
                    &scratch_a[vs..ve],
                    &alpha_buf[..1],
                    &beta_buf[..1],
                    d_state,
                    d_head_ssm,
                    &mut ssm_out_buf[hs..he],
                );
            }
        }

        // 6. Sigmoid gate from q.
        for i in 0..ssm_len {
            let gate = 1.0f32 / (1.0 + (-scratch_a[i]).exp());
            ssm_out_buf[i] *= gate;
        }

        // 7. Output projection.
        let (out_in, out_out) = Self::matmul_dims(&out_info);
        if out_in > ssm_len || out_out == 0 { return false; }
        if !self.dispatch_gemm_into(index, &out_info, &ssm_out_buf[..out_in], scratch_a, out_in, out_out) {
            return false;
        }

        // 8. Residual add.
        add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..out_out], emb_dim.min(out_out));
        true
    }

    /// Sum strides of all preceding hybrid SSM layers to find the arena offset for 	arget_layer.
    fn ssm_state_offset_for_layer(&self, index: &GgufTensorIndex, target_layer: u32) -> usize {
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let mut offset = 0usize;
        for layer in 0..target_layer {
            let tensors = index.get_layer_tensors(layer);
            if tensors.is_hybrid_ssm_layer() {
                let layout = ssm_layout_for(&tensors, &h, n_embd);
                offset += layout.stride;
            }
        }
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gguf_sharder::{GgufHyperparams, LayerTensors};

    #[test]
    fn ssm_layout_new_granite_style() {
        let layout = SsmLayerStateLayout::new(4096, 128, 64, 1);
        assert_eq!(layout.conv_offset, 0);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 4096);
        assert_eq!(layout.delta_offset, layout.conv_len);
        assert_eq!(layout.delta_len, 128 * 64 * 1);
        assert_eq!(layout.stride, layout.conv_len + layout.delta_len);
    }

    #[test]
    fn ssm_layout_new_qwen_style() {
        let layout = SsmLayerStateLayout::new(2048, 128, 128, 16);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 2048);
        assert_eq!(layout.delta_len, 128 * 128 * 16);
        assert_eq!(layout.stride, layout.conv_len + layout.delta_len);
    }

    #[test]
    fn ssm_layout_for_uses_explicit_ssm_inner_size() {
        let mut h = GgufHyperparams::default();
        h.ssm_inner_size = 1024;
        h.ssm_state_size = 64;
        h.ssm_group_count = 4;
        h.n_embd = 512;
        let tensors = LayerTensors::default();
        let layout = ssm_layout_for(&tensors, &h, 512);
        assert_eq!(layout.conv_len, CONV_HISTORY_DEPTH * 1024);
        assert!(layout.delta_len > 0);
    }

    #[test]
    fn ssm_state_size_within_sentinel_budget() {
        // 96-layer Granite: stride per SSM layer = 3*4096 + 128*64 = 20480 f32
        let layout = SsmLayerStateLayout::new(4096, 128, 64, 1);
        let bytes = layout.stride * 96 * 4;
        // 96 * 20480 * 4 = 7,864,320 < 8 MB
        assert!(bytes < 8 * 1024 * 1024, "SSM state exceeds 8 MB sentinel: {bytes} bytes");
    }
}
