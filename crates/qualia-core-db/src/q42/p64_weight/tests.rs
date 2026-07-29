use super::*;
use crate::gguf_sharder::GgufTensorIndex;

#[test]
fn p64_magic_sniff_is_exact_and_case_sensitive() {
    assert!(has_p64_magic(b"p64\0payload"));
    assert!(has_p64_magic(&P64_MAGIC));
    assert!(!has_p64_magic(b"P64\0payload"));
    assert!(!has_p64_magic(b"P64"));
    assert!(!has_p64_magic(b"p64"));
    assert!(!has_p64_magic(b"Q42\0payload"));
    assert!(!has_p64_magic(b"GGUFpayload"));
}

#[test]
fn p64_layer_major_flag_and_header_reserved_round_trip() {
    let gguf = synthetic_gguf();
    let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile");
    let index = P64TensorIndex::from_p64(&p64).expect("parse");
    assert_eq!(index.header.version, P64_VERSION);
    assert_ne!(index.header.flags & P64_FLAG_LAYER_MAJOR, 0);
    assert_eq!(
        index.header.flags & P64_FLAG_LITTLE_ENDIAN,
        P64_FLAG_LITTLE_ENDIAN
    );
    // Verbatim F32 synth has no Q4_K_SOA.
    assert_eq!(index.header.flags & P64_FLAG_Q4K_SOA, 0);
    // Known roles: layer tensors before globals (layer-major).
    let roles: Vec<u16> = index.entries.iter().map(|e| e.role_id).collect();
    let q_pos = roles.iter().position(|&r| r == P64_ROLE_ATTN_Q);
    let emb_pos = roles.iter().position(|&r| r == P64_ROLE_TOKEN_EMBD);
    assert!(q_pos.is_some() && emb_pos.is_some());
    assert!(
        q_pos.unwrap() < emb_pos.unwrap(),
        "layer-major: attn_q must precede token_embd, roles={roles:?}"
    );
    // Reserved I/O: write non-zero reserved, re-read.
    let mut hdr = index.header;
    hdr.reserved[0] = 0xAB;
    hdr.reserved[19] = 0xCD;
    let mut bytes = [0u8; 64];
    hdr.write_le(&mut bytes);
    let back = P64WeightHeader::read_le(&bytes).expect("read header");
    assert_eq!(back.reserved[0], 0xAB);
    assert_eq!(back.reserved[19], 0xCD);
}

fn put_kv_u32(out: &mut Vec<u8>, key: &str, value: u32) {
    out.extend_from_slice(&(key.len() as u64).to_le_bytes());
    out.extend_from_slice(key.as_bytes());
    out.extend_from_slice(&4u32.to_le_bytes()); // GGUF_TYPE_UINT32
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_tensor(out: &mut Vec<u8>, name: &str, dims: &[u64], offset: u64) {
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
    for dimension in dims {
        out.extend_from_slice(&dimension.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes()); // GGML F32
    out.extend_from_slice(&offset.to_le_bytes());
}

fn synthetic_gguf() -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"GGUF");
    output.extend_from_slice(&3u32.to_le_bytes());
    output.extend_from_slice(&2u64.to_le_bytes());
    output.extend_from_slice(&3u64.to_le_bytes());
    put_kv_u32(&mut output, "llama.block_count", 1);
    put_kv_u32(&mut output, "llama.embedding_length", 4);
    put_kv_u32(&mut output, "llama.attention.head_count", 1);
    put_tensor(&mut output, "blk.0.attn_q.weight", &[4, 4], 0);
    put_tensor(&mut output, "token_embd.weight", &[4, 2], 64);
    output.resize(align_up(output.len(), 32), 0);
    for byte in 0u8..96 {
        output.push(byte.wrapping_mul(17).wrapping_add(3));
    }
    output
}

fn synthetic_ffn_gguf() -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"GGUF");
    output.extend_from_slice(&3u32.to_le_bytes());
    output.extend_from_slice(&2u64.to_le_bytes());
    output.extend_from_slice(&3u64.to_le_bytes());
    put_kv_u32(&mut output, "llama.block_count", 1);
    put_kv_u32(&mut output, "llama.embedding_length", 32);
    put_kv_u32(&mut output, "llama.attention.head_count", 1);
    put_tensor(&mut output, "blk.0.ffn_gate.weight", &[32, 2], 0);
    put_tensor(&mut output, "token_embd.weight", &[32, 1], 256);
    output.resize(align_up(output.len(), 32), 0);
    for index in 0..64 {
        let value = if index % 2 == 0 { 1.25f32 } else { -0.75f32 };
        output.extend_from_slice(&value.to_le_bytes());
    }
    for index in 0..32 {
        output.extend_from_slice(&(index as f32 / 32.0).to_le_bytes());
    }
    output
}

