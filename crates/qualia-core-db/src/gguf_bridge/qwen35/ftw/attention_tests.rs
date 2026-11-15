use super::*;
use etalon_inference::ordinary::qwen35::weights::{CpuByteHost, SliceBytes};
use etalon_model::inference::{
    qwen_attention::{QkNorm, Qwen35AttentionGeometry},
    TensorInfo,
};

#[test]
fn ftw_fused_weight_and_scale_keep_distinct_source_descriptors() {
    // The compatibility index canonicalizes both qkv_proj.weight and
    // qkv_proj.weight_scale to attn_qkv.weight. Architecture admission must
    // resolve the original manifest entries, not select either collision.
    let dir = tempfile::tempdir().unwrap();
    let manifest = serde_json::json!({
        "format":"freetoken_weight", "version":1, "total_bytes":16,
        "shards":[{"file":"weights.ftw","global_off":0,"nbytes":16}],
        "tensors":[
            {"name":"model.layers.3.self_attn.qkv_proj.weight","kind":"weight",
             "dtype":"float8_e4m3fn","shape":[2,4],"global_off":0,"nbytes":8},
            {"name":"model.layers.3.self_attn.qkv_proj.weight_scale","kind":"weight",
             "dtype":"float32","shape":[2],"global_off":8,"nbytes":8}
        ]
    });
    let metadata = serde_json::to_vec(&manifest).unwrap();
    let config = br#"{"model_type":"qwen2","hidden_size":4,"num_hidden_layers":4,"num_attention_heads":1,"num_key_value_heads":1,"head_dim":4}"#;
    assert!(
        metadata.len() + config.len() + 16 < 2048,
        "fixture byte budget"
    );
    std::fs::write(dir.path().join("freetoken_weight.json"), metadata).unwrap();
    std::fs::write(dir.path().join("config.json"), config).unwrap();
    std::fs::write(dir.path().join("weights.ftw"), [0u8; 16]).unwrap();
    let pkg = FtwModelPackage::open_from_dir(dir.path()).unwrap();
    let p = projection(&pkg, 3, "self_attn.qkv_proj", 4, 2).unwrap();
    assert_eq!(p.weight.byte_offset, 0);
    assert_eq!(p.weight.dims, [4, 2, 0, 0]);
    assert_eq!(p.row_scale.unwrap().byte_offset, 8);
    assert_eq!(p.row_scale.unwrap().dims, [2, 0, 0, 0]);
}

fn matrix(offset: u64, cols: u64, rows: u64, ty: u32) -> TensorInfo {
    TensorInfo {
        byte_offset: offset,
        dims: [cols, rows, 0, 0],
        n_dims: 2,
        ggml_type: ty,
    }
}

#[test]
fn fused_qkv_slices_keep_independent_row_scales_and_bounds() {
    use crate::ggml_quants::{GGML_TYPE_F32, GGML_TYPE_FP8_E4M3};
    let fused = Projection {
        weight: matrix(100, 8, 12, GGML_TYPE_FP8_E4M3),
        row_scale: Some(TensorInfo {
            byte_offset: 300,
            dims: [12, 0, 0, 0],
            n_dims: 1,
            ggml_type: GGML_TYPE_F32,
        }),
    };
    let k = row_range(fused, 8, 2).unwrap();
    assert_eq!(k.weight.byte_offset, 164);
    assert_eq!(k.weight.dims, [8, 2, 0, 0]);
    assert_eq!(k.row_scale.unwrap().byte_offset, 332);
    assert_eq!(k.row_scale.unwrap().dims, [2, 0, 0, 0]);
    let v = row_range(fused, 10, 2).unwrap();
    assert_eq!(v.weight.byte_offset, 180);
    assert_eq!(v.row_scale.unwrap().byte_offset, 340);
    assert!(row_range(fused, 11, 2).is_err());
    assert!(row_range(fused, usize::MAX, 1).is_err());
    assert!(row_range(fused, 0, 0).is_err());
}

#[test]
fn gated_full_attention_preserves_cache_and_hidden_on_capacity_failure() {
    use crate::ggml_quants::GGML_TYPE_F32;
    // Q/gate: Q=[1,0], gate=[0,0]; K=identity, V=identity,
    // O=identity, ordinary QK scale=1. Gate sigmoid is exactly 1/2.
    let weights: [f32; 24] = [
        1., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 1., 0., 0., 1., 1., 1., 1.,
        1.,
    ];
    let bytes: Vec<u8> = weights.into_iter().flat_map(f32::to_le_bytes).collect();
    let p = |off, rows| Projection {
        weight: matrix(off, 2, rows, GGML_TYPE_F32),
        row_scale: None,
    };
    let vector = |off| TensorInfo {
        byte_offset: off,
        dims: [2, 0, 0, 0],
        n_dims: 1,
        ggml_type: GGML_TYPE_F32,
    };
    let plan = AttentionPlan::Full(Qwen35AttentionRoles {
        geometry: Qwen35AttentionGeometry {
            hidden: 2,
            query_heads: 1,
            kv_heads: 1,
            head_dim: 2,
            rotary_dim: 2,
        },
        q_gate: p(0, 4),
        k: p(32, 2),
        v: p(48, 2),
        out: p(64, 2),
        q_norm: vector(80),
        k_norm: vector(88),
        norm: QkNorm::FoldedMultiplier,
    });
    let source = SliceBytes { bytes: &bytes };
    let host = CpuByteHost { source: &source };
    let sizes = plan.requirements(1).unwrap();
    assert_eq!(sizes.kv, 2);
    let mut workspace = vec![0.; sizes.workspace];
    let mut keys = vec![0.; sizes.kv];
    let mut values = vec![0.; sizes.kv];
    let mut hidden = [1., 2.];
    plan.execute(
        &host,
        &[1., 2.],
        &mut hidden,
        &mut [],
        &mut [],
        &mut keys,
        &mut values,
        0,
        &[1.],
        1e-6,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(hidden, [1.5, 3.]);
    assert_eq!(values, [1., 2.]);
    let original = (hidden, keys.clone(), values.clone());
    assert!(plan
        .execute(
            &host,
            &[3., 4.],
            &mut hidden,
            &mut [],
            &mut [],
            &mut keys,
            &mut values,
            1,
            &[1.],
            1e-6,
            &mut workspace
        )
        .is_err());
    assert_eq!((hidden, keys, values), original);
}
