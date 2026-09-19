//! MoE FFN dispatch: routing, expert GEMV accumulation, and FTW package adoption.

use super::*;
use crate::inference::moe::dispatch::MAX_ROUTED_EXPERTS;
use crate::inference::moe::ftw_loader::FtwModelPackage;
use std::sync::Arc;

impl QTensorEngine {
    /// Adopt an FTW multi-shard model directory for native MoE execution.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_ftw_package(&mut self, dir: &std::path::Path) -> Result<GgufLoadReport, String> {
        let pkg = Arc::new(FtwModelPackage::open_from_dir(dir)?);
        let index = pkg.tensor_index.clone();
        self.tensor_index_cache = Some(index.clone());
        self.hyperparams = index.hyperparams;
        self.tensor_data_offset = index.tensor_data_start;
        self.max_tensor_bytes = index.max_layer_tensor_bytes;
        self.gguf_mmap = pkg.shards.first().cloned();
        self.ftw_package = Some(pkg);

        Ok(GgufLoadReport {
            mapped_bytes: self.ftw_package.as_ref().map(|p| p.manifest.total_bytes).unwrap_or(0),
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.n_kv_head,
            max_tensor_bytes: self.max_tensor_bytes,
            kv_cache_bytes: self.kv_cache_bytes(),
            directml_enabled: false,
        })
    }

    /// Dispatch MoE layer step: pre-norm input -> router -> top-k SwiGLU experts into scratch_a.
    pub(crate) fn dispatch_moe_ffn(
        &mut self,
        _index: &crate::gguf_sharder::GgufTensorIndex,
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        ffn_input: &[f32],
    ) -> bool {
        let router_info = match tensors.moe_router.as_ref() {
            Some(r) => r,
            None => return false,
        };

        let layer_idx = tensors.layer_idx as u16;
        let num_experts = (router_info.dims[1] as usize).min(MAX_ROUTED_EXPERTS).max(1);
        let topk = 8usize.min(num_experts);

        let mut gate_logits = [0.0f32; MAX_ROUTED_EXPERTS];

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref pkg) = self.ftw_package {
                let router_name = format!("model.layers.{}.mlp.gate.weight", layer_idx);
                let alt_router_name = format!("blk.{}.ffn_gate_inp.weight", layer_idx);
                let router_bytes = pkg.fetch_tensor_bytes(&router_name)
                    .or_else(|| pkg.fetch_tensor_bytes(&alt_router_name));

                if let Some(raw) = router_bytes {
                    compute_router_logits_from_raw(
                        raw,
                        router_info.ggml_type,
                        &ffn_input[..emb_dim],
                        emb_dim,
                        num_experts,
                        &mut gate_logits[..num_experts],
                    );
                } else if let Some(ref mmap) = self.gguf_mmap {
                    let off = router_info.byte_offset as usize;
                    if off < mmap.len() {
                        compute_router_logits_from_raw(
                            &mmap[off..],
                            router_info.ggml_type,
                            &ffn_input[..emb_dim],
                            emb_dim,
                            num_experts,
                            &mut gate_logits[..num_experts],
                        );
                    }
                }
            } else if let Some(ref mmap) = self.gguf_mmap {
                let off = router_info.byte_offset as usize;
                if off < mmap.len() {
                    compute_router_logits_from_raw(
                        &mmap[off..],
                        router_info.ggml_type,
                        &ffn_input[..emb_dim],
                        emb_dim,
                        num_experts,
                        &mut gate_logits[..num_experts],
                    );
                }
            }
        }

        let mut expert_indices = [0u16; crate::inference::moe::routing::MAX_MOE_TOPK];
        let mut expert_weights = [0.0f32; crate::inference::moe::routing::MAX_MOE_TOPK];
        let selected = match crate::inference::moe::routing::route_topk(
            &gate_logits[..num_experts],
            topk,
            true,
            &mut expert_indices,
            &mut expert_weights,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };

        for x in scratch_a[..emb_dim].iter_mut() {
            *x = 0.0;
        }

        let mut gate_buf = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
        let mut up_buf = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
        let mut swiglu_buf = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
        let mut expert_out = [0.0f32; 8192];

        #[cfg(not(target_arch = "wasm32"))]
        let ftw_pkg = self.ftw_package.clone();

        for i in 0..selected {
            let expert_id = expert_indices[i];
            let weight = expert_weights[i];

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(ref pkg) = ftw_pkg {
                if let Some(data) = pkg.get_expert_data(layer_idx, expert_id) {
                    // Derive intermediate_dim from actual tensor geometry.
                    // gate_up_packed is gate+up concatenated: len = 2 * inter_dim * (emb_dim/2)
                    // so inter_dim = len / emb_dim
                    let bytes_per_row = emb_dim / 2; // NVFP4: 2 nibbles/byte
                    let inter_dim = if bytes_per_row > 0 {
                        data.gate_up_packed.len() / (2 * bytes_per_row)
                    } else {
                        0
                    };
                    let view = data.to_view(emb_dim, inter_dim);
                    if crate::inference::moe::dispatch::evaluate_swiglu_expert_nvfp4(
                        &ffn_input[..emb_dim],
                        &view,
                        &mut gate_buf,
                        &mut up_buf,
                        &mut swiglu_buf,
                        &mut expert_out[..emb_dim],
                    ).is_ok() {
                        for d in 0..emb_dim {
                            scratch_a[d] += weight * expert_out[d];
                        }
                    }
                }
            }
        }

        true
    }
}

/// Compute router logits directly from raw weights without allocating a full matrix:
/// g_e = sum_i input[i] * W_router[e, i]
fn compute_router_logits_from_raw(
    raw: &[u8],
    ggml_type: u32,
    input: &[f32],
    emb_dim: usize,
    num_experts: usize,
    out_logits: &mut [f32],
) {
    let mut row_buf = [0.0f32; 8192];
    let row_len = emb_dim.min(row_buf.len());

    for e in 0..num_experts {
        let row_offset = match ggml_type {
            crate::ggml_quants::GGML_TYPE_F32 => e * emb_dim * 4,
            crate::ggml_quants::GGML_TYPE_F16 | crate::ggml_quants::GGML_TYPE_BF16 => e * emb_dim * 2,
            crate::ggml_quants::GGML_TYPE_Q8_0 => e * (emb_dim / 32) * 34,
            _ => e * emb_dim * 2,
        };

        if row_offset < raw.len() {
            if crate::ggml_quants::dequantize_row_into(
                &raw[row_offset..],
                ggml_type,
                row_len,
                &mut row_buf[..row_len],
            ).is_ok() {
                let mut dot = 0.0f32;
                for i in 0..row_len {
                    dot += input[i] * row_buf[i];
                }
                out_logits[e] = dot;
                continue;
            }
        }
        out_logits[e] = 0.0;
    }
}
