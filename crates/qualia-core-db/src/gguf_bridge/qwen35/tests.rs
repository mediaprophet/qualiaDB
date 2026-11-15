use super::*;
use crate::gguf_bridge::ModelByteHandle;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
use std::sync::Arc;

fn fixture() -> (QTensorEngine, ModelByteHandle) {
    let mut engine = QTensorEngine::new();
    let mut index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&[]);
    index.hyperparams.n_layer = 1;
    index.hyperparams.n_embd = 1;
    index.hyperparams.architecture = etalon_model::inference::ARCH_QWEN35_MOE;
    index.hyperparams.ssm_group_count = 1;
    index.hyperparams.ssm_time_step_rank = 1;
    index.hyperparams.ssm_state_size = 1;
    index.hyperparams.ssm_inner_size = 1;
    index.hyperparams.ssm_conv_kernel = 2;
    let mut bytes = vec![0xAA; 16];
    for (name, dims, values) in [
        ("linear_attn.in_proj_qkvz.weight", vec![1, 4], vec![0.1; 4]),
        ("linear_attn.in_proj_ba.weight", vec![1, 2], vec![0.2; 2]),
        ("linear_attn.conv1d.weight", vec![2, 1, 3], vec![0.3; 6]),
        ("linear_attn.A_log", vec![1], vec![-0.5]),
        ("linear_attn.dt_bias", vec![1], vec![0.1]),
        ("linear_attn.norm.weight", vec![1], vec![1.0]),
        ("linear_attn.out_proj.weight", vec![1, 1], vec![0.2]),
        ("input_layernorm.weight", vec![1], vec![1.0]),
    ] {
        let mut shape = [1; 4];
        shape[..dims.len()].copy_from_slice(&dims);
        let info = TensorInfo {
            dims: shape,
            n_dims: dims.len() as u32,
            ggml_type: 0,
            byte_offset: (bytes.len() - 16) as u64,
        };
        let full_name = format!("model.layers.0.{name}");
        index
            .entries
            .push((etalon_model::gguf::fnv1a64(full_name.as_bytes()), info));
        for value in values {
            bytes.extend_from_slice(&f32::to_le_bytes(value));
        }
    }
    let mut mmap = memmap2::MmapMut::map_anon(bytes.len()).unwrap();
    mmap.copy_from_slice(&bytes);
    let handle = ModelByteHandle::snapshot(Arc::new(mmap.make_read_only().unwrap()));
    engine.tensor_data_offset = 16;
    engine.tensor_index_cache = Some(index);
    (engine, handle)
}
fn profile(norm: OrdinaryNorm) -> Profile {
    Profile {
        packing: Qwen35Packing::FusedGlobal,
        ordinary_norm: norm,
        epsilon: 1e-6,
    }
}
#[test]
fn actual_retained_source_steps_without_allocation_and_preserves_error_state() {
    let (engine, handle) = fixture();
    let mut layer = engine
        .prepare_qwen35_with_source(
            0,
            profile(OrdinaryNorm::AlreadyFolded),
            4096,
            RetainedBytes::test_snapshot(handle, 16),
        )
        .unwrap();
    drop(engine);
    let mut hidden = [1.0];
    assert_zero_alloc("retained Qwen linear byte host", || {
        layer.step(&mut hidden).unwrap()
    });
    assert!(hidden[0] > 1.0);
    let state = layer.recurrent_state().to_vec();
    let history = layer.convolution_history().to_vec();
    hidden[0] = f32::MAX;
    assert!(layer.step(&mut hidden).is_err());
    assert_eq!(hidden, [f32::MAX]);
    assert_eq!(layer.recurrent_state(), state);
    assert_eq!(layer.convolution_history(), history);
}
#[test]
fn budget_and_explicit_norm_contract_are_enforced() {
    let (engine, handle) = fixture();
    assert!(matches!(
        engine.prepare_qwen35_with_source(
            0,
            profile(OrdinaryNorm::AlreadyFolded),
            1,
            RetainedBytes::test_snapshot(handle.clone(), 16)
        ),
        Err(Error::Budget)
    ));
    let mut folded = engine
        .prepare_qwen35_with_source(
            0,
            profile(OrdinaryNorm::AlreadyFolded),
            4096,
            RetainedBytes::test_snapshot(handle.clone(), 16),
        )
        .unwrap();
    let mut raw = engine
        .prepare_qwen35_with_source(
            0,
            profile(OrdinaryNorm::RawZeroCentered),
            4096,
            RetainedBytes::test_snapshot(handle, 16),
        )
        .unwrap();
    let mut a = [1.0];
    let mut b = [1.0];
    folded.step(&mut a).unwrap();
    raw.step(&mut b).unwrap();
    assert_ne!(a, b);
    assert!(matches!(
        engine.prepare_qwen35_linear_layer(0, profile(OrdinaryNorm::AlreadyFolded), 4096),
        Err(Error::MissingSource)
    ));
}

