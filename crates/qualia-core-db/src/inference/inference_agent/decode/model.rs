use super::*;
use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};
pub(super) struct PreparedModel {
    pub tok: std::sync::Arc<GgufTokenizer>,
    pub tensor_idx: Option<GgufTensorIndex>,
}
pub(super) fn decode_supported(index: Option<&GgufTensorIndex>) -> bool {
    if let Some(index) = index {
        if let Err(reason) = index.hyperparams.decode_supported() {
            log::error!("LLM_DECODE|unsupported-model|{reason}");
            return false;
        }
    }
    true
}
/// Cold model-native lifecycle census; no per-token tensor discovery.
pub(super) fn layer_kinds(index: Option<&GgufTensorIndex>) -> (u32, u32) {
    let mut ssm = 0;
    let mut moe = 0;
    if let Some(index) = index {
        for layer in 0..index.hyperparams.n_layer.min(512) {
            let tensors = index.get_layer_tensors(layer);
            ssm += u32::from(tensors.is_hybrid_ssm_layer());
            moe += u32::from(tensors.moe_router.is_some());
        }
    }
    (ssm, moe)
}
pub(super) fn prepare(engine: &QTensorEngine, model_path: &str) -> PreparedModel {
    // Tokenizer + tensor index come from the matching on-disk
    // format. P64 carries a Q42T tokenizer section and a manifest;
    // GGUF carries both in its own metadata.
    let is_p64_mmap = engine
        .model_bytes()
        .map(|m| crate::p64_weight::has_p64_magic(&m[..]))
        .unwrap_or(false);
    // Prefer engine-cached index from adopt (zero re-CRC); else parse once.
    let p64_index = engine.p64_index.clone().or_else(|| {
        if is_p64_mmap {
            engine
                .model_bytes()
                .and_then(|m| crate::p64_weight::P64TensorIndex::from_p64(m).ok())
        } else {
            None
        }
    });
    let mut tok = if let (Some(qi), Some(m)) = (p64_index.as_ref(), engine.model_bytes()) {
        GgufTokenizer::from_p64_section(qi.tokenizer_bytes(m)).unwrap_or_default()
    } else if let Some(sibling_tok) = {
        let mp = std::path::Path::new(&model_path);
        let candidate = if mp.is_dir() {
            mp.join("tokenizer.json")
        } else {
            mp.parent()
                .map(|p| p.join("tokenizer.json"))
                .unwrap_or_default()
        };
        if candidate.exists() {
            std::fs::read_to_string(&candidate)
                .ok()
                .and_then(|s| GgufTokenizer::from_hf_json(&s))
        } else {
            None
        }
    } {
        sibling_tok
    } else {
        engine
            .model_bytes()
            .map(|m| GgufTokenizer::from_gguf(m))
            .unwrap_or_default()
    };
    // Sibling canonical `.q42` metadata (convert-time stop set / chat metadata).
    apply_model_helper_stops(&model_path, &mut tok);

    let tensor_idx = engine.tensor_index_cache.clone().or_else(|| {
        if let Some(qi) = p64_index {
            Some(qi.to_gguf_index())
        } else {
            engine
                .model_bytes()
                .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m))
        }
    });

    PreparedModel {
        tok: std::sync::Arc::new(tok),
        tensor_idx,
    }
}
