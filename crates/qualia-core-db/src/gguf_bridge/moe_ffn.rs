//! MoE FFN dispatch: routing, expert GEMV accumulation, and FTW package adoption.

use super::*;
use crate::inference::moe::dispatch::MAX_ROUTED_EXPERTS;
use crate::inference::moe::ftw_loader::FtwModelPackage;
use std::sync::Arc;

use super::moe_gguf::{
    compute_router_logits_from_gguf, compute_router_logits_from_raw, dot,
    evaluate_gguf_dense_swiglu, evaluate_gguf_swiglu_expert,
    evaluate_gguf_swiglu_expert_fused_gate_up,
};

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
            mapped_bytes: self
                .ftw_package
                .as_ref()
                .map(|p| p.manifest.total_bytes)
                .unwrap_or(0),
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
        index: &crate::gguf_sharder::GgufTensorIndex,
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
        let num_experts = (router_info.dims[1] as usize)
            .min(MAX_ROUTED_EXPERTS)
            .max(1);
        let topk = 8usize.min(num_experts);

        let mut gate_logits = [0.0f32; MAX_ROUTED_EXPERTS];
        let mut router_ready = false;

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref pkg) = self.ftw_package {
                let router_name = format!("model.layers.{}.mlp.gate.weight", layer_idx);
                let alt_router_name = format!("blk.{}.ffn_gate_inp.weight", layer_idx);
                let router_bytes = pkg
                    .fetch_tensor_bytes(&router_name)
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
                    router_ready = true;
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
                        router_ready = true;
                    }
                }
            }
        }

        // Ordinary GGUF expert tensors are 3-D ([input, intermediate, expert]).
        // Their rows must be addressed through the tensor-data base, not the
        // raw `byte_offset` (which is relative to that base).  This also keeps
        // Q4_K/Q6_K router rows on the canonical dequantization path.
        if !router_ready {
            let Some(mmap) = self.gguf_mmap.as_deref() else {
                return false;
            };
            if let Ok(raw) =
                crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, router_info)
            {
                if !compute_router_logits_from_gguf(
                    raw,
                    router_info,
                    &ffn_input[..emb_dim],
                    num_experts,
                    &mut gate_logits[..num_experts],
                ) {
                    return false;
                }
            } else {
                return false;
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
        let mmap = self.gguf_mmap.clone();

        let direct_gate = tensors.moe_gate_exps.as_ref();
        let direct_up = tensors.moe_up_exps.as_ref();
        let direct_gate_up = tensors.moe_gate_up_exps.as_ref();
        let direct_down = tensors.moe_down_exps.as_ref();

        for i in 0..selected {
            let expert_id = expert_indices[i];
            let weight = expert_weights[i];

            let mut evaluated = false;
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
                    )
                    .is_ok()
                    {
                        for d in 0..emb_dim {
                            scratch_a[d] += weight * expert_out[d];
                        }
                        evaluated = true;
                    }
                }
            }
            if !evaluated {
                let Some(mmap) = mmap.as_deref() else {
                    return false;
                };
                let direct_ok = match (direct_gate, direct_up, direct_gate_up, direct_down) {
                    (Some(gate), Some(up), _, Some(down)) => evaluate_gguf_swiglu_expert(
                        mmap,
                        index.tensor_data_start,
                        gate,
                        up,
                        down,
                        expert_id as usize,
                        &ffn_input[..emb_dim],
                        &mut gate_buf,
                        &mut up_buf,
                        &mut swiglu_buf,
                        &mut expert_out[..emb_dim],
                    ),
                    (_, _, Some(gate_up), Some(down)) => evaluate_gguf_swiglu_expert_fused_gate_up(
                        mmap,
                        index.tensor_data_start,
                        gate_up,
                        down,
                        expert_id as usize,
                        &ffn_input[..emb_dim],
                        &mut gate_buf,
                        &mut up_buf,
                        &mut swiglu_buf,
                        &mut expert_out[..emb_dim],
                    ),
                    _ => false,
                };
                if !direct_ok {
                    return false;
                }
                for d in 0..emb_dim {
                    scratch_a[d] += weight * expert_out[d];
                }
            }
        }

        // Qwen-style shared expert: an ordinary SwiGLU MLP whose output is
        // scaled by sigmoid(W_shared_gate x).  It is mandatory whenever any
        // of its tensors are present; partial tensor sets fail closed.
        if tensors.moe_shared_gate.is_some()
            || tensors.moe_shared_up.is_some()
            || tensors.moe_shared_down.is_some()
            || tensors.moe_shared_gate_input.is_some()
        {
            let (Some(gate), Some(up), Some(down), Some(gate_input), Some(mmap)) = (
                tensors.moe_shared_gate.as_ref(),
                tensors.moe_shared_up.as_ref(),
                tensors.moe_shared_down.as_ref(),
                tensors.moe_shared_gate_input.as_ref(),
                mmap.as_deref(),
            ) else {
                return false;
            };
            if !evaluate_gguf_dense_swiglu(
                mmap,
                index.tensor_data_start,
                gate,
                up,
                down,
                &ffn_input[..emb_dim],
                &mut gate_buf,
                &mut up_buf,
                &mut swiglu_buf,
                &mut expert_out[..emb_dim],
            ) {
                return false;
            }
            let Ok(gate_raw) =
                crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_input)
            else {
                return false;
            };
            let mut gate_row = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
            if gate_input.n_dims != 1
                || gate_input.dims[0] as usize != emb_dim
                || emb_dim > gate_row.len()
                || crate::ggml_quants::dequantize_row_into(
                    gate_raw,
                    gate_input.ggml_type,
                    emb_dim,
                    &mut gate_row[..emb_dim],
                )
                .is_err()
            {
                return false;
            }
            let gate_weight =
                1.0 / (1.0 + (-dot(&ffn_input[..emb_dim], &gate_row[..emb_dim])).exp());
            for d in 0..emb_dim {
                scratch_a[d] += gate_weight * expert_out[d];
            }
        }

        true
    }

    /// Dispatches MoE layer using the zero-heap `dispatch_moe_step` driver with expert fetcher (EOS-070).
    pub fn dispatch_moe_step_routed<F>(
        &mut self,
        emb_dim: usize,
        router_weights: &[f32],
        num_experts: usize,
        topk: usize,
        expert_fetcher: F,
        shared_expert: Option<&crate::inference::moe::dispatch::ExpertWeightView<'_>>,
        shared_gate_weight: f32,
        ffn_input: &[f32],
        scratch_a: &mut [f32],
    ) -> bool
    where
        F: FnMut(u16) -> Option<crate::inference::moe::dispatch::ExpertWeightView<'static>>,
    {
        crate::inference::moe::dispatch::dispatch_moe_step(
            &ffn_input[..emb_dim],
            emb_dim,
            router_weights,
            num_experts,
            topk,
            expert_fetcher,
            shared_expert,
            shared_gate_weight,
            &mut scratch_a[..emb_dim],
        )
        .is_ok()
    }

    /// Dispatches MoE layer using the clustered shared-down MoE operator (EOS-071).
    pub fn dispatch_clustered_moe_operator(
        &mut self,
        op: &crate::inference::operator_runtime::ClusteredMoEOperator,
        routed_experts: &[crate::inference::operator_runtime::RoutedExpert],
        ffn_input: &[f32],
        output: &mut [f32],
        scratch: &mut crate::inference::operator_runtime::ClusteredMoeScratch<'_>,
    ) -> bool {
        op.dispatch(routed_experts, ffn_input, output, scratch).is_ok()
    }
}

