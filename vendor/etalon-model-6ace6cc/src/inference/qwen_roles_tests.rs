use super::*;
use crate::ggml::{GGML_TYPE_F32, GGML_TYPE_FP8_E4M3};

fn h() -> ModelHyperparams {
    ModelHyperparams {
        n_embd: 5,
        n_layer: 1,
        n_head: 100,
        head_dim: 777,
        ssm_group_count: 2,
        ssm_time_step_rank: 4,
        ssm_state_size: 3,
        ssm_inner_size: 8,
        ssm_conv_kernel: 4,
        architecture: ARCH_QWEN35_MOE,
        ..Default::default()
    }
}

fn info(dims: &[u64], kind: u32) -> TensorInfo {
    let mut shape = [1; 4];
    shape[..dims.len()].copy_from_slice(dims);
    TensorInfo {
        dims: shape,
        n_dims: dims.len() as u32,
        ggml_type: kind,
        byte_offset: 0,
    }
}

fn put(index: &mut TensorIndex, suffix: &str, dims: &[u64], kind: u32) {
    let name = format!("model.layers.0.linear_attn.{suffix}");
    index.entries.push((
        super::super::tensor_name_hash(name.as_bytes()),
        info(dims, kind),
    ));
}

fn index(fused: bool) -> TensorIndex {
    let mut index = TensorIndex {
        entries: vec![],
        tensor_data_start: 0,
        token_embd: None,
        output_weight: None,
        output_norm: None,
        ple_ngram_embedding: None,
        moe_routing: Default::default(),
        ple_config: None,
        hyperparams: h(),
        max_tensor_bytes: 0,
        max_layer_tensor_bytes: 0,
    };
    if fused {
        put(&mut index, "in_proj_qkvz.weight", &[5, 28], GGML_TYPE_F32);
        put(&mut index, "in_proj_ba.weight", &[5, 8], GGML_TYPE_F32);
    } else {
        for (stem, rows) in [("qkv", 20), ("z", 8), ("a", 4), ("b", 4)] {
            put(
                &mut index,
                &format!("in_proj_{stem}.weight"),
                &[5, rows],
                GGML_TYPE_F32,
            );
        }
    }
    put(&mut index, "conv1d.weight", &[4, 1, 20], GGML_TYPE_F32);
    put(&mut index, "A_log", &[4], GGML_TYPE_F32);
    put(&mut index, "dt_bias", &[4], GGML_TYPE_F32);
    put(&mut index, "norm.weight", &[2], GGML_TYPE_F32);
    put(&mut index, "out_proj.weight", &[8, 5], GGML_TYPE_F32);
    index
}

#[test]
fn geometry_is_rectangular_and_independent_of_attention() {
    let g = Qwen35Geometry::from_hyperparams(h()).unwrap();
    assert_eq!(
        (g.key_dim, g.value_dim, g.qk_width, g.value_width),
        (3, 2, 6, 8)
    );
    assert_eq!((g.channels, g.history_floats, g.state_floats), (20, 60, 24));
    let mut local = h();
    local.n_embd = 2048;
    local.ssm_group_count = 16;
    local.ssm_time_step_rank = 32;
    local.ssm_state_size = 128;
    local.ssm_inner_size = 4096;
    let g = Qwen35Geometry::from_hyperparams(local).unwrap();
    assert_eq!((g.history_floats + g.state_floats) * 4, 2_195_456);
}

#[test]
fn malformed_and_overflow_geometry_fails_closed() {
    for field in 0..6 {
        let mut p = h();
        match field {
            0 => p.n_embd = 0,
            1 => p.ssm_group_count = 0,
            2 => p.ssm_time_step_rank = 0,
            3 => p.ssm_state_size = 0,
            4 => p.ssm_inner_size = 0,
            _ => p.ssm_conv_kernel = 0,
        }
        assert_eq!(
            Qwen35Geometry::from_hyperparams(p),
            Err(Qwen35RoleError::Geometry)
        );
    }
    let mut p = h();
    p.ssm_inner_size = 9;
    assert_eq!(
        Qwen35Geometry::from_hyperparams(p),
        Err(Qwen35RoleError::Geometry)
    );
    p = h();
    p.ssm_group_count = 3;
    assert_eq!(
        Qwen35Geometry::from_hyperparams(p),
        Err(Qwen35RoleError::Geometry)
    );
    p = h();
    p.ssm_group_count = u32::MAX;
    p.ssm_time_step_rank = u32::MAX;
    p.ssm_state_size = u32::MAX;
    p.ssm_inner_size = u32::MAX;
    assert_eq!(
        Qwen35Geometry::from_hyperparams(p),
        Err(Qwen35RoleError::Overflow)
    );
}

