//! Preserve the actual compiled-asset Q42 facts and lexicon using native codecs.
use crate::q42_volume::{UnifiedVolumeBuilder, WasmSliceQ42Reader};
use crate::render::compile_10d::CompiledAsset;

pub(super) fn key(asset_key: &str) -> String {
    format!("graphs/{asset_key}.q42")
}

/// Conservative cold construction charge, including fixed SuperBlock scratch,
/// postings/DAG/directory and lexicon work; not a peak-allocator measurement.
pub(super) fn charge(asset: &CompiledAsset) -> Result<usize, String> {
    let blocks = asset.quins.len().div_ceil(crate::QUINS_PER_BLOCK);
    let lex = asset.lexicon.values().try_fold(0usize, |sum, text| {
        sum.checked_add(text.len())
            .and_then(|n| n.checked_add(128))
            .ok_or("graph lexicon budget overflow")
    })?;
    blocks
        .checked_mul(crate::q42_volume::SUPERBLOCK_SIZE * 8)
        .and_then(|n| n.checked_add(lex.saturating_mul(4)))
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| "graph construction budget overflow".into())
}

pub(super) fn encode(asset: &CompiledAsset) -> Result<Vec<u8>, String> {
    if asset.quins.iter().any(|q| !q.verify_ecc_parity()) {
        return Err("compiled asset graph contains invalid native NQuin parity".into());
    }
    let mut quins = asset.quins.clone();
    quins.sort_unstable_by_key(|q| q.object);
    // Default author is the native builder's system/anonymous ingest context;
    // no reviewer/rights/adoption/signature is invented here.
    let mut builder = UnifiedVolumeBuilder::with_lex_map(&asset.lexicon)
        .map_err(|e| format!("native graph lexicon: {e:?}"))?;
    for (sequence, chunk) in quins.chunks(crate::QUINS_PER_BLOCK).enumerate() {
        builder
            .push_block(sequence as u64, chunk)
            .map_err(|e| e.to_string())?;
    }
    Ok(builder.finish_to_bytes())
}

pub(super) fn validate(bytes: &[u8], work_budget: usize) -> Result<(), String> {
    let reader = WasmSliceQ42Reader::open(bytes).map_err(|e| e.to_string())?;
    // Retain every Q42 dialect accepted by the canonical reader. New compiled
    // graphs use unified volumes, but a working copy must not silently discard
    // or rewrite an existing legacy graph/lexicon to fit that writer profile.
    let blocks = usize::try_from(reader.block_count()).map_err(|_| "graph block count overflow")?;
    let charge = blocks
        .checked_mul(crate::q42_volume::SUPERBLOCK_SIZE * 8)
        .and_then(|n| n.checked_add(4096))
        .ok_or("graph work budget overflow")?;
    if charge > work_budget {
        return Err("checkpoint graph decode exceeds caller cold-work budget".into());
    }
    let quins = reader.collect_all_quins().map_err(|e| e.to_string())?;
    if quins.iter().any(|q| !q.verify_ecc_parity()) {
        return Err("checkpoint graph parity failed".into());
    }
    Ok(())
}
