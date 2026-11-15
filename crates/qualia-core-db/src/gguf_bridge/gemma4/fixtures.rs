use super::super::{metadata::Profile, session::Session};
use crate::{
    gguf_bridge::{ModelByteHandle, QTensorEngine},
    gguf_sharder::{GgufHyperparams, GgufTensorIndex},
};
use etalon_model::inference::TensorInfo;
use std::sync::Arc;

pub(super) fn fixture() -> (Profile, GgufTensorIndex, ModelByteHandle) {
    let p = Profile {
        hidden: 4,
        layers: 6,
        heads: 16,
        kv_heads: vec![8, 2, 8, 2, 8, 2],
        sliding: vec![true, false, true, false, true, false],
        head_dim: 2,
        swa_dim: 2,
        window: 2,
        shared: 2,
        ple: 2,
        ff: vec![4; 6],
        experts: 0,
        topk: 0,
        expert_ff: 0,
        rope_dim: 2,
        swa_rope_dim: 2,
        rope_base: 10000.,
        swa_base: 10000.,
        epsilon: 1e-6,
        softcap: 30.,
        trained_context: 8,
    };
    let mut bytes = vec![0u8; 64];
    let mut entries = Vec::new();
    let mut add = |name: &str, dims: &[usize], kind: u8| {
        let mut shape = [0; 4];
        for (i, d) in dims.iter().enumerate() {
            shape[i] = *d as u64;
        }
        let info = TensorInfo {
            dims: shape,
            n_dims: dims.len() as u32,
            ggml_type: 0,
            byte_offset: (bytes.len() - 64) as u64,
        };
        for i in 0..dims.iter().product::<usize>() {
            let v = match kind {
                0 => 1.,
                1 => ((i * 7 % 13) as f32 - 5.) * 0.01,
                2 => ((i * 11 % 17) as f32 - 7.) * 0.1,
                _ => unreachable!(),
            };
            bytes.extend(v.to_le_bytes());
        }
        entries.push((etalon_model::gguf::fnv1a64(name.as_bytes()), info));
        info
    };
    let token = add("token_embd.weight", &[4, 3], 2);
    let output = add("output_norm.weight", &[4], 0);
    add("rope_freqs.weight", &[1], 0);
    let ple = add("per_layer_token_embd.weight", &[12, 3], 2);
    add("per_layer_model_proj.weight", &[4, 12], 1);
    add("per_layer_proj_norm.weight", &[2], 0);
    for layer in 0..6 {
        for name in [
            "attn_norm.weight",
            "post_attention_norm.weight",
            "ffn_norm.weight",
            "post_ffw_norm.weight",
            "post_norm.weight",
        ] {
            add(&format!("blk.{layer}.{name}"), &[4], 0);
        }
        for name in ["attn_q_norm.weight", "attn_k_norm.weight"] {
            add(&format!("blk.{layer}.{name}"), &[2], 0);
        }
        add(&format!("blk.{layer}.attn_q.weight"), &[4, 32], 1);
        add(&format!("blk.{layer}.attn_output.weight"), &[32, 4], 1);
        for name in ["attn_k.weight", "attn_v.weight"] {
            add(
                &format!("blk.{layer}.{name}"),
                &[4, 2 * p.kv_heads[layer]],
                1,
            );
        }
        for name in ["ffn_gate.weight", "ffn_up.weight", "ffn_down.weight"] {
            add(&format!("blk.{layer}.{name}"), &[4, 4], 1);
        }
        add(&format!("blk.{layer}.inp_gate.weight"), &[4, 2], 1);
        add(&format!("blk.{layer}.proj.weight"), &[2, 4], 1);
        add(&format!("blk.{layer}.layer_output_scale.weight"), &[1], 0);
    }
    entries.sort_by_key(|v| v.0);
    let mut map = memmap2::MmapMut::map_anon(bytes.len()).unwrap();
    map.copy_from_slice(&bytes);
    let source = ModelByteHandle::Legacy(Arc::new(map.make_read_only().unwrap()));
    let index = GgufTensorIndex {
        entries,
        tensor_data_start: 64,
        token_embd: Some(token),
        output_weight: None,
        output_norm: Some(output),
        ple_ngram_embedding: Some(ple),
        ple_config: None,
        moe_routing: Default::default(),
        hyperparams: GgufHyperparams {
            architecture: crate::gguf_sharder::ARCH_GEMMA4,
            ..Default::default()
        },
        max_tensor_bytes: 512,
        max_layer_tensor_bytes: 512,
    };
    (p, index, source)
}
pub(super) fn engine() -> (QTensorEngine, GgufTensorIndex) {
    let (p, index, source) = fixture();
    let s = Session::new(source.clone(), index, p, 4, 1024 * 1024).unwrap();
    let index = s.plan.index.clone();
    let mut e = QTensorEngine::new();
    e.gemma4 = Some(s);
    e.gemma4_required = true;
    e.prepared_native_bytes = Some(source);
    e.tensor_index_cache = Some(index.clone());
    e.hyperparams = index.hyperparams;
    e.tensor_data_offset = index.tensor_data_start;
    (e, index)
}
