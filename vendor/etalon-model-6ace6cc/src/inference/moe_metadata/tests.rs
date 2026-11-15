use super::*;
use crate::inference::moe_routing::MoeRoutingSettings;

fn metadata(k: Option<u32>) -> ModelMetadata {
    ModelMetadata {
        hyperparams: ModelHyperparams::default(),
        ple_config: None,
        moe_routing: MoeRoutingSettings {
            expert_count: k.map(|_| 256),
            experts_per_token: k,
        },
    }
}

#[test]
fn legacy_record_bytes_unchanged_and_routing_roundtrips() {
    let old = metadata(None);
    let mut expected = [0; MODEL_METADATA_EXTENSION_BYTES];
    old.write_legacy_extension_into(&mut expected).unwrap();
    let mut written = [0; MODEL_METADATA_EXTENSION_BYTES];
    old.write_extension_into(&mut written).unwrap();
    assert_eq!(written, expected);
    assert_eq!(ModelMetadata::read_extension(&written).unwrap(), old);
    for k in [6, 10] {
        let original = metadata(Some(k));
        let mut bytes = [0; MODEL_ROUTING_EXTENSION_BYTES];
        assert_eq!(
            original.write_extension_into(&mut bytes).unwrap(),
            bytes.len()
        );
        let restored = ModelMetadata::read_extension(&bytes).unwrap();
        assert_eq!(restored, original);
        assert_eq!(
            restored.moe_routing.admit(256).unwrap().experts_per_token(),
            k as usize
        );
        assert_eq!(&bytes[16..752], &expected);
        assert_eq!(metadata_extension_locator(&bytes).unwrap()[16], 2);
    }
}

#[test]
fn checked_routing_refuses_corruption_and_invalid_semantics() {
    let mut bytes = [0; MODEL_ROUTING_EXTENSION_BYTES];
    metadata(Some(6)).write_extension_into(&mut bytes).unwrap();
    let good = bytes;
    for (offset, value) in [(8, 3), (752, 7), (757, 0), (760, 0), (760, 17), (764, 1)] {
        bytes = good;
        bytes[offset] = value;
        let crc = crc32c(&bytes[..780]);
        bytes[780..784].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            ModelMetadata::read_extension(&bytes),
            Err(PrepareModelError::MetadataExtension)
        );
    }
    bytes = good;
    bytes[760] ^= 1;
    assert!(ModelMetadata::read_extension(&bytes).is_err());
    assert!(ModelMetadata::read_extension(&good[..783]).is_err());
}

#[test]
fn gguf_import_keeps_explicit_top6_and_top10() {
    for k in [6u32, 10] {
        let mut gguf = b"GGUF".to_vec();
        gguf.extend_from_slice(&3u32.to_le_bytes());
        gguf.extend_from_slice(&0u64.to_le_bytes());
        gguf.extend_from_slice(&2u64.to_le_bytes());
        for (key, value) in [
            ("llama.expert_count", 256u32),
            ("llama.expert_used_count", k),
        ] {
            gguf.extend_from_slice(&(key.len() as u64).to_le_bytes());
            gguf.extend_from_slice(key.as_bytes());
            gguf.extend_from_slice(&4u32.to_le_bytes());
            gguf.extend_from_slice(&value.to_le_bytes());
        }
        let index = TensorIndex::<ModelHyperparams>::try_build(&gguf).unwrap();
        assert_eq!(index.moe_routing, metadata(Some(k)).moe_routing);
        assert_eq!(
            index.moe_routing.admit(256).unwrap().experts_per_token(),
            k as usize
        );
    }
}

#[test]
fn ftw_retains_explicit_topk_and_refuses_nested_conflicts() {
    use super::super::ftw_config::*;
    for k in [6, 10] {
        let mut config = FtwConfig::default();
        config.fields.num_experts = Some(256);
        config.fields.num_experts_per_tok = Some(k);
        assert_eq!(config.moe_routing().unwrap(), metadata(Some(k)).moe_routing);
        config.text_config = Some(FtwTextConfig {
            num_experts: Some(256),
            num_experts_per_tok: Some(k + 1),
            ..Default::default()
        });
        assert!(config.moe_routing().is_err());
    }
}
