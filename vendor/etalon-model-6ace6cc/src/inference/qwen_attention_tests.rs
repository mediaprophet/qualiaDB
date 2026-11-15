use super::*;

fn geometry() -> Qwen35AttentionGeometry {
    Qwen35AttentionGeometry {
        hidden: 3,
        query_heads: 2,
        kv_heads: 1,
        head_dim: 6,
        rotary_dim: 4,
    }
}

fn index() -> TensorIndex {
    TensorIndex {
        entries: vec![],
        tensor_data_start: 0,
        token_embd: None,
        output_weight: None,
        output_norm: None,
        ple_ngram_embedding: None,
        ple_config: None,
        moe_routing: Default::default(),
        max_tensor_bytes: 0,
        max_layer_tensor_bytes: 0,
        hyperparams: ModelHyperparams {
            architecture: ARCH_QWEN35_MOE,
            n_embd: 3,
            n_head: 2,
            n_kv_head: 1,
            head_dim: 6,
            n_layer: 1,
            ..Default::default()
        },
    }
}

#[test]
fn resolver_rejects_inconsistent_metadata_before_missing_roles() {
    for field in 0..5 {
        let mut i = index();
        match field {
            0 => i.hyperparams.n_embd = 4,
            1 => i.hyperparams.n_head = 4,
            2 => i.hyperparams.n_kv_head = 2,
            3 => i.hyperparams.head_dim = 8,
            _ => i.hyperparams.n_layer = 0,
        }
        assert_eq!(
            Qwen35AttentionRoles::resolve(&i, 0, geometry(), QkNorm::RawZeroCentered),
            Err(Qwen35RoleError::Geometry)
        );
    }
    assert_eq!(
        Qwen35AttentionRoles::resolve(&index(), 1, geometry(), QkNorm::RawZeroCentered),
        Err(Qwen35RoleError::Geometry)
    );
}

#[test]
fn geometry_overflow_and_invalid_rotary_fail_closed() {
    let mut g = geometry();
    g.hidden = usize::MAX;
    assert_eq!(g.validate(), Err(Qwen35RoleError::Overflow));
    for rotary in [0, 3, 8] {
        let mut g = geometry();
        g.rotary_dim = rotary;
        assert_eq!(g.validate(), Err(Qwen35RoleError::Geometry));
    }
}

#[test]
fn resolver_requires_roles_and_rejects_each_projection_bias() {
    assert_eq!(
        Qwen35AttentionRoles::resolve(&index(), 0, geometry(), QkNorm::RawZeroCentered),
        Err(Qwen35RoleError::MissingTensor)
    );
    for stem in ["q_proj", "k_proj", "v_proj", "o_proj"] {
        let mut i = index();
        let name = format!("model.layers.0.self_attn.{stem}.bias");
        i.entries.push((
            super::super::tensor_name_hash(name.as_bytes()),
            TensorInfo {
                dims: [1, 1, 1, 1],
                n_dims: 1,
                ggml_type: 0,
                byte_offset: 0,
            },
        ));
        assert_eq!(
            Qwen35AttentionRoles::resolve(&i, 0, geometry(), QkNorm::FoldedMultiplier),
            Err(Qwen35RoleError::MixedProjectionProfile)
        );
    }
}
