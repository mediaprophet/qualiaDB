//! One explicit CPU linear layer. No full-model activation or token generation.
use etalon_model::inference::qwen_roles::Qwen35Packing;
use etalon_model_io::lease::{LeaseLimits, ModelLease};
use qualia_core_db::gguf_bridge::{
    qwen35::{OrdinaryNorm, Profile},
    QTensorEngine,
};
use qualia_core_db::inference::moe::ftw_loader::FtwModelPackage;
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

fn digest(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.is_ascii() {
        return Err("expected independently selected 64-digit SHA256".into());
    }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[2 * i..2 * i + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 8 {
        return Err("usage: qwen35_linear_lab <ftw-dir> <layer> <global|grouped|separate> <raw|folded> <epsilon> <workspace-bytes> <source-ledger-bytes> <comma-separated-shard-sha256-in-manifest-order>".into());
    }
    let path = PathBuf::from(&args[0]);
    let layer = args[1].parse::<u32>().map_err(|e| e.to_string())?;
    let packing = match args[2].as_str() {
        "global" => Qwen35Packing::FusedGlobal,
        "grouped" => Qwen35Packing::FusedKeyHeadGrouped,
        "separate" => Qwen35Packing::Separate,
        _ => return Err("explicit packing required".into()),
    };
    let ordinary_norm = match args[3].as_str() {
        "raw" => OrdinaryNorm::RawZeroCentered,
        "folded" => OrdinaryNorm::AlreadyFolded,
        _ => return Err("explicit ordinary norm storage required".into()),
    };
    let profile = Profile {
        packing,
        ordinary_norm,
        epsilon: args[4].parse::<f32>().map_err(|e| e.to_string())?,
    };
    let budget = args[5].parse::<usize>().map_err(|e| e.to_string())?;
    let source_budget = args[6].parse::<u64>().map_err(|e| e.to_string())?;
    let expected: Vec<_> = args[7].split(',').map(digest).collect::<Result<_, _>>()?;
    let package = Arc::new(FtwModelPackage::open_from_dir(&path)?);
    if expected.len() != package.manifest.shards.len() {
        return Err("shard identity count mismatch".into());
    }
    let mut remaining = source_budget;
    let cancelled = AtomicBool::new(false);
    let mut leases = Vec::with_capacity(expected.len());
    for (shard, expected) in package.manifest.shards.iter().zip(expected) {
        let lease = ModelLease::open(
            &path.join(&shard.file),
            expected,
            LeaseLimits {
                max_source_bytes: remaining / 2,
                max_mapped_bytes: remaining / 2,
                max_total_bytes: remaining,
            },
            &cancelled,
        )
        .map_err(|e| format!("model lease: {e:?}"))?;
        remaining = remaining
            .checked_sub(lease.bytes().len() as u64 * 2)
            .ok_or("source ledger exhausted")?;
        leases.push(Arc::new(lease));
    }
    let mut engine = QTensorEngine::new();
    engine.tensor_index_cache = Some(package.tensor_index.clone());
    engine.tensor_data_offset = package.tensor_index.tensor_data_start;
    engine.ftw_package = Some(package);
    let primary = leases.first().cloned().ok_or("empty shard list")?;
    let mut prepared = engine
        .prepare_qwen35_linear_from_leases(layer, profile, budget, primary, &leases)
        .map_err(|e| format!("linear admission: {e:?}"))?;
    // Retained leased sources survive the loader and original engine.
    drop(engine);
    drop(leases);
    let mut hidden = vec![0.125; prepared.roles().geometry.hidden];
    prepared
        .step(&mut hidden)
        .map_err(|e| format!("linear step: {e:?}"))?;
    println!("developer linear-layer only: layer={layer}, profile={profile:?}, workspace_bytes={}, source_ledger_bytes={}, first_hidden={:?}",
        prepared.reserved_bytes(), source_budget - remaining, hidden.first());
    Ok(())
}
