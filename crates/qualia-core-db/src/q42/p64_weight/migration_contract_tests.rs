//! Consumer tests use the real Qualia compiler and public owned reader APIs.
use super::*;

fn source_gguf() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GGUF");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&2u64.to_le_bytes());
    out.extend_from_slice(&3u64.to_le_bytes());
    for (name, value) in [
        ("llama.block_count", 1u32),
        ("llama.embedding_length", 4),
        ("llama.attention.head_count", 1),
    ] {
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    for (name, dims, offset) in [
        ("blk.0.attn_q.weight", [4u64, 4], 0u64),
        ("token_embd.weight", [4, 2], 64),
    ] {
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        for dimension in dims {
            out.extend_from_slice(&dimension.to_le_bytes());
        }
        out.extend_from_slice(&0u32.to_le_bytes()); // F32, verbatim bytes.
        out.extend_from_slice(&offset.to_le_bytes());
    }
    out.resize((out.len() + 31) & !31, 0);
    for byte in 0u8..96 {
        out.push(byte.wrapping_mul(17).wrapping_add(3));
    }
    out
}

fn fixture() -> Vec<u8> {
    compile_gguf_to_p64(&source_gguf(), 8).unwrap()
}
fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn error(bytes: &[u8], mode: IntegrityMode) -> String {
    match P64TensorIndex::from_p64_with_integrity(bytes, mode) {
        Err(error) => error,
        Ok(_) => panic!("malformed input produced an owned index"),
    }
}

#[test]
fn migration_actual_compiler_gguf_roundtrip_and_public_aliases_remain_available() {
    let gguf = source_gguf();
    let bytes = compile_gguf_to_p64(&gguf, 8).unwrap();
    let index: crate::p64_weight::P64TensorIndex =
        crate::q42_weight::Q42TensorIndex::from_p64_with_integrity(&bytes, IntegrityMode::Full)
            .unwrap();
    let report = index.validate_against_gguf(&bytes, &gguf).unwrap();
    assert_eq!(report.tensor_count, 2);
    assert_eq!(report.tensor_bytes, 96);
    assert_eq!(report.manifold_count, 2);
    let equivalent = index.to_gguf_index();
    assert_eq!(equivalent.hyperparams, index.hyperparams());
    assert!(equivalent.get_layer_tensors(0).attn_q.is_some());
    assert!(equivalent.token_embd_info().is_some());
    assert_eq!(
        index
            .entries
            .iter()
            .map(|entry| index.blob(&bytes, entry).len())
            .sum::<usize>(),
        96
    );
    assert_eq!(
        index.tokenizer_bytes(&bytes),
        &bytes[index.header.tokenizer_offset as usize..index.header.checksum_offset as usize]
    );
    assert!(index.manifold_coordinate(&bytes, 0).is_ok());
    assert_eq!(
        index.manifold_coordinate(&bytes, 2).err().unwrap(),
        "p64: manifold index out of bounds"
    );
}

#[test]
fn migration_owned_manifest_and_clone_do_not_borrow_source_or_each_other() {
    let mut index = {
        let bytes = fixture();
        P64TensorIndex::from_p64_with_integrity(&bytes, IntegrityMode::Full).unwrap()
    };
    let entries: &Vec<P64TensorEntry> = &index.entries;
    assert_eq!(entries.len(), 2);
    let cloned: Q42TensorIndex = index.clone();
    let original_role = cloned.entries[0].role_id;
    index.entries[0].role_id = u16::MAX;
    index.entries.pop();
    assert_eq!(cloned.entries.len(), 2);
    assert_eq!(cloned.entries[0].role_id, original_role);
    assert_eq!(cloned.header.tensor_count, 2);
}

#[test]
fn migration_structure_errors_keep_exact_strings_and_never_return_partial_index() {
    let original = fixture();
    let header = P64WeightHeader::read_le(&original).unwrap();
    assert_eq!(
        error(&original[..63], IntegrityMode::Structure),
        "p64: truncated header"
    );
    for (offset, value, expected) in [
        (0, 0, "p64: invalid magic"),
        (40, 3, "p64: invalid page size"),
        (
            20,
            u32::MAX,
            if cfg!(target_pointer_width = "32") {
                "p64: hyperparameter offset overflow"
            } else {
                "p64: hyperparameters out of bounds"
            },
        ),
        (12, 0, "p64: metadata sections overlap or are out of bounds"),
        (
            header.hparams_offset as usize,
            u32::MAX,
            "p64: manifold count overflow",
        ),
        (
            header.tensor_table_offset as usize + 12,
            0,
            "p64: tensor 0 has invalid rank",
        ),
        (
            header.tensor_table_offset as usize + 64 + 12,
            5,
            "p64: tensor 1 has invalid rank",
        ),
        (
            header.tensor_table_offset as usize + 8,
            2,
            "p64: tensor 0 has invalid manifold index",
        ),
        (
            header.tensor_table_offset as usize,
            u32::MAX,
            if cfg!(target_pointer_width = "32") {
                "p64: tensor name offset overflow"
            } else {
                "p64: tensor 0 has invalid name"
            },
        ),
        (
            header.tensor_table_offset as usize + 32,
            1,
            "p64: tensor 0 is unaligned, overlapping, or out of bounds",
        ),
    ] {
        let mut bytes = original.clone();
        put_u32(&mut bytes, offset, value);
        assert_eq!(error(&bytes, IntegrityMode::Structure), expected);
    }
    let mut bytes = original.clone();
    bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
    assert_eq!(
        error(&bytes, IntegrityMode::Structure),
        "p64: unsupported version 99"
    );
    bytes = original;
    bytes[6] &= !1;
    assert_eq!(
        error(&bytes, IntegrityMode::Structure),
        "p64: non-little-endian container is unsupported"
    );
}