#[cfg(windows)]
#[test]
fn public_ftw_lease_budget_custody_mismatch_and_reset() {
    use crate::inference::moe::ftw_loader::FtwModelPackage;
    use etalon_model_io::lease::{LeaseLimits, ModelLease};
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;
    let (mut engine, payload) = fixture();
    for (_, info) in &mut engine.tensor_index_cache.as_mut().unwrap().entries {
        info.byte_offset += 16;
    }
    engine.tensor_data_offset = 0;
    engine
        .tensor_index_cache
        .as_mut()
        .unwrap()
        .tensor_data_start = 0;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("layer.ftw");
    std::fs::write(&path, &*payload).unwrap();
    std::fs::write(temp.path().join("freetoken_weight.json"), format!(
        r#"{{"format":"freetoken_weight","version":1,"total_bytes":{},"tensors":[],"shards":[{{"file":"layer.ftw","global_off":0,"nbytes":{}}}]}}"#,
        payload.len(), payload.len())).unwrap();
    std::fs::write(temp.path().join("config.json"),
        r#"{"model_type":"qwen2","hidden_size":1,"num_hidden_layers":1,"num_attention_heads":1,"num_key_value_heads":1}"#).unwrap();
    let mut package = FtwModelPackage::open_from_dir(temp.path()).unwrap();
    package.tensor_index = engine.tensor_index_cache.as_ref().unwrap().clone();
    assert!(package.fetch_by_global_offset(u64::MAX, 4).is_none());
    let mut invalid = package.clone();
    invalid.manifest.shards[0].global_off = u64::MAX - 1;
    assert!(invalid.fetch_by_global_offset(u64::MAX - 1, 1).is_none());
    engine.ftw_package = Some(Arc::new(package));
    let lease = Arc::new(
        ModelLease::open(
            &path,
            Sha256::digest(&*payload).into(),
            LeaseLimits {
                max_source_bytes: 4096,
                max_mapped_bytes: 4096,
                max_total_bytes: 8192,
            },
            &AtomicBool::new(false),
        )
        .unwrap(),
    );
    let p = profile(OrdinaryNorm::AlreadyFolded);
    let shards = [lease.clone()];
    let owners = Arc::strong_count(&lease);
    let extra_shards = [lease.clone(), lease.clone()];
    let mismatched_owners = Arc::strong_count(&lease);
    assert!(matches!(
        engine.prepare_qwen35_linear_from_leases(0, p, 1 << 20, lease.clone(), &extra_shards),
        Err(Error::MissingSource)
    ));
    assert_eq!(Arc::strong_count(&lease), mismatched_owners);
    drop(extra_shards);
    assert_eq!(Arc::strong_count(&lease), owners);
    assert!(matches!(
        engine.prepare_qwen35_linear_from_leases(0, p, 1, lease.clone(), &shards),
        Err(Error::Budget)
    ));
    assert_eq!(Arc::strong_count(&lease), owners);
    engine
        .tensor_index_cache
        .as_mut()
        .unwrap()
        .hyperparams
        .n_head = 2;
    assert!(matches!(
        engine.prepare_qwen35_linear_from_leases(0, p, 1 << 20, lease.clone(), &shards),
        Err(Error::MissingSource)
    ));
    assert_eq!(Arc::strong_count(&lease), owners);
    engine
        .tensor_index_cache
        .as_mut()
        .unwrap()
        .hyperparams
        .n_head = 0;
    let mut layer = engine
        .prepare_qwen35_linear_from_leases(0, p, 1 << 20, lease.clone(), &shards)
        .unwrap();
    let required = layer.reserved_bytes();
    drop(layer);
    assert_eq!(Arc::strong_count(&lease), owners);
    assert!(matches!(
        engine.prepare_qwen35_linear_from_leases(0, p, required - 1, lease.clone(), &shards),
        Err(Error::Budget)
    ));
    assert_eq!(Arc::strong_count(&lease), owners);
    layer = engine
        .prepare_qwen35_linear_from_leases(0, p, required, lease.clone(), &shards)
        .unwrap();
    assert_eq!(layer.reserved_bytes(), required);
    drop(engine);
    drop(shards);
    assert!(Arc::strong_count(&lease) > 1);
    let mut first = [1.0];
    layer.step(&mut first).unwrap();
    layer.reset();
    let mut replay = [1.0];
    layer.step(&mut replay).unwrap();
    assert_eq!(first, replay);
    drop(layer);
    assert_eq!(Arc::strong_count(&lease), 1);
    drop(lease);
    std::fs::OpenOptions::new().write(true).open(&path).unwrap();
}