fn synthetic_safetensor() -> Vec<u8> {
    let values = [1.0f32, -1.0, 0.5, -0.5, 2.0, -2.0, 0.25, -0.25];
    let mut ffn = Vec::new();
    let mut attention = Vec::new();
    for value in values {
        ffn.extend_from_slice(&half::f16::from_f32(value).to_le_bytes());
        attention.extend_from_slice(&half::f16::from_f32(value / 2.0).to_le_bytes());
    }
    let header = serde_json::json!({
        "model.layers.0.mlp.gate_proj.weight": {
            "dtype": "F16", "shape": [4, 2], "data_offsets": [0, ffn.len()]
        },
        "model.layers.0.self_attn.q_proj.weight": {
            "dtype": "F16", "shape": [4, 2],
            "data_offsets": [ffn.len(), ffn.len() + attention.len()]
        }
    });
    let header = serde_json::to_vec(&header).unwrap();
    let mut output = Vec::new();
    output.extend_from_slice(&(header.len() as u64).to_le_bytes());
    output.extend_from_slice(&header);
    output.extend_from_slice(&ffn);
    output.extend_from_slice(&attention);
    output
}

#[test]
fn gguf_to_p64_round_trip_is_byte_exact_and_cache_aligned() {
    let gguf = synthetic_gguf();
    let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile synthetic GGUF");
    let index = P64TensorIndex::from_p64(&p64).expect("validate P64");
    let report = index
        .validate_against_gguf(&p64, &gguf)
        .expect("full tensor parity");

    assert_eq!(report.tensor_count, 2);
    assert_eq!(report.tensor_bytes, 96);
    assert_eq!(report.manifold_count, 2);
    assert_eq!(index.header.manifold_table_offset as usize % 64, 0);
    assert_ne!(index.header.flags & P64_FLAG_LAYER_PACK, 0);
    assert_ne!(index.header.flags & P64_FLAG_LAYER_SCHEDULE, 0);
    // Layer-pack: first blob page-aligned; within-layer only needs 256 B.
    assert_eq!(index.entries[0].blob_offset as usize % 4096, 0);
    for entry in &index.entries {
        assert_eq!(entry.blob_offset as usize % 256, 0);
        assert_eq!(
            (index.header.manifold_table_offset as usize
                + entry.manifold_idx as usize * P64_MANIFOLD_ENTRY_BYTES)
                % 64,
            0
        );
    }
    let synthetic = index.to_gguf_index();
    assert_eq!(synthetic.hyperparams, index.hyperparams());
    assert!(synthetic.get_layer_tensors(0).attn_q.is_some());
    assert!(synthetic.token_embd_info().is_some());
}

#[test]
fn p64_rejects_metadata_and_tensor_corruption() {
    let gguf = synthetic_gguf();
    let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile");
    let index =
        P64TensorIndex::from_p64_with_integrity(&p64, IntegrityMode::Full).expect("baseline");

    let mut metadata_corrupt = p64.clone();
    metadata_corrupt[index.header.tensor_table_offset as usize + 12] ^= 1;
    assert!(
        P64TensorIndex::from_p64_with_integrity(&metadata_corrupt, IntegrityMode::Metadata)
            .is_err()
    );

    let mut tensor_corrupt = p64;
    tensor_corrupt[index.entries[0].blob_offset as usize] ^= 1;
    // Tensor CRC is only checked in Full mode.
    assert!(P64TensorIndex::from_p64_with_integrity(&tensor_corrupt, IntegrityMode::Full).is_err());
    // Metadata mode still accepts (bounds ok) — intentional fast-activate tradeoff.
    assert!(
        P64TensorIndex::from_p64_with_integrity(&tensor_corrupt, IntegrityMode::Metadata).is_ok()
    );
}

#[test]
fn p64_round_trips_after_filesystem_write() {
    let gguf = synthetic_gguf();
    let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile");
    let path = std::env::temp_dir().join(format!(
        "qualia-p64-roundtrip-{}-{}.p64",
        std::process::id(),
        p64.len()
    ));
    std::fs::write(&path, &p64).expect("write P64");
    let persisted = std::fs::read(&path).expect("read P64");
    let _ = std::fs::remove_file(&path);

    assert_eq!(persisted, p64, "filesystem changed P64 bytes");
    let index = P64TensorIndex::from_p64(&persisted).expect("parse persisted P64");
    index
        .validate_against_gguf(&persisted, &gguf)
        .expect("persisted tensor parity");
}

#[test]
fn ffn_quantized_p64_variants_are_loadable_and_preserve_non_ffn_weights() {
    let gguf = synthetic_ffn_gguf();
    let source = GgufTensorIndex::from_gguf(&gguf);

    for (quant, expected_type, expected_size) in [
        (
            FfnQuant::Ternary,
            crate::ternary::GGML_TYPE_TERNARY_158,
            crate::ternary::ternary_blob_len(64),
        ),
        (
            FfnQuant::Q4_0,
            crate::ggml_quants::GGML_TYPE_Q4_0,
            crate::llm_kernel_parity::q4_0_bytes(64),
        ),
    ] {
        let scales = [vec![1.0f32; 32]];
        let p64 = compile_gguf_to_q42_ffn_quant_awq(&gguf, 12, Some(&scales), 0.5, quant)
            .expect("quantized P64");
        let index = P64TensorIndex::from_p64(&p64).expect("load quantized P64");
        let ffn = index
            .entries
            .iter()
            .find(|entry| entry.role_id == P64_ROLE_FFN_GATE)
            .expect("FFN gate");
        assert_eq!(ffn.dtype as u32, expected_type);
        assert_eq!(ffn.blob_size as usize, expected_size);

        let token = index
            .entries
            .iter()
            .find(|entry| entry.role_id == P64_ROLE_TOKEN_EMBD)
            .expect("token embedding");
        let original = source.token_embd_info().unwrap();
        let source_start = source.tensor_data_start as usize + original.byte_offset as usize;
        let source_len = crate::ggml_quants::tensor_byte_len(original).unwrap();
        assert_eq!(
            index.blob(&p64, token),
            &gguf[source_start..source_start + source_len]
        );
    }
}