#[test]
fn migration_integrity_modes_keep_metadata_and_payload_corruption_distinct() {
    let original = fixture();
    let index = P64TensorIndex::from_p64_with_integrity(&original, IntegrityMode::Full).unwrap();
    let mut metadata = original.clone();
    metadata[44] ^= 1; // Opaque reserved byte: only metadata integrity should reject.
    for mode in [IntegrityMode::Full, IntegrityMode::Metadata] {
        assert_eq!(error(&metadata, mode), "p64: metadata CRC-32C mismatch");
    }
    assert!(P64TensorIndex::from_p64_with_integrity(&metadata, IntegrityMode::Structure).is_ok());
    let mut payload = original;
    payload[index.entries[0].blob_offset as usize] ^= 1;
    assert_eq!(
        error(&payload, IntegrityMode::Full),
        "p64: tensor 0 CRC-32C mismatch"
    );
    for mode in [IntegrityMode::Metadata, IntegrityMode::Structure] {
        assert!(P64TensorIndex::from_p64_with_integrity(&payload, mode).is_ok());
    }
}

#[test]
fn migration_raw_unknown_roles_survive_but_qualia_semantic_filters_still_apply() {
    let mut bytes = fixture();
    let header = P64WeightHeader::read_le(&bytes).unwrap();
    bytes[header.tensor_table_offset as usize + 4..header.tensor_table_offset as usize + 6]
        .copy_from_slice(&u16::MAX.to_le_bytes());
    put_u32(
        &mut bytes,
        header.manifold_table_offset as usize,
        f32::NAN.to_bits(),
    );
    let index = P64TensorIndex::from_p64_with_integrity(&bytes, IntegrityMode::Structure).unwrap();
    assert_eq!(index.entries.len(), 2);
    assert_eq!(index.entries[0].role_id, u16::MAX);
    assert_eq!(index.to_gguf_index().entries.len(), 1);
    assert!(index.to_gguf_index().get_layer_tensors(0).attn_q.is_none());
    assert!(index.to_gguf_index().token_embd_info().is_some());
    assert_eq!(
        index.manifold_coordinate(&bytes, 0).err().unwrap(),
        "p64: non-finite 10D manifold coordinate"
    );
}

#[test]
fn p64_migration_environment_child() {
    let Ok(expected) = std::env::var("QUALIA_P64_MIGRATION_CHILD") else {
        return;
    };
    let mode = match expected.as_str() {
        "full" => IntegrityMode::Full,
        "structure" => IntegrityMode::Structure,
        "metadata" => IntegrityMode::Metadata,
        _ => panic!("invalid child expectation"),
    };
    assert_eq!(IntegrityMode::from_env(), mode);
    let bytes = fixture();
    let via_alias: Q42TensorIndex = P64TensorIndex::from_q42(&bytes).unwrap();
    let mut payload = bytes.clone();
    payload[via_alias.entries[0].blob_offset as usize] ^= 1;
    assert_eq!(
        P64TensorIndex::from_p64(&payload).is_err(),
        mode == IntegrityMode::Full
    );
    let mut metadata = bytes;
    metadata[44] ^= 1;
    assert_eq!(
        P64TensorIndex::from_q42(&metadata).is_ok(),
        mode == IntegrityMode::Structure
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn migration_environment_aliases_are_checked_in_isolated_processes() {
    for (value, expected) in [
        (None, "metadata"),
        (Some("full"), "full"),
        (Some("strict"), "full"),
        (Some("  ALL  "), "full"),
        (Some("metadata"), "metadata"),
        (Some("meta"), "metadata"),
        (Some("FAST"), "metadata"),
        (Some("structure"), "structure"),
        (Some("struct"), "structure"),
        (Some("skip"), "structure"),
        (Some(" NoNe "), "structure"),
        (Some("unknown"), "metadata"),
        (Some(""), "metadata"),
    ] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["p64_migration_environment_child", "--test-threads=1"])
            .env("QUALIA_P64_MIGRATION_CHILD", expected);
        if let Some(value) = value {
            command.env("QUALIA_P64_INTEGRITY", value);
        } else {
            command.env_remove("QUALIA_P64_INTEGRITY");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "child {value:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    }
}
