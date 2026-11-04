use super::*;
use etalon_model::inference::{ModelHyperparams, ModelMetadata, MODEL_ROUTING_EXTENSION_BYTES};

pub fn gguf_native_model_metadata(index: &GgufTensorIndex) -> ModelMetadata {
    let hyperparams = ModelHyperparams::from(index.hyperparams);
    ModelMetadata {
        hyperparams,
        ple_config: index.ple_config,
        moe_routing: index.moe_routing,
    }
}

pub(super) fn append(index: &GgufTensorIndex, tokenizer: &mut Vec<u8>) -> Result<(), String> {
    let metadata = gguf_native_model_metadata(index);
    if !metadata.needs_extension() {
        return Ok(());
    }
    let extension_bytes = metadata.extension_bytes();
    tokenizer
        .len()
        .checked_add(extension_bytes)
        .ok_or("p64: model metadata length overflow")?;
    tokenizer
        .try_reserve_exact(extension_bytes)
        .map_err(|_| "p64: model metadata allocation failed")?;
    let start = tokenizer.len();
    tokenizer.resize(start + extension_bytes, 0);
    metadata
        .write_extension_into(&mut tokenizer[start..])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn locator(
    index: &GgufTensorIndex,
    tokenizer: &[u8],
) -> Result<[u8; 20], GgufCompileError> {
    let reserved = etalon_model::inference::metadata_extension_locator(tokenizer)
        .map_err(|_| GgufCompileError::NativeMetadata)?;
    let expected = gguf_native_model_metadata(index);
    if expected.needs_extension() {
        if reserved == [0; 20] {
            return Err(GgufCompileError::NativeMetadata);
        }
        let start = u32::from_le_bytes(reserved[8..12].try_into().unwrap()) as usize;
        let mut canonical = [0; MODEL_ROUTING_EXTENSION_BYTES];
        let canonical_len = expected
            .write_extension_into(&mut canonical)
            .map_err(|_| GgufCompileError::NativeMetadata)?;
        if tokenizer[start..] != canonical[..canonical_len] {
            return Err(GgufCompileError::NativeMetadata);
        }
    }
    Ok(reserved)
}
