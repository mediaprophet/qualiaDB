use super::*;
use crate::{
    gguf_bridge::{KvCacheLayout, ModelByteHandle, QTensorEngine},
    gguf_sharder::{GgufHyperparams, GgufTensorIndex},
};
use etalon_model::inference::TensorInfo;
use std::sync::Arc;

fn fixture(layers: usize) -> (Profile, GgufTensorIndex, ModelByteHandle) {
    let p = Profile {
        hidden: 4,
        ff: 8,
        layers,
        interval: 4,
        kh: 2,
        vh: 4,
        kd: 2,
        vd: 2,
        channels: 16,
        value: 8,
        epsilon: 1e-6,
    };
    let h = GgufHyperparams {
        n_layer: layers as u32,
        n_embd: 4,
        n_head: 1,
        n_kv_head: 1,
        head_dim: 64,
        rope_dim: 64,
        rope_freq_base: 10_000_000.,
        rms_norm_eps: p.epsilon,
        ..Default::default()
    };
    let mut payload = vec![0u8; 64];
    let mut entries = Vec::new();
    let mut add = |name: &str, dims: &[usize], value: f32| {
        let mut shape = [0; 4];
        for (i, n) in dims.iter().enumerate() {
            shape[i] = *n as u64;
        }
        let info = TensorInfo {
            dims: shape,
            n_dims: dims.len() as u32,
            ggml_type: 0,
            byte_offset: (payload.len() - 64) as u64,
        };
        for _ in 0..dims.iter().product::<usize>() {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        entries.push((etalon_model::gguf::fnv1a64(name.as_bytes()), info));
        info
    };
    let token = add("token_embd.weight", &[4, 8], 0.01);
    let output = add("output_norm.weight", &[4], 1.);
    for l in 0..layers {
        for name in ["attn_norm.weight", "post_attention_norm.weight"] {
            add(&format!("blk.{l}.{name}"), &[4], 1.);
        }
        for name in ["ffn_gate.weight", "ffn_up.weight"] {
            add(&format!("blk.{l}.{name}"), &[4, 8], 0.01);
        }
        add(&format!("blk.{l}.ffn_down.weight"), &[8, 4], 0.01);
        if p.linear(l) {
            for (name, dims, value) in [
                ("attn_qkv.weight", vec![4, 16], 0.01),
                ("attn_gate.weight", vec![4, 8], 0.01),
                ("ssm_alpha.weight", vec![4, 4], 0.01),
                ("ssm_beta.weight", vec![4, 4], 0.01),
                ("ssm_conv1d.weight", vec![4, 16], 0.01),
                ("ssm_a", vec![4], -1.),
                ("ssm_dt.bias", vec![4], 0.1),
                ("ssm_norm.weight", vec![2], 1.),
                ("ssm_out.weight", vec![8, 4], 0.01),
            ] {
                add(&format!("blk.{l}.{name}"), &dims, value);
            }
        } else {
            for (name, dims, value) in [
                ("attn_q.weight", vec![4, 128], 0.01),
                ("attn_k.weight", vec![4, 64], 0.01),
                ("attn_v.weight", vec![4, 64], 0.01),
                ("attn_output.weight", vec![64, 4], 0.01),
                ("attn_q_norm.weight", vec![64], 1.),
                ("attn_k_norm.weight", vec![64], 1.),
            ] {
                add(&format!("blk.{l}.{name}"), &dims, value);
            }
        }
    }
    entries.sort_by_key(|v| v.0);
    let mut map = memmap2::MmapMut::map_anon(payload.len()).unwrap();
    map.copy_from_slice(&payload);
    let owner = ModelByteHandle::Legacy(Arc::new(map.make_read_only().unwrap()));
    let index = GgufTensorIndex {
        entries,
        tensor_data_start: 64,
        token_embd: Some(token),
        output_weight: None,
        output_norm: Some(output),
        ple_ngram_embedding: None,
        ple_config: None,
        moe_routing: Default::default(),
        hyperparams: h,
        max_tensor_bytes: 2048,
        max_layer_tensor_bytes: 2048,
    };
    (p, index, owner)
}
fn engine(layers: usize) -> (QTensorEngine, GgufTensorIndex) {
    let (p, index, source) = fixture(layers);
    let mut e = QTensorEngine::new();
    e.qwen35_gguf = Some(Session::new(source.clone(), index.clone(), p, 2 * 1024 * 1024).unwrap());
    e.qwen35_gguf_required = true;
    e.prepared_native_bytes = Some(source);
    e.hyperparams = index.hyperparams;
    e.tensor_data_offset = index.tensor_data_start;
    e.tensor_index_cache = Some(index.clone());
    let layout = KvCacheLayout::from_hyperparams_mode(&index.hyperparams, false, 0, 0).unwrap();
    e.kv_cache_cpu = Some(vec![0.; layout.total_f32_elems].into_boxed_slice());
    e.kv_layout = Some(layout);
    (e, index)
}
#[test]
fn dense_gguf_recurrent_geometry_and_tiled_heads() {
    let p = Profile {
        hidden: 2560,
        ff: 9216,
        layers: 32,
        interval: 4,
        kh: 16,
        vh: 32,
        kd: 128,
        vd: 128,
        channels: 8192,
        value: 4096,
        epsilon: 1e-6,
    };
    assert_eq!(p.state_stride(), 548864);
    assert_eq!(linear::tiled_query_offset(p, 0), 0);
    assert_eq!(linear::tiled_query_offset(p, 1), 256);
    assert_eq!(linear::tiled_query_offset(p, 16), 0);
    let offsets: Vec<_> = (0..32)
        .filter(|l| p.linear(*l))
        .map(|l| (l - l / p.interval) * p.state_stride())
        .collect();
    assert_eq!(offsets.len(), 24);
    assert!(offsets.windows(2).all(|w| w[1] - w[0] == p.state_stride()));
    assert_eq!(offsets[23] + p.state_stride(), 24 * p.state_stride());
}
#[test]
fn dense_gguf_all_32_layers_and_reset_are_reproducible() {
    let (mut e, index) = engine(32);
    let mut hidden = [0.5; 4];
    let mut a = [0.; 128];
    let mut b = [0.; 128];
    assert_eq!(
        e.dense_qwen_forward(&index, &mut hidden, 4, 0, 0, &mut a, &mut b),
        32
    );
    let first = hidden;
    assert!(e.dense_qwen_ready(&index));
    hidden.fill(0.5);
    assert_eq!(
        e.dense_qwen_forward(&index, &mut hidden, 4, 1, 0, &mut a, &mut b),
        32
    );
    e.reset_kv_cache();
    hidden.fill(0.5);
    assert_eq!(
        e.dense_qwen_forward(&index, &mut hidden, 4, 0, 0, &mut a, &mut b),
        32
    );
    assert_eq!(first, hidden);
}
#[test]
fn dense_gguf_attention_failure_cannot_count_ffn_or_emit_logits() {
    let (mut e, index) = engine(4);
    let mut h = [0.5; 4];
    let mut a = [0.; 128];
    let mut b = [0.; 128];
    for l in 0..3 {
        assert!(e.dense_qwen_layer(&index, l, 0, &mut h, 4, &mut a, &mut b));
    }
    e.kv_cache_cpu = None;
    assert!(!e.dense_qwen_layer(&index, 3, 0, &mut h, 4, &mut a, &mut b));
    assert!(!e.dense_qwen_ready(&index));
    assert!(!e.dense_qwen_output_norm(&index, &mut h, 4));
    assert!(!e.dense_qwen_layer(&index, 3, 0, &mut h, 4, &mut a, &mut b));
}
#[test]
fn dense_gguf_wrong_position_directory_and_source_are_refused() {
    let (mut e, index) = engine(4);
    let mut h = [0.5; 4];
    let mut a = [0.; 128];
    let mut b = [0.; 128];
    let mut bad = index.clone();
    bad.entries[0].1.byte_offset += 4;
    assert_eq!(
        e.dense_qwen_forward(&bad, &mut h, 4, 0, 0, &mut a, &mut b),
        0
    );
    let (_, _, foreign) = fixture(4);
    e.prepared_native_bytes = Some(foreign);
    assert_eq!(
        e.dense_qwen_forward(&index, &mut h, 4, 0, 0, &mut a, &mut b),
        0
    );
    let (mut e, index) = engine(4);
    assert_eq!(
        e.dense_qwen_forward(&index, &mut h, 4, 1, 0, &mut a, &mut b),
        0
    );
    assert_eq!(
        e.dense_qwen_forward(&index, &mut h, 4, 0, 0, &mut a, &mut b),
        0
    );
}
#[test]
fn dense_gguf_missing_role_duplicate_and_budget_fail_cold() {
    let (p, index, source) = fixture(4);
    assert!(Session::new(source.clone(), index.clone(), p, 0).is_err());
    let mut bad = index.clone();
    bad.entries.pop();
    assert!(Session::new(source.clone(), bad, p, 2 * 1024 * 1024).is_err());
    let mut bad = index.clone();
    bad.entries.push(bad.entries[0]);
    assert!(Session::new(source, bad, p, 2 * 1024 * 1024).is_err());
}

#[test]
fn dense_gguf_metadata_requires_exact_partial_rope_and_unique_fields() {
    fn metadata(extra: Option<(&str, u32, Vec<u8>)>) -> Vec<u8> {
        let mut kv = vec![
            ("general.architecture", 8, {
                let mut b = 6u64.to_le_bytes().to_vec();
                b.extend(b"qwen35");
                b
            }),
            (
                "qwen35.feed_forward_length",
                4,
                9216u32.to_le_bytes().to_vec(),
            ),
            (
                "qwen35.rope.dimension_count",
                4,
                64u32.to_le_bytes().to_vec(),
            ),
            (
                "qwen35.attention.value_length",
                4,
                256u32.to_le_bytes().to_vec(),
            ),
            (
                "qwen35.attention.layer_norm_rms_epsilon",
                6,
                1e-6f32.to_le_bytes().to_vec(),
            ),
            ("qwen35.rope.dimension_sections", 9, {
                let mut b = 5u32.to_le_bytes().to_vec();
                b.extend(4u64.to_le_bytes());
                for n in [11i32, 11, 10, 0] {
                    b.extend(n.to_le_bytes());
                }
                b
            }),
        ];
        if let Some(e) = extra {
            kv.push(e);
        }
        let mut b = b"GGUF".to_vec();
        b.extend(3u32.to_le_bytes());
        b.extend(0u64.to_le_bytes());
        b.extend((kv.len() as u64).to_le_bytes());
        for (k, t, v) in kv {
            b.extend((k.len() as u64).to_le_bytes());
            b.extend(k.as_bytes());
            b.extend(t.to_le_bytes());
            b.extend(v);
        }
        b
    }
    let (_, mut i, _) = fixture(32);
    i.hyperparams = GgufHyperparams {
        n_layer: 32,
        n_embd: 2560,
        n_head: 16,
        n_kv_head: 4,
        head_dim: 256,
        rope_freq_base: 10_000_000.,
        ssm_conv_kernel: 4,
        ssm_state_size: 128,
        ssm_group_count: 16,
        ssm_time_step_rank: 32,
        ssm_inner_size: 4096,
        full_attention_interval: 4,
        ..Default::default()
    };
    assert!(Profile::parse(&metadata(None), &mut i).unwrap().is_some());
    assert_eq!(i.hyperparams.rope_dim, 64);
    assert_eq!(i.hyperparams.rms_norm_eps, 1e-6);
    assert!(
        Profile::parse(
            &metadata(Some((
                "qwen35.rope.dimension_count",
                4,
                256u32.to_le_bytes().to_vec()
            ))),
            &mut i
        )
        .is_err()
    );
    assert!(
        Profile::parse(
            &metadata(Some((
                "qwen35.rope.scaling.factor",
                6,
                2f32.to_le_bytes().to_vec()
            ))),
            &mut i
        )
        .is_err()
    );
}
