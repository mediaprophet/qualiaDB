use super::*;
use crate::p64::build::{
    plan_into, write_into, BuildBudget, BuildInput, BuildProfile, TensorDescriptor,
};
use crate::p64::{
    IntegrityMode, P64TensorEntry, P64_FLAG_LAYER_MAJOR, P64_FLAG_LAYER_PACK,
    P64_FLAG_LAYER_SCHEDULE, P64_FLAG_LITTLE_ENDIAN,
};

const BUDGET: ModelPreparationBudget = ModelPreparationBudget {
    metadata_bytes: 4096,
    tensor_count: 8,
};

fn native(dtype: u16, dims: [u32; 4], payload: &[u8]) -> Vec<u8> {
    let tensors = [TensorDescriptor {
        name: b"unknown.tensor",
        role_id: 999,
        dtype,
        manifold_idx: 0,
        rank: 2,
        dimensions: dims,
        source_offset: 123,
        source_name_hash: 456,
        alt_dtype: 777,
        precision_views_mask: 13,
        alt_blob_offset: 321,
        blob: payload,
    }];
    let input = BuildInput {
        profile: BuildProfile::GgufLayerPacked,
        page_log2: 8,
        flags: P64_FLAG_LITTLE_ENDIAN
            | P64_FLAG_LAYER_MAJOR
            | P64_FLAG_LAYER_PACK
            | P64_FLAG_LAYER_SCHEDULE,
        header_reserved: [0; 20],
        hparams_bytes: &[0; 64],
        manifold_bytes: &[0; 64],
        tokenizer_bytes: b"opaque-tokenizer",
        tensors: &tensors,
    };
    let mut entries = [P64TensorEntry {
        name_offset: 0,
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 0,
        dimensions: [0; 4],
        blob_offset: 0,
        blob_size: 0,
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
    }];
    let plan = plan_into(
        input,
        &mut entries,
        BuildBudget {
            output_bytes: 8192,
            scratch_bytes: 64,
        },
    )
    .unwrap();
    let mut output = vec![0; plan.output_len()];
    write_into(&plan, &mut output).unwrap();
    output
}

#[test]
fn native_unknown_metadata_and_bytes_remain_authoritative() {
    let bytes = native(999, [4, 1, 0, 0], &[1, 2, 3, 4]);
    let prepared =
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, None).unwrap();
    assert_eq!(prepared.index().entries.len(), 1);
    let info = prepared.index().tensor_info(b"unknown.tensor").unwrap();
    let tensor = prepared.view().tensor(0).unwrap();
    assert_eq!(prepared.index().tensor_data_start, 0);
    assert_eq!(info.byte_offset, u64::from(tensor.entry().blob_offset));
    assert_eq!(info.ggml_type, 999);
    assert_eq!(tensor.entry().alt_dtype, 777);
    assert_eq!(tensor.entry().precision_views_mask, 13);
    assert_eq!(tensor.entry().alt_blob_offset, 321);
    assert_eq!(prepared.view().header().reserved, [0; 20]);
    assert_eq!(prepared.view().bytes(), bytes);
    assert!(matches!(
        crate::ggml::fetch_tensor_bytes(&bytes, 0, &info),
        Err(crate::ggml::ExecutionError::UnsupportedType)
    ));
}

#[test]
fn native_f32_rows_use_absolute_individual_tensor_bounds() {
    let bytes = native(0, [2, 1, 0, 0], &[0, 0, 128, 63, 0, 0, 0, 64]);
    let prepared =
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, None).unwrap();
    let info = prepared.index().tensor_info(b"unknown.tensor").unwrap();
    let raw = crate::ggml::fetch_tensor_bytes(&bytes, 0, &info).unwrap();
    let mut row = [0.0; 2];
    crate::ggml::dequant_matrix_row_into(raw, &info, 0, &mut row).unwrap();
    assert_eq!(row, [1.0, 2.0]);
}

#[test]
fn native_refuses_known_payload_shorter_than_shape_before_exposing_index() {
    let bytes = native(0, [4, 1, 0, 0], &[0; 4]);
    assert!(matches!(
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, None),
        Err(PrepareModelError::TensorPayload(0))
    ));
}

