use super::*;

fn fixture() -> (tempfile::TempDir, FtwModelPackage) {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = vec![0x22; 16]; // Sixteen weights of 1 per row.
    bytes.extend([0x38, 0x38]); // E4M3 scales of one.
    bytes.extend(half::f16::from_f32(2.0).to_le_bytes());
    bytes.extend(half::f16::from_f32(3.0).to_le_bytes());
    let tensors = serde_json::json!([
        {"name":"head.weight","kind":"weight","dtype":"uint8","shape":[2,8],"global_off":0,"nbytes":16},
        {"name":"head.weight_scale","kind":"weight","dtype":"float8_e4m3fn","shape":[2,1],"global_off":16,"nbytes":2},
        {"name":"head.weight_global","kind":"weight","dtype":"float16","shape":[2],"global_off":18,"nbytes":4}
    ]);
    std::fs::write(dir.path().join("part.ftw"), &bytes).unwrap();
    std::fs::write(dir.path().join("freetoken_weight.json"),serde_json::to_vec(&serde_json::json!({"format":"freetoken_weight","version":1,"total_bytes":bytes.len(),"tensors":tensors,"shards":[{"file":"part.ftw","global_off":0,"nbytes":bytes.len()}]})).unwrap()).unwrap();
    std::fs::write(dir.path().join("config.json"),r#"{"model_type":"qwen2","text_config":{"hidden_size":16,"head_dim":8,"num_hidden_layers":1,"num_attention_heads":2,"num_key_value_heads":1}}"#).unwrap();
    let pkg = FtwModelPackage::open_from_dir(dir.path()).unwrap();
    (dir, pkg)
}

#[test]
fn row_global_scaling_and_argmax_match_independent_dot() {
    let (_dir, pkg) = fixture();
    let projection = Nvfp4Projection::admit(&pkg, "head", 2, 16).unwrap();
    let mut row = [0.0; 16];
    projection.row(&pkg, 0, &mut row).unwrap();
    assert_eq!(row, [2.0; 16]);
    let mut out = [0.0; 2];
    let guard =
        crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard::begin(
            "qwen_ftw_nvfp4_projection",
            true,
        );
    projection.gemv(&pkg, &[1.0; 16], &mut out).unwrap();
    assert_eq!(out, [32.0, 48.0]);
    assert_eq!(projection.argmax(&pkg, &[1.0; 16]).unwrap(), (1, 48.0));
    guard
        .check()
        .expect("NVFP4 projection must perform zero allocation calls");
}

#[test]
fn malformed_scale_and_global_metadata_fail_admission() {
    let (_dir, mut pkg) = fixture();
    pkg.manifest.tensors[1].shape[1] = 2;
    assert!(matches!(
        Nvfp4Projection::admit(&pkg, "head", 2, 16),
        Err(WeightError::Shape)
    ));
    pkg.manifest.tensors[1].shape[1] = 1;
    pkg.manifest.tensors[2].dtype = "float32".into();
    assert!(matches!(
        Nvfp4Projection::admit(&pkg, "head", 2, 16),
        Err(WeightError::Encoding)
    ));
}

#[test]
fn unknown_receipt_cannot_admit_same_architecture_names() {
    let (dir, pkg) = fixture();
    let result =
        super::super::admission::admit_known(std::sync::Arc::new(pkg), dir.path(), 1, 1024 * 1024);
    assert!(result.is_err());
}
