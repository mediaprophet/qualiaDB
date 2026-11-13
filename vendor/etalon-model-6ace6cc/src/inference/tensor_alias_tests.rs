use super::{ModelHyperparams, TensorIndex, canonical_component_name};
use crate::gguf::fnv1a64;
use crate::tensor_roles::name_to_role;

#[test]
fn tensor_alias_roles_require_exact_weight_component() {
    for name in [
        "blk.0.attn_q_norm.weight",
        "blk.0.attn_k_norm.weight",
        "blk.0.attn_qkv.weight",
        "blk.0.attn_query.weight",
        "blk.0.attn_q.bias",
        "blk.0.ffn_gate_inp.scale",
        "blk.0.ffn_down_exps.scale",
        "blk.0.ffn_gate.weight.scale",
        "model.layers.3.self_attn.q_proj.bias",
        "model.layers.3.self_attn.q_proj_norm.weight",
        "model.layers.3.mlp.gate_proj_norm.weight",
        "unknown_token_embd.weight",
        "model.layers.3.mlp.shared_expert.gate_proj_norm.weight",
    ] {
        assert_eq!(name_to_role(name), None, "{name}");
    }
    for name in [
        "blk.0.attn_q.weight",
        "blk.0.attn_k.weight",
        "blk.0.ffn_gate_inp.weight",
        "blk.0.ffn_down_exps.weight",
        "model.layers.3.self_attn.q_proj.weight",
        "model.layers.3.mlp.gate_proj.weight",
        "token_embd.weight",
    ] {
        assert!(name_to_role(name).is_some(), "{name}");
    }
}

#[test]
fn tensor_alias_normalizer_preserves_specialized_roles_and_nonweights() {
    for name in [
        "blk.0.attn_q_norm.weight",
        "blk.0.attn_k_norm.weight",
        "blk.0.ffn_gate_inp.scale",
        "blk.0.ffn_down_exps.scale",
        "blk.0.attn_qkv.bias",
        "blk.0.attn_qkv_norm.weight",
        "blk.0.ffn_gate_up_exps.scale",
    ] {
        assert_eq!(
            canonical_component_name(name.as_bytes(), &mut [0; 128]),
            0,
            "{name}"
        );
    }
    for (name, expected) in [
        (
            "model.layers.3.self_attn.q_proj.weight",
            "blk.3.attn_q.weight",
        ),
        (
            "model.layers.3.self_attn.qkv_proj.weight",
            "blk.3.attn_qkv.weight",
        ),
        (
            "model.layers.3.mlp.experts.gate_up_proj.weight",
            "blk.3.ffn_gate_up_exps.weight",
        ),
    ] {
        let mut out = [0; 128];
        let n = canonical_component_name(name.as_bytes(), &mut out);
        assert_eq!(&out[..n], expected.as_bytes());
    }
}

fn string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

#[test]
fn tensor_alias_real_gguf_directory_does_not_duplicate_projection_or_router() {
    // The exact conflicting roles in the physical Gemma/Qwen directories.
    let names = [
        ("blk.0.attn_q.weight", &[2560u64, 2048][..]),
        ("blk.0.attn_q_norm.weight", &[256][..]),
        ("blk.0.attn_k.weight", &[2560, 512][..]),
        ("blk.0.attn_k_norm.weight", &[256][..]),
        ("blk.0.ffn_gate_inp.weight", &[2816, 128][..]),
        ("blk.0.ffn_gate_inp.scale", &[2816][..]),
        ("blk.0.ffn_down_exps.weight", &[704, 2816, 128][..]),
        ("blk.0.ffn_down_exps.scale", &[128][..]),
    ];
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&(names.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    for (ordinal, (name, dims)) in names.iter().enumerate() {
        string(&mut bytes, name);
        bytes.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for d in *dims {
            bytes.extend_from_slice(&d.to_le_bytes());
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&((ordinal as u64) * 32).to_le_bytes());
    }
    let index = TensorIndex::<ModelHyperparams>::from_gguf(&bytes);
    assert_eq!(index.entries.len(), names.len());
    for (ordinal, (name, dims)) in names.iter().enumerate() {
        let matches: Vec<_> = index
            .entries
            .iter()
            .filter(|(h, _)| *h == fnv1a64(name.as_bytes()))
            .collect();
        assert_eq!(matches.len(), 1, "{name}");
        let info = matches[0].1;
        assert_eq!(info.n_dims as usize, dims.len());
        assert_eq!(&info.dims[..dims.len()], *dims);
        assert_eq!(info.byte_offset, (ordinal as u64) * 32);
    }
}
