//! Bounded exact union of canonical texture dependencies with retained HMC entries.
//! Opaque retained metadata belongs to its original entry and is never regenerated.
use super::{codec, graph, snapshot, CheckpointLimits, ResourceData};
use crate::render::{
    asset_package::texture_hmc_key, assets::TextureDependency, compile_10d::CompiledAsset,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct TextureUnion<'a> {
    pub new_dependencies: Vec<&'a TextureDependency>,
    pub retained_keys: BTreeSet<String>,
}
fn texture_namespace(key: &str) -> bool {
    // Reject case aliases instead of normalising distinct resource identifiers.
    key.eq_ignore_ascii_case("textures/sha256")
        || key
            .get(..16)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("textures/sha256/"))
}
fn image_kind(kind: &str) -> bool {
    let mime = kind.split(';').next().unwrap_or("").trim();
    let Some((family, subtype)) = mime.split_once('/') else {
        return false;
    };
    family.eq_ignore_ascii_case("image")
        && !subtype.is_empty()
        && subtype
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.' | b'_'))
        && !kind.as_bytes().contains(&0)
}
fn insert_key(
    keys: &mut BTreeSet<String>,
    key: &str,
    limits: CheckpointLimits,
) -> Result<(), String> {
    if key.is_empty()
        || key.len() > limits.file_bytes
        || key.as_bytes().contains(&0)
        || keys.contains(key)
    {
        return Err("derived checkpoint key is invalid, reserved or already retained".into());
    }
    if keys.len() >= limits.entries {
        return Err("checkpoint entry budget exceeded".into());
    }
    keys.insert(key.to_owned());
    Ok(())
}
pub(super) fn plan<'a>(
    retained: &[ResourceData],
    assets: &'a [(String, CompiledAsset)],
    limits: CheckpointLimits,
) -> Result<TextureUnion<'a>, String> {
    let mut keys = BTreeSet::from([snapshot::SOURCE_KEY.to_owned()]);
    let mut existing = BTreeMap::new();
    for resource in retained {
        insert_key(&mut keys, &resource.key, limits)?;
        if texture_namespace(&resource.key) {
            // Unrelated retained entries remain inert, including future kinds.
            // Exact known dependency checks apply only when joining below.
            existing.insert(resource.key.as_str(), resource);
        }
    }
    // Reserve all non-texture derived keys before admitting dependency keys.
    // Geometry/graph/source collisions never acquire texture deduplication rules.
    for (key, asset) in assets {
        let mut charge = 0;
        codec::charge(
            &mut charge,
            key,
            "10d",
            asset.container_10d.len(),
            0,
            limits.file_bytes,
        )?;
        if key
            .len()
            .checked_add(11)
            .is_none_or(|n| n > limits.file_bytes)
        {
            return Err("derived graph key exceeds checkpoint file budget".into());
        }
        if texture_namespace(key) {
            return Err("derived geometry cannot occupy the canonical texture namespace".into());
        }
        insert_key(&mut keys, key, limits)?;
        let graph_key = graph::key(key);
        insert_key(&mut keys, &graph_key, limits)?;
    }
    let mut declarations = BTreeMap::<[u8; 32], &'a TextureDependency>::new();
    let mut new_dependencies = Vec::new();
    let mut retained_keys = BTreeSet::new();
    for (_, asset) in assets {
        for dependency in &asset.texture_dependencies {
            // Length/entry charge before hashing caller-owned payloads, MIME
            // scans or key construction. This is a single-entry lower bound;
            // aggregate unique charges are checked by encode after planning.
            let mut charge = 0;
            codec::charge(
                &mut charge,
                "textures/sha256/0000000000000000000000000000000000000000000000000000000000000000",
                &dependency.mime_type,
                dependency.bytes.len(),
                0,
                limits.file_bytes,
            )?;
            if codec::digest(&dependency.bytes) != dependency.digest
                || !image_kind(&dependency.mime_type)
            {
                return Err(
                    "compiled texture dependency digest or image MIME kind is invalid".into(),
                );
            }
            if let Some(old) = declarations.get(&dependency.digest) {
                if old.bytes != dependency.bytes || old.mime_type != dependency.mime_type {
                    return Err(
                        "compiled texture dependencies declare conflicting payload/MIME metadata"
                            .into(),
                    );
                }
                continue;
            }
            declarations.insert(dependency.digest, dependency);
            let key = texture_hmc_key(&dependency.digest);
            if let Some(old) = existing.get(key.as_str()) {
                if old.bytes != dependency.bytes || old.kind != dependency.mime_type {
                    return Err("compiled texture conflicts with retained payload/MIME metadata; retained entry preserved".into());
                }
                // TextureDependency carries no opaque metadata. Preserve the
                // complete existing record, including its exact metadata bytes.
                retained_keys.insert(key);
            } else {
                if existing.keys().any(|old| old.eq_ignore_ascii_case(&key)) {
                    return Err("retained resource aliases a canonical texture key; explicit reconciliation required".into());
                }
                insert_key(&mut keys, &key, limits)?;
                new_dependencies.push(dependency);
            }
        }
    }
    Ok(TextureUnion {
        new_dependencies,
        retained_keys,
    })
}