#[test]
fn distinct_separate_projection_and_scalar_roles() {
    let roles = Qwen35Roles::resolve(&index(false), 0, Qwen35Packing::Separate).unwrap();
    let Qwen35Projections::Separate { qkv, z, a, b } = roles.projections else {
        panic!()
    };
    assert_eq!((qkv.rows(), z.rows(), a.rows(), b.rows()), (20, 8, 4, 4));
    assert_eq!(a.weight.n_dims, 2);
    assert_eq!(roles.a_log.n_dims, 1);
    assert_eq!(roles.norm.dims[0], 2);
}

#[test]
fn fused_packing_is_explicit_and_mixed_profiles_refused() {
    for packing in [
        Qwen35Packing::FusedGlobal,
        Qwen35Packing::FusedKeyHeadGrouped,
    ] {
        let mut i = index(true);
        let roles = Qwen35Roles::resolve(&i, 0, packing).unwrap();
        assert!(
            matches!(roles.projections, Qwen35Projections::Fused { packing: p, .. } if p == packing)
        );
        put(&mut i, "in_proj_a.weight", &[5, 4], GGML_TYPE_F32);
        assert_eq!(
            Qwen35Roles::resolve(&i, 0, packing),
            Err(Qwen35RoleError::MixedProjectionProfile)
        );
    }
    assert_eq!(
        Qwen35Roles::resolve(&index(true), 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::MixedProjectionProfile)
    );
}

#[test]
fn fp8_preserves_exact_row_scale_and_requires_compatible_metadata() {
    let mut i = index(false);
    i.entries[0].1.ggml_type = GGML_TYPE_FP8_E4M3;
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::MissingRowScale)
    );
    put(&mut i, "in_proj_qkv.weight_scale", &[20], GGML_TYPE_F32);
    let roles = Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate).unwrap();
    let Qwen35Projections::Separate { qkv, .. } = roles.projections else {
        panic!()
    };
    assert_eq!(qkv.row_scale, Some(info(&[20], GGML_TYPE_F32)));
    let last = i.entries.len() - 1;
    i.entries[last].1.dims[0] = 1;
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::IncompatibleRowScale)
    );
    i.entries[last].1 = info(&[20], GGML_TYPE_FP8_E4M3);
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::IncompatibleRowScale)
    );
    i.entries[0].1.ggml_type = GGML_TYPE_F32;
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::IncompatibleRowScale)
    );
}

#[test]
fn required_shape_encoding_and_alias_conflicts_are_errors() {
    let mut i = index(false);
    i.entries[0].1.dims[0] = 6;
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::Shape)
    );
    i.entries[0].1 = info(&[5, 20], 999);
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::UnsupportedEncoding)
    );
    i.entries[0].1 = info(&[5, 20], GGML_TYPE_F32);
    let hash = super::super::tensor_name_hash(b"blk.0.linear_attn.in_proj_qkv.weight");
    i.entries.push((hash, info(&[5, 21], GGML_TYPE_F32)));
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::AmbiguousTensor)
    );
    let i = index(false);
    assert_eq!(
        Qwen35Roles::resolve(&i, 1, Qwen35Packing::Separate),
        Err(Qwen35RoleError::MissingTensor)
    );
}

#[test]
fn caller_constructed_descriptors_are_revalidated() {
    let roles = Qwen35Roles::resolve(&index(false), 0, Qwen35Packing::Separate).unwrap();
    assert_eq!(roles.validate(), Ok(()));
    let mut bad = roles;
    bad.geometry.channels += 1;
    assert_eq!(bad.validate(), Err(Qwen35RoleError::Geometry));
    bad = roles;
    bad.a_log.n_dims = 2;
    assert_eq!(bad.validate(), Err(Qwen35RoleError::Shape));
    let mut fused = Qwen35Roles::resolve(&index(true), 0, Qwen35Packing::FusedGlobal).unwrap();
    let Qwen35Projections::Fused {
        ref mut packing, ..
    } = fused.projections
    else {
        panic!()
    };
    *packing = Qwen35Packing::Separate;
    assert_eq!(
        fused.validate(),
        Err(Qwen35RoleError::MixedProjectionProfile)
    );
    let mut i = index(false);
    let (hash, mut duplicate) = i.entries[0];
    duplicate.byte_offset = 42;
    i.entries.push((hash, duplicate));
    assert_eq!(
        Qwen35Roles::resolve(&i, 0, Qwen35Packing::Separate),
        Err(Qwen35RoleError::AmbiguousTensor)
    );
}
