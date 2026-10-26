//! Uses the canonical HMC reader/writer and compiled-asset packaging API.
use super::snapshot::{self, Checkpoint, CheckpointLimits};
use crate::bundle::{BundleReader, BundleWriter};
use crate::render::{asset_package::build_hmc_bundle, compile_10d::CompiledAsset};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{self, Write};

pub(super) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other(
                "checkpoint source serialization budget exceeded",
            ));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Preflight conservative output charge before the core writer allocates. This
/// bounds cold scratch to a constant multiple of the selected output budget;
/// it is not a measured allocator/peak-process-memory claim.
pub(super) fn charge(
    total: &mut usize,
    key: &str,
    kind: &str,
    bytes: usize,
    meta: usize,
    cap: usize,
) -> Result<(), String> {
    let entry = bytes
        .checked_add(key.len())
        .and_then(|n| n.checked_add(kind.len()))
        .and_then(|n| n.checked_add(meta))
        .and_then(|n| n.checked_add(512))
        .ok_or("checkpoint size overflow")?;
    *total = total.checked_add(entry).ok_or("checkpoint size overflow")?;
    if *total > cap {
        return Err("checkpoint exceeds caller file budget before packaging".into());
    }
    Ok(())
}

/// Optional derived assets must already be actual CompiledAssets from the core
/// compiler. Source checkpoint only is allowed; neither case is a complete HMC
/// embodiment claim. Material Vibe expressions remain exact accepted cells.
pub fn encode(
    checkpoint: &Checkpoint,
    assets: Vec<(String, CompiledAsset)>,
    limits: CheckpointLimits,
) -> Result<Vec<u8>, String> {
    if checkpoint.source_meta.is_some() {
        return Err("opaque source-entry metadata cannot be rebased; capture current source and retain a verified source carrier".into());
    }
    snapshot::validate_record(&checkpoint.record, limits)?;
    super::resources::validate(&checkpoint.resources, limits)?;
    let textures = super::texture_union::plan(&checkpoint.resources, &assets, limits)?;
    let mut source = BoundedBytes {
        bytes: Vec::new(),
        limit: limits.source_bytes.min(limits.file_bytes),
    };
    serde_json::to_writer(&mut source, &checkpoint.record).map_err(|e| e.to_string())?;
    let mut total = 128usize;
    let mut count = checkpoint.resources.len().saturating_add(1);
    charge(
        &mut total,
        snapshot::SOURCE_KEY,
        snapshot::SOURCE_KIND,
        source.bytes.len(),
        checkpoint.source_meta.as_ref().map_or(0, Vec::len),
        limits.file_bytes,
    )?;
    for resource in &checkpoint.resources {
        charge(
            &mut total,
            &resource.key,
            &resource.kind,
            resource.bytes.len(),
            resource.meta.as_ref().map_or(0, Vec::len),
            limits.file_bytes,
        )?;
    }
    for (key, asset) in &assets {
        if key == snapshot::SOURCE_KEY {
            return Err("derived asset collides with source entry".into());
        }
        count = count.saturating_add(1);
        charge(
            &mut total,
            key,
            "10d",
            asset.container_10d.len(),
            0,
            limits.file_bytes,
        )?;
        count = count.saturating_add(1);
        charge(
            &mut total,
            &super::graph::key(key),
            "q42",
            super::graph::charge(asset)?,
            0,
            limits.file_bytes,
        )?;
    }
    // Count/charge each new canonical texture once. A matching retained entry
    // already carries both the charge and its original opaque metadata above.
    for dependency in &textures.new_dependencies {
        count = count.saturating_add(1);
        charge(
            &mut total,
            &crate::render::asset_package::texture_hmc_key(&dependency.digest),
            &dependency.mime_type,
            dependency.bytes.len(),
            0,
            limits.file_bytes,
        )?;
    }
    let retained_texture_keys = textures.retained_keys;
    drop(textures.new_dependencies);
    if count > limits.entries {
        return Err("checkpoint entry budget exceeded".into());
    }
    let mut writer = BundleWriter::new();
    writer
        .add_file(
            snapshot::SOURCE_KEY,
            snapshot::SOURCE_KIND,
            source.bytes,
            checkpoint.source_meta.clone(),
        )
        .map_err(|e| e.to_string())?;
    for resource in &checkpoint.resources {
        writer
            .add_file(
                &resource.key,
                &resource.kind,
                resource.bytes.clone(),
                resource.meta.clone(),
            )
            .map_err(|e| e.to_string())?;
    }
    let mut graph_assets = assets.iter().collect::<Vec<_>>();
    graph_assets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    for (key, asset) in graph_assets {
        writer
            .add_file(
                super::graph::key(key),
                "q42",
                super::graph::encode(asset)?,
                None,
            )
            .map_err(|e| e.to_string())?;
    }
    if !assets.is_empty() {
        let derived = build_hmc_bundle(assets).map_err(|e| e.to_string())?;
        let reader = BundleReader::parse(&derived).map_err(|e| e.to_string())?;
        for entry in reader.entries() {
            if retained_texture_keys.contains(&entry.key) {
                // Preflight proved exact digest, bytes and MIME equivalence.
                // Existing entry (including unknown metadata) is already in writer.
                continue;
            }
            writer
                .add_file(
                    &entry.key,
                    &entry.kind,
                    reader
                        .get(&entry.key)
                        .ok_or("derived resource absent")?
                        .to_vec(),
                    entry.meta.clone(),
                )
                .map_err(|e| e.to_string())?;
        }
    }
    let bytes = writer.build().map_err(|e| e.to_string())?;
    if bytes.len() > limits.file_bytes {
        return Err("checkpoint output budget exceeded".into());
    }
    decode(&bytes, limits)?;
    Ok(bytes)
}