#[test]
fn native_metadata_admission_is_explicit_and_bounded() {
    let bytes = native(999, [4, 1, 0, 0], &[0; 4]);
    assert!(matches!(
        PreparedP64::<ModelHyperparams>::parse(
            &bytes,
            IntegrityMode::Full,
            ModelPreparationBudget {
                metadata_bytes: 0,
                ..BUDGET
            },
            None
        ),
        Err(PrepareModelError::BudgetExceeded)
    ));
    let metadata = ModelMetadata {
        hyperparams: ModelHyperparams {
            n_embd: 1,
            ..ModelHyperparams::default()
        },
        moe_routing: Default::default(),
        ple_config: None,
    };
    assert!(matches!(
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, Some(metadata)),
        Err(PrepareModelError::MetadataMismatch)
    ));
    let metadata = ModelMetadata {
        hyperparams: ModelHyperparams {
            ssm_state_size: 128,
            ..ModelHyperparams::default()
        },
        moe_routing: Default::default(),
        ple_config: Some(Qwen4ExpPleConfig::default()),
    };
    let prepared =
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, Some(metadata))
            .unwrap();
    assert_eq!(prepared.index().hyperparams.ssm_state_size, 128);
    assert_eq!(prepared.index().ple_config, metadata.ple_config);
}

#[test]
fn descriptor_offset_overflow_refuses_without_indexing() {
    let info = TensorInfo {
        dims: [2, 1, 0, 0],
        n_dims: 2,
        ggml_type: 0,
        byte_offset: u64::MAX,
    };
    assert_eq!(
        crate::ggml::fetch_tensor_bytes(&[], 1, &info),
        Err(crate::ggml::ExecutionError::MmapBounds)
    );
    assert_eq!(
        crate::ggml::fetch_token_embedding(&[], 1, &info, 0),
        Err(crate::ggml::ExecutionError::MmapBounds)
    );
}

#[test]
fn extension_preserves_complete_metadata_and_rejects_corruption() {
    let metadata = ModelMetadata {
        hyperparams: ModelHyperparams {
            ssm_conv_kernel: 4,
            ssm_state_size: 128,
            ssm_group_count: 2,
            ssm_time_step_rank: 16,
            ssm_inner_size: 256,
            full_attention_interval: 4,
            ..ModelHyperparams::default()
        },
        moe_routing: Default::default(),
        ple_config: Some(Qwen4ExpPleConfig {
            ngram_size: 3,
            head_offsets: [-7; 32],
            ..Qwen4ExpPleConfig::default()
        }),
    };
    let mut record = [0; MODEL_METADATA_EXTENSION_BYTES];
    assert_eq!(
        metadata.write_extension_into(&mut record).unwrap(),
        record.len()
    );
    assert_eq!(ModelMetadata::read_extension(&record).unwrap(), metadata);
    record[100] ^= 1;
    assert_eq!(
        ModelMetadata::read_extension(&record),
        Err(PrepareModelError::MetadataExtension)
    );
}

#[test]
fn native_extension_reopens_without_external_metadata_and_binds_unknown_hashes() {
    let metadata = ModelMetadata {
        hyperparams: ModelHyperparams {
            ssm_state_size: 128,
            ..ModelHyperparams::default()
        },
        moe_routing: Default::default(),
        ple_config: Some(Qwen4ExpPleConfig::default()),
    };
    let mut tokenizer = vec![1, 2, 3];
    let start = tokenizer.len();
    tokenizer.resize(start + MODEL_METADATA_EXTENSION_BYTES, 0);
    metadata
        .write_extension_into(&mut tokenizer[start..])
        .unwrap();
    let source_hash = tensor_name_hash(b"blk.0.ssm_a");
    let name = format!("tensor.{source_hash:016x}");
    let tensors = [TensorDescriptor {
        name: name.as_bytes(),
        role_id: 999,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [1, 0, 0, 0],
        source_offset: 0,
        source_name_hash: source_hash,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
        blob: &[0; 4],
    }];
    let mut bytes = native(0, [1, 1, 0, 0], &[0; 4]);
    let mut scratch = [crate::p64::P64View::parse(&bytes, IntegrityMode::Full)
        .unwrap()
        .tensor(0)
        .unwrap()
        .entry()];
    let plan = plan_into(
        BuildInput {
            profile: BuildProfile::GgufLayerPacked,
            page_log2: 8,
            flags: P64_FLAG_LITTLE_ENDIAN
                | P64_FLAG_LAYER_MAJOR
                | P64_FLAG_LAYER_PACK
                | P64_FLAG_LAYER_SCHEDULE,
            header_reserved: metadata_extension_locator(&tokenizer).unwrap(),
            hparams_bytes: &[0; 64],
            manifold_bytes: &[0; 64],
            tokenizer_bytes: &tokenizer,
            tensors: &tensors,
        },
        &mut scratch,
        BuildBudget {
            output_bytes: 8192,
            scratch_bytes: 64,
        },
    )
    .unwrap();
    bytes.resize(plan.output_len(), 0);
    write_into(&plan, &mut bytes).unwrap();
    let prepared =
        PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, None).unwrap();
    assert_eq!(prepared.index().hyperparams, metadata.hyperparams);
    assert_eq!(prepared.index().ple_config, metadata.ple_config);
    assert!(prepared.index().tensor_info(b"blk.0.ssm_a").is_some());
    assert!(prepared.index().tensor_info(name.as_bytes()).is_some());
    assert_eq!(
        prepared.view().tensor(0).unwrap().name_bytes(),
        name.as_bytes()
    );
}