#[test]
fn safetensor_p64_variants_share_the_validated_container_contract() {
    let source = synthetic_safetensor();
    let source_plan = crate::safetensor::parse_safetensor_header(&source).unwrap();

    let mut verbatim = Vec::new();
    let report = transcode_safetensor_to_p64(&source, 12, &mut verbatim).unwrap();
    assert_eq!(report.n_tensors, 2);
    let index = P64TensorIndex::from_p64(&verbatim).expect("verbatim P64");
    for (entry, tensor) in index.entries.iter().zip(&source_plan.tensors) {
        let start = source_plan.data_start + tensor.begin;
        let end = source_plan.data_start + tensor.end;
        assert_eq!(index.blob(&verbatim, entry), &source[start..end]);
    }

    let mut all_ternary = Vec::new();
    transcode_safetensor_to_p64_ternary(&source, 12, &mut all_ternary).unwrap();
    let all_index = P64TensorIndex::from_p64(&all_ternary).expect("all-ternary P64");
    assert!(all_index
        .entries
        .iter()
        .all(|entry| entry.dtype as u32 == crate::ternary::GGML_TYPE_TERNARY_158));

    let mut policy = Vec::new();
    transcode_safetensor_to_p64_policy(&source, 12, &mut policy).unwrap();
    let policy_index = P64TensorIndex::from_p64(&policy).expect("policy P64");
    let ffn = policy_index
        .entries
        .iter()
        .find(|entry| entry.role_id == P64_ROLE_FFN_GATE)
        .unwrap();
    let attention = policy_index
        .entries
        .iter()
        .find(|entry| entry.role_id == P64_ROLE_ATTN_Q)
        .unwrap();
    assert_eq!(ffn.dtype as u32, crate::ternary::GGML_TYPE_TERNARY_158);
    assert_eq!(attention.dtype as u32, crate::safetensor::GGML_F16);
}

#[test]
fn real_smollm_p64_round_trip_on_disk() {
    let source_path = "C:/LLM_Models/GGUF/lmstudio-community/smollm2-360m-instruct-q8_0.gguf";
    if !std::path::Path::new(source_path).exists() {
        eprintln!("local SmolLM2 model absent; skipping");
        return;
    }
    let gguf = std::fs::read(source_path).expect("read real GGUF");
    let p64 = compile_gguf_to_p64(&gguf, 14).expect("compile real GGUF");
    let output_path =
        std::env::temp_dir().join(format!("qualia-smollm-p64-{}.p64", std::process::id()));
    std::fs::write(&output_path, &p64).expect("persist real P64");
    drop(p64);

    let file = std::fs::File::open(&output_path).expect("reopen P64");
    let persisted = unsafe { memmap2::Mmap::map(&file).expect("mmap P64") };
    let index = P64TensorIndex::from_p64(&persisted).expect("validate persisted P64");
    let report = index
        .validate_against_gguf(&persisted, &gguf)
        .expect("real model tensor parity");
    assert!(report.tensor_count > 100);
    assert!(report.tensor_bytes > 300_000_000);
    assert_eq!(report.manifold_count, index.hparams.n_layer as usize + 1);
    drop(persisted);
    let _ = std::fs::remove_file(&output_path);
    eprintln!(
        "real P64 parity: {} tensors / {} bytes / {} manifold coordinates",
        report.tensor_count, report.tensor_bytes, report.manifold_count
    );
}

#[test]
fn test_vision_tensors_p64_round_trip() {
    let t1 = RawVisionTensor {
        name: "conv1.weight".to_string(),
        shape: vec![1, 1, 3, 3],
        data: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        role_id: P64_ROLE_VISION_CONV2D,
    };
    let t2 = RawVisionTensor {
        name: "conv1.bias".to_string(),
        shape: vec![1],
        data: vec![0.5],
        role_id: P64_ROLE_VISION_BN,
    };

    let mut out = Vec::new();
    let report =
        transcode_vision_tensors_to_p64(&[t1, t2], 12, &mut out).expect("transcode vision");
    assert_eq!(report.n_tensors, 2);
    assert!(report.bytes_written > 0);

    assert!(has_p64_magic(&out));
    let index = P64TensorIndex::from_p64(&out).expect("parse vision p64");
    assert_eq!(index.entries.len(), 2);
    assert_eq!(index.entries[0].role_id, P64_ROLE_VISION_CONV2D);
    assert_eq!(index.entries[1].role_id, P64_ROLE_VISION_BN);
}