/// Integrity and schema checks precede any store access. No evaluation, grants,
/// runtime handles, principal mappings, adoption or rights decisions are loaded.
pub fn decode(bytes: &[u8], limits: CheckpointLimits) -> Result<Checkpoint, String> {
    if bytes.len() > limits.file_bytes {
        return Err("checkpoint file budget exceeded".into());
    }
    // Bound the declared index cardinality before the canonical CBOR reader
    // allocates; still verify the actual index count after parsing.
    if bytes.len() < crate::bundle::BUNDLE_HEADER_SIZE {
        return Err("checkpoint header is truncated".into());
    }
    let declared =
        u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "header count")?) as usize;
    if declared == 0 || declared > limits.entries {
        return Err("checkpoint declared entry budget exceeded".into());
    }
    let reader = BundleReader::parse(bytes).map_err(|e| e.to_string())?;
    if declared != reader.entries().len() {
        return Err("checkpoint declared/index entry count mismatch".into());
    }
    if reader.flags() != 0 || reader.entries().is_empty() || reader.entries().len() > limits.entries
    {
        return Err("unsupported checkpoint flags or entry budget".into());
    }
    let mut keys = BTreeSet::new();
    let mut ranges = Vec::with_capacity(reader.entries().len());
    for entry in reader.entries() {
        if entry.key.is_empty()
            || entry.key.as_bytes().contains(&0)
            || !keys.insert(entry.key.as_str())
            || entry.sha256.len() != 32
            || !reader.verify_entry(&entry.key)
            || entry.offset % crate::bundle::BUNDLE_ENTRY_ALIGN as u64 != 0
        {
            return Err("checkpoint entry identifiers/alignment/hash verification failed".into());
        }
        ranges.push((entry.offset, entry.offset + entry.length));
        super::carrier::validate_origin(
            &entry.key,
            &entry.kind,
            reader
                .get(&entry.key)
                .ok_or("checkpoint origin resource missing")?,
            entry.meta.as_deref(),
        )?;
        if entry.kind == "q42" {
            super::graph::validate(
                reader.get(&entry.key).ok_or("checkpoint graph missing")?,
                limits.file_bytes,
            )?;
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err("checkpoint entry payloads overlap".into());
    }
    let entry = reader
        .entry(snapshot::SOURCE_KEY)
        .ok_or("checkpoint source resource missing")?;
    if entry.kind != snapshot::SOURCE_KIND || entry.length > limits.source_bytes as u64 {
        return Err("checkpoint source kind or byte budget mismatch".into());
    }
    let record = serde_json::from_slice(
        reader
            .get(snapshot::SOURCE_KEY)
            .ok_or("checkpoint source missing")?,
    )
    .map_err(|e| format!("checkpoint source schema: {e}"))?;
    snapshot::validate_record(&record, limits)?;
    // Derived resources remain inert on reopen; format/use admission belongs to
    // the selected native resource loader, never implicit first-entry loading.
    Ok(Checkpoint {
        record,
        file_digest: Some(digest(bytes)),
        source_meta: entry.meta.clone(),
        resources: reader
            .entries()
            .iter()
            .filter(|entry| entry.key != snapshot::SOURCE_KEY)
            .map(|entry| {
                Ok(super::ResourceData {
                    key: entry.key.clone(),
                    kind: entry.kind.clone(),
                    bytes: reader
                        .get(&entry.key)
                        .ok_or("checkpoint resource missing")?
                        .to_vec(),
                    meta: entry.meta.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}
