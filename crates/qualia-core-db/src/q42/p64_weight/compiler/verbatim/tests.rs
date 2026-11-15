use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};
use crate::p64_weight::*;
use etalon_model::p64::build::TensorDescriptor;

fn string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}
fn fixture() -> Vec<u8> {
    let mut b = b"GGUF".to_vec();
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(&2u64.to_le_bytes());
    b.extend_from_slice(&1u64.to_le_bytes());
    string(&mut b, "llama.block_count");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    for (name, offset) in [("x.unknown", 32u64), ("blk.0.attn_q.weight", 0)] {
        string(&mut b, name);
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&2u64.to_le_bytes());
        b.extend_from_slice(&1u64.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&offset.to_le_bytes());
    }
    b.resize((b.len() + 31) & !31, 0);
    let start = b.len();
    b.resize(start + 40, 0);
    for (offset, value) in [(0, 1f32), (4, -2.), (32, 3.), (36, 4.)] {
        b[start + offset..start + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    b
}
fn descriptor() -> TensorDescriptor<'static> {
    TensorDescriptor {
        name: &[],
        role_id: 0,
        dtype: 0,
        manifold_idx: 0,
        rank: 1,
        dimensions: [0; 4],
        source_offset: 0,
        source_name_hash: 0,
        alt_dtype: 0,
        precision_views_mask: 0,
        alt_blob_offset: 0,
        blob: &[],
    }
}
fn entry() -> etalon_model::p64::P64TensorEntry {
    etalon_model::p64::P64TensorEntry {
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
    }
}

#[test]
fn borrowed_output_matches_wrapper_and_preserves_tail() {
    let input = fixture();
    let index = GgufTensorIndex::from_gguf(&input);
    let tokenizer = GgufTokenizer::from_gguf(&input).to_p64_section();
    let expected = compile_gguf_to_p64(&input, 9).unwrap();
    let r = gguf_verbatim_requirements(&index).unwrap();
    let mut order = vec![GgufCompileTensor::default(); r.tensor_capacity];
    let mut descriptors = vec![descriptor(); r.tensor_capacity];
    let mut entries = vec![entry(); r.tensor_capacity];
    let mut manifolds = vec![0; r.manifold_bytes];
    let mut hparams = [0; 64];
    let mut output = vec![0xa5; expected.len() + 7];
    let length = compile_gguf_index_to_p64_into(
        &input,
        &index,
        &tokenizer,
        9,
        &mut output,
        GgufCompileWorkspace {
            order: &mut order,
            descriptors: &mut descriptors,
            entries: &mut entries,
            manifolds: &mut manifolds,
            hparams: &mut hparams,
        },
        GgufCompileBudget {
            output_bytes: expected.len(),
            scratch_bytes: r.scratch_bytes,
        },
    )
    .unwrap();
    assert_eq!(length, expected.len());
    assert_eq!(&output[..length], expected);
    assert_eq!(&output[length..], &[0xa5; 7]);
}

#[test]
fn budget_capacity_and_source_refusals_preserve_output() {
    let input = fixture();
    for mode in 0..4 {
        let mut index = GgufTensorIndex::from_gguf(&input);
        if mode == 3 {
            index.tensor_data_start = u64::MAX;
        }
        let tokenizer = GgufTokenizer::from_gguf(&input).to_p64_section();
        let r = gguf_verbatim_requirements(&index).unwrap();
        let mut order = vec![GgufCompileTensor::default(); r.tensor_capacity];
        let mut descriptors = vec![descriptor(); r.tensor_capacity];
        let mut entries = vec![entry(); r.tensor_capacity];
        let mut manifolds = vec![0xcc; r.manifold_bytes];
        let mut hparams = [0xcc; 64];
        let mut output = [0xa5; 16];
        let result = compile_gguf_index_to_p64_into(
            &input,
            &index,
            &tokenizer,
            9,
            &mut output,
            GgufCompileWorkspace {
                order: &mut order,
                descriptors: &mut descriptors,
                entries: &mut entries,
                manifolds: &mut manifolds,
                hparams: &mut hparams,
            },
            GgufCompileBudget {
                output_bytes: if mode == 1 { 0 } else { u32::MAX as usize },
                scratch_bytes: r.scratch_bytes - usize::from(mode == 0),
            },
        );
        assert!(result.is_err());
        assert_eq!(output, [0xa5; 16]);
        if mode == 0 {
            assert!(matches!(
                result,
                Err(GgufCompileError::ScratchBudget { .. })
            ));
            assert_eq!(hparams, [0xcc; 64]);
        }
        if mode == 2 {
            assert!(matches!(result, Err(GgufCompileError::Writing(_))));
        }
    }
}

