use std::ops::Range;

use super::types::{CudaDecodePlan, CudaLayerPlan};
use crate::ggml_quants::{tensor_byte_len, GGML_TYPE_Q4_K_SOA, GGML_TYPE_Q8_0};
use crate::gguf_bridge::cpu_ops::dequant_norm_row_into;
use crate::inference::cuda_lane::{
    ensure_device_kv_cache, preload_q4k_soa_weights, preload_resident_blob,
    prepare_mega_pass_kernels, q4k_weight_resident, weight_fingerprint, MegaPassLayerDims,
    MegaPassWeightLayout,
};

impl CudaDecodePlan {
    pub(super) fn build(
        engine: &crate::gguf_bridge::QTensorEngine,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb_dim: usize,
    ) -> Option<Box<Self>> {
        let hyper = index.hyperparams;
        let n_layer = hyper.n_layer;
        let n_embd = hyper.n_embd as usize;
        if n_layer == 0 || n_layer > 128 || n_embd == 0 || n_embd > 4096 || n_embd != emb_dim {
            return None;
        }
        let layout = super::context::configured_dense_layout(engine.kv_layout?)?;
        if !ensure_device_kv_cache(
            layout.max_context,
            layout.n_layer,
            layout.n_kv_head,
            layout.head_dim,
            layout.slot_kv_elems,
            layout.layer_stride,
            layout.total_f32_elems,
        ) {
            return None;
        }
        let mmap = engine.gguf_mmap.clone()?;
        let tensor_data_start = index.tensor_data_start;
        let mut layers = Vec::with_capacity(n_layer as usize);
        let mut weight_layout = None;

        for layer_index in 0..n_layer {
            let tensors = index.get_layer_tensors(layer_index);
            let q = tensors.attn_q.as_ref()?;
            let k = tensors.attn_k.as_ref()?;
            let v = tensors.attn_v.as_ref()?;
            let o = tensors.attn_output.as_ref()?;
            let gate = tensors.ffn_gate.as_ref()?;
            let up = tensors.ffn_up.as_ref()?;
            let down = tensors.ffn_down.as_ref()?;
            let layer_layout = match q.ggml_type {
                GGML_TYPE_Q4_K_SOA => MegaPassWeightLayout::Q4KSoa,
                GGML_TYPE_Q8_0 => MegaPassWeightLayout::Q8_0,
                _ => return None,
            };
            if [q, k, v, o, gate, up, down]
                .iter()
                .any(|tensor| tensor.ggml_type != q.ggml_type)
                || weight_layout.is_some_and(|layout| layout != layer_layout)
            {
                return None;
            }
            weight_layout = Some(layer_layout);

            let (q_in, q_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(q);
            let (kv_in, kv_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(k);
            let (o_in, o_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(o);
            let (gate_in, gate_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(gate);
            let (up_in, up_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(up);
            let (down_in, down_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(down);
            let mut attn_norm = vec![0.0f32; n_embd];
            let mut ffn_norm = vec![0.0f32; n_embd];
            if let Some(info) = tensors.attn_norm.as_ref() {
                if dequant_norm_row_into(&mmap, tensor_data_start, info, &mut attn_norm) < n_embd {
                    return None;
                }
            }
            if let Some(info) = tensors.ffn_norm.as_ref() {
                if dequant_norm_row_into(&mmap, tensor_data_start, info, &mut ffn_norm) < n_embd {
                    return None;
                }
            }
            let dims = MegaPassLayerDims {
                q_in,
                q_out,
                kv_in,
                kv_out,
                o_in,
                o_out,
                gate_in,
                gate_out,
                up_in,
                up_out,
                down_in,
                down_out,
            };
            let weights = [
                tensor_range(&mmap, tensor_data_start, q)?,
                tensor_range(&mmap, tensor_data_start, k)?,
                tensor_range(&mmap, tensor_data_start, v)?,
                tensor_range(&mmap, tensor_data_start, o)?,
                tensor_range(&mmap, tensor_data_start, gate)?,
                tensor_range(&mmap, tensor_data_start, up)?,
                tensor_range(&mmap, tensor_data_start, down)?,
            ];
            let weight_keys = [
                weight_fingerprint(mmap.get(weights[0].clone())?, q_in, q_out),
                weight_fingerprint(mmap.get(weights[1].clone())?, kv_in, kv_out),
                weight_fingerprint(mmap.get(weights[2].clone())?, kv_in, kv_out),
                weight_fingerprint(mmap.get(weights[3].clone())?, o_in, o_out),
                weight_fingerprint(mmap.get(weights[4].clone())?, gate_in, gate_out),
                weight_fingerprint(mmap.get(weights[5].clone())?, up_in, up_out),
                weight_fingerprint(mmap.get(weights[6].clone())?, down_in, down_out),
            ];
            let norm_keys = [
                norm_fingerprint(&attn_norm, 0xA771_0001),
                norm_fingerprint(&ffn_norm, 0xFF71_0002),
            ];
            if !preload_resident_blob(norm_keys[0], bytemuck::cast_slice(&attn_norm))
                || !preload_resident_blob(norm_keys[1], bytemuck::cast_slice(&ffn_norm))
            {
                return None;
            }
            layers.push(CudaLayerPlan {
                dims,
                weights,
                weight_keys,
                norm_keys,
            });
        }

        let weight_layout = weight_layout?;
        match weight_layout {
            MegaPassWeightLayout::Q4KSoa => {
                let mut preload = Vec::with_capacity(layers.len() * 7);
                for layer in &layers {
                    let dimensions = layer_dimensions(layer);
                    for (range, (n_in, n_out)) in layer.weights.iter().zip(dimensions) {
                        preload.push((mmap.get(range.clone())?, n_in, n_out));
                    }
                }
                let _newly_resident = preload_q4k_soa_weights(&preload);
            }
            MegaPassWeightLayout::Q8_0 => {
                for layer in &layers {
                    for (range, key) in layer.weights.iter().zip(layer.weight_keys) {
                        if !preload_resident_blob(key, mmap.get(range.clone())?) {
                            return None;
                        }
                    }
                }
            }
        }
        if layers.iter().any(|layer| {
            layer
                .weight_keys
                .iter()
                .any(|key| !q4k_weight_resident(*key))
        }) {
            return None;
        }
        if !prepare_mega_pass_kernels() {
            return None;
        }

        let token_embedding_info = index.token_embd_info()?;
        let token_embedding_vocab = index.vocab_dim();
        let token_embedding = (weight_layout == MegaPassWeightLayout::Q8_0
            && token_embedding_info.ggml_type == GGML_TYPE_Q8_0)
            .then(|| tensor_range(&mmap, tensor_data_start, token_embedding_info))
            .flatten();
        let token_embedding_key = token_embedding.as_ref().and_then(|range| {
            mmap.get(range.clone())
                .map(|raw| weight_fingerprint(raw, n_embd, token_embedding_vocab))
        });
        if let (Some(range), Some(key)) = (token_embedding.as_ref(), token_embedding_key) {
            if !preload_resident_blob(key, mmap.get(range.clone())?) || !q4k_weight_resident(key) {
                return None;
            }
        }

        let output_norm_key = index.output_norm_info().and_then(|info| {
            let mut values = vec![0.0f32; n_embd];
            if dequant_norm_row_into(&mmap, tensor_data_start, info, &mut values) < n_embd {
                return None;
            }
            let key = norm_fingerprint(&values, 0x0A71_0003);
            preload_resident_blob(key, bytemuck::cast_slice(&values)).then_some(key)
        });
        if index.output_norm_info().is_some() && output_norm_key.is_none() {
            return None;
        }
        let lm_info = index.logits_projection_info()?;
        let (lm_head_in, lm_head_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(lm_info);
        let expected_lm_type = match weight_layout {
            MegaPassWeightLayout::Q4KSoa => GGML_TYPE_Q4_K_SOA,
            MegaPassWeightLayout::Q8_0 => GGML_TYPE_Q8_0,
        };
        let lm_head = (lm_info.ggml_type == expected_lm_type)
            .then(|| tensor_range(&mmap, tensor_data_start, lm_info))
            .flatten();
        let lm_head_key = lm_head.as_ref().and_then(|range| {
            mmap.get(range.clone())
                .map(|raw| weight_fingerprint(raw, lm_head_in, lm_head_out))
        });
        if let (Some(range), Some(key)) = (lm_head.as_ref(), lm_head_key) {
            let raw = mmap.get(range.clone())?;
            match weight_layout {
                MegaPassWeightLayout::Q4KSoa => {
                    let _ = preload_q4k_soa_weights(&[(raw, lm_head_in, lm_head_out)]);
                }
                MegaPassWeightLayout::Q8_0 => {
                    if !preload_resident_blob(key, raw) {
                        return None;
                    }
                }
            }
            if !q4k_weight_resident(key) {
                return None;
            }
        }
        let key = (
            mmap.as_ptr() as u64,
            tensor_data_start,
            index.hyperparams.n_layer,
        );

        Some(Box::new(Self {
            key,
            weight_layout,
            mmap,
            layers,
            output_norm_key,
            token_embedding_key,
            token_embedding_vocab,
            lm_head,
            lm_head_key,
            lm_head_in,
            lm_head_out,
            n_embd,
            n_head: hyper.n_head as usize,
            n_kv: hyper.effective_n_kv_head() as usize,
            head_dim: hyper.head_dim() as usize,
            n_layer,
            max_context: layout.max_context,
            layer_stride: layout.layer_stride,
            slot_kv_elems: layout.slot_kv_elems,
            rope_base: hyper.effective_rope_freq_base(),
            rope_scale: hyper.effective_rope_scale(),
            rms_eps: super::super::RMS_NORM_EPS,
        }))
    }
}

fn layer_dimensions(layer: &CudaLayerPlan) -> [(usize, usize); 7] {
    [
        (layer.dims.q_in, layer.dims.q_out),
        (layer.dims.kv_in, layer.dims.kv_out),
        (layer.dims.kv_in, layer.dims.kv_out),
        (layer.dims.o_in, layer.dims.o_out),
        (layer.dims.gate_in, layer.dims.gate_out),
        (layer.dims.up_in, layer.dims.up_out),
        (layer.dims.down_in, layer.dims.down_out),
    ]
}

fn norm_fingerprint(values: &[f32], domain: usize) -> u64 {
    weight_fingerprint(bytemuck::cast_slice(values), values.len(), domain)
}

fn tensor_range(
    mmap: &[u8],
    tensor_data_start: u64,
    tensor: &crate::gguf_sharder::GgufTensorInfo,
) -> Option<Range<usize>> {
    let start = tensor_data_start.checked_add(tensor.byte_offset)? as usize;
    let end = start.checked_add(tensor_byte_len(tensor)?)?;
    (end <= mmap.len()).then_some(start..end)
}