#[test]
fn native_routing_extension_reopens_with_explicit_top6_and_top10() {
    for k in [6, 10] {
        let metadata = ModelMetadata {
            hyperparams: ModelHyperparams {
                ssm_state_size: 128,
                ..ModelHyperparams::default()
            },
            moe_routing: moe_routing::MoeRoutingSettings {
                expert_count: Some(256),
                experts_per_token: Some(k),
            },
            ple_config: Some(Qwen4ExpPleConfig::default()),
        };
        let mut tokenizer = vec![1, 2, 3];
        let start = tokenizer.len();
        tokenizer.resize(start + MODEL_ROUTING_EXTENSION_BYTES, 0);
        metadata
            .write_extension_into(&mut tokenizer[start..])
            .unwrap();
        let source_hash = tensor_name_hash(b"blk.0.ssm_a");
        let name = format!("tensor.{source_hash:016x}");
        let tensors = [TensorDescriptor {
            name: name.as_bytes(),
            role_id: 999,
            dtype: 0,
            manifold_idx: 0,
            rank: 1,
            dimensions: [1, 0, 0, 0],
            source_offset: 0,
            source_name_hash: source_hash,
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
            blob: &[0; 4],
        }];
        let mut bytes = native(0, [1, 1, 0, 0], &[0; 4]);
        let mut scratch = [crate::p64::P64View::parse(&bytes, IntegrityMode::Full)
            .unwrap()
            .tensor(0)
            .unwrap()
            .entry()];
        let plan = plan_into(
            BuildInput {
                profile: BuildProfile::GgufLayerPacked,
                page_log2: 8,
                flags: P64_FLAG_LITTLE_ENDIAN
                    | P64_FLAG_LAYER_MAJOR
                    | P64_FLAG_LAYER_PACK
                    | P64_FLAG_LAYER_SCHEDULE,
                header_reserved: metadata_extension_locator(&tokenizer).unwrap(),
                hparams_bytes: &[0; 64],
                manifold_bytes: &[0; 64],
                tokenizer_bytes: &tokenizer,
                tensors: &tensors,
            },
            &mut scratch,
            BuildBudget {
                output_bytes: 8192,
                scratch_bytes: 64,
            },
        )
        .unwrap();
        bytes.resize(plan.output_len(), 0);
        write_into(&plan, &mut bytes).unwrap();
        let prepared =
            PreparedP64::<ModelHyperparams>::parse(&bytes, IntegrityMode::Full, BUDGET, None)
                .unwrap();
        assert_eq!(prepared.index().hyperparams, metadata.hyperparams);
        assert_eq!(prepared.index().ple_config, metadata.ple_config);
        assert_eq!(prepared.index().moe_routing, metadata.moe_routing);
        assert_eq!(
            prepared
                .index()
                .moe_routing
                .admit(256)
                .unwrap()
                .experts_per_token(),
            k as usize
        );
        assert!(prepared.index().tensor_info(b"blk.0.ssm_a").is_some());
        assert!(prepared.index().tensor_info(name.as_bytes()).is_some());
        assert_eq!(
            prepared.view().tensor(0).unwrap().name_bytes(),
            name.as_bytes()
        );
    }
}