#[test]
fn legacy_source_validation_and_page_errors_stay_precise() {
    let input = fixture();
    assert_eq!(
        compile_gguf_to_p64(&input, 7).unwrap_err(),
        "p64: page_log2 7 out of range"
    );
    assert_eq!(
        compile_gguf_to_p64(&input[..input.len() - 1], 8).unwrap_err(),
        "p64: source tensor 1 is out of bounds"
    );
    assert_eq!(
        compile_gguf_to_p64(b"bad", 8).unwrap_err(),
        "p64: GGUF parse yielded no tensors"
    );
}

#[test]
fn fixed_workspace_can_be_inspected_and_reinitialized_for_second_compile() {
    let input = fixture();
    let index = GgufTensorIndex::from_gguf(&input);
    let tokenizer = GgufTokenizer::from_gguf(&input).to_p64_section();
    let required = gguf_verbatim_requirements(&index).unwrap();
    assert_eq!(required.tensor_capacity, 5);
    let expected = compile_gguf_to_p64(&input, 9).unwrap();
    let mut output = vec![0; expected.len()];
    let mut order = [GgufCompileTensor::default(); 5];
    let mut descriptors = [descriptor(); 5];
    let address = descriptors.as_ptr() as usize;
    let mut entries = [entry(); 5];
    let mut manifolds = [0; 128];
    let mut hparams = [0; 64];
    macro_rules! compile {
        () => {
            compile_gguf_index_to_p64_into(
                &input,
                &index,
                &tokenizer,
                9,
                &mut output,
                GgufCompileWorkspace {
                    order: &mut order,
                    descriptors: &mut descriptors,
                    entries: &mut entries,
                    manifolds: &mut manifolds,
                    hparams: &mut hparams,
                },
                GgufCompileBudget {
                    output_bytes: expected.len(),
                    scratch_bytes: required.scratch_bytes,
                },
            )
        };
    }
    compile!().unwrap();
    assert_eq!(descriptors[0].name, b"blk.0.attn_q.weight");
    assert!(descriptors[2..].iter().all(|entry| entry.name.is_empty()));
    assert_eq!(output, expected);
    // Whole-array assignment ends retained name references; .fill would not.
    descriptors = [descriptor(); 5];
    assert_eq!(descriptors.as_ptr() as usize, address);
    output.fill(0);
    compile!().unwrap();
    assert_eq!(descriptors[0].name, b"blk.0.attn_q.weight");
    assert_eq!(output, expected);
}

#[test]
fn vec_backed_prepared_plan_supports_repeated_writes_after_capacity_refusal() {
    let input = fixture();
    let index = GgufTensorIndex::from_gguf(&input);
    let tokenizer = GgufTokenizer::from_gguf(&input).to_p64_section();
    let expected = compile_gguf_to_p64(&input, 9).unwrap();
    let r = gguf_verbatim_requirements(&index).unwrap();
    let mut order = vec![GgufCompileTensor::default(); r.tensor_capacity];
    let mut descriptors = vec![descriptor(); r.tensor_capacity];
    let mut entries = vec![entry(); r.tensor_capacity];
    let mut manifolds = vec![0; r.manifold_bytes];
    let mut hparams = vec![0; 64];
    let mut output = vec![0xa5; expected.len() + 7];
    let plan = prepare_gguf_index_to_p64(
        &input,
        &index,
        &tokenizer,
        9,
        GgufCompileWorkspace {
            order: &mut order,
            descriptors: &mut descriptors,
            entries: &mut entries,
            manifolds: &mut manifolds,
            hparams: &mut hparams,
        },
        GgufCompileBudget {
            output_bytes: expected.len(),
            scratch_bytes: r.scratch_bytes,
        },
    )
    .unwrap();
    assert_eq!(plan.output_len(), expected.len());
    let mut short = [0xa5; 7];
    assert!(plan.write_into(&mut short).is_err());
    assert_eq!(short, [0xa5; 7]);
    for _ in 0..2 {
        output.fill(0xa5);
        assert_eq!(plan.write_into(&mut output).unwrap(), expected.len());
        assert_eq!(&output[..expected.len()], expected);
        assert_eq!(&output[expected.len()..], &[0xa5; 7]);
    }
    drop(plan);
    assert_eq!(descriptors[0].name, b"blk.0.attn_q.weight");
    assert!(descriptors[2..].iter().all(|entry| entry.name.is_empty()));
}
