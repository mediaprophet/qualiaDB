//! Deterministic HMC packing, HTTP range merit order, and digest-based deltas.

use super::{hmc_entry_key_iri, invalid, open_semantic_model, resource_identity_meta, SemanticModelBuilder,
    SpatialEntryMeta, MODEL_ALIAS, SEMANTIC_MODEL_KEY};
use crate::{
    bundle::{BundleEntry, BundleReadLimits, BundleReader, BundleWriter, BUNDLE_PAGE_ALIGN_4096},
    container_10d::crc32c::crc32c,
    q42::{
        q42_volume::payload_catalogue::sha256_hex_bytes,
        selective_streamer::{
            resolve_selective_hmc_ranges_zero_heap, HmcIndexSliceEntry, ResolvedByteRange,
            SelectiveStreamQuery, CONTAINER_KIND_10D, CONTAINER_KIND_M64, CONTAINER_KIND_P64,
            CONTAINER_KIND_Q42,
        },
    },
};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, io, sync::atomic::{AtomicBool, Ordering}};

const GRAPH: &str = "https://qualiadb.org/ontology/hmc#SalienceDelta";
const VOCABULARY: &str = "https://qualiadb.org/ontology/hmc#";
const SALIENCE_META: &str = "stream_salience";
const HAS_ENTRY_SELECTOR: &str = "https://qualiadb.org/ontology/hmc#hasEntrySelector";
const ENTRY_ROOT: &str = "https://qualiadb.org/ontology/hmc#entryRoot";
const ENTRY_CONTENT: &str = "https://qualiadb.org/ontology/hmc#entryContent";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HmcSalience { A, B, C, D }

impl HmcSalience {
    pub const fn as_str(self) -> &'static str {
        match self { Self::A => "A", Self::B => "B", Self::C => "C", Self::D => "D" }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "A" => Some(Self::A), "B" => Some(Self::B), "C" => Some(Self::C), "D" => Some(Self::D),
            _ => None,
        }
    }

    fn for_kind(kind: &str) -> Self {
        match kind {
            "q42" | "manifest" | "vibe" => Self::A,
            "10d" | "m64" | "emf" | "qac" | "gtop" => Self::B,
            "p64" => Self::D,
            _ => Self::C,
        }
    }
}

pub struct HmcPackAsset<'a> {
    pub key: &'a str,
    pub kind: &'a str,
    pub bytes: &'a [u8],
    pub metadata: Option<&'a [u8]>,
    pub salience: Option<HmcSalience>,
}

#[derive(Clone, Copy)]
pub struct HmcPackLimits {
    pub bundle: BundleReadLimits,
    pub resource_bytes: usize,
    pub total_resource_bytes: usize,
    pub semantic_bytes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HmcPackedRange {
    pub key: String,
    pub kind: String,
    pub offset: u64,
    pub length: u32,
    pub sha256_hex: String,
    pub crc32c: u32,
    pub salience: HmcSalience,
}

pub struct HmcPackOutput {
    pub hmc: Vec<u8>,
    pub ranges: Vec<HmcPackedRange>,
    pub delta: Option<Vec<u8>>,
}

fn is_model(key: &str) -> bool {
    key == SEMANTIC_MODEL_KEY || key == MODEL_ALIAS
}

fn check_cancelled(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "HMC packing cancelled"));
    }
    Ok(())
}

fn valid_key(key: &str) -> bool {
    !key.chars().any(|ch| ch.is_control() || ch == '\\' || ch == ':')
        && key.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn read_meta(bytes: Option<&[u8]>) -> io::Result<BTreeMap<String, ciborium::Value>> {
    let Some(mut bytes) = bytes else { return Ok(BTreeMap::new()); };
    let ciborium::Value::Map(fields) = ciborium::from_reader(&mut bytes)
        .map_err(|error| invalid(format!("invalid HMC metadata: {error}")))?
    else {
        return Err(invalid("HMC metadata must be a CBOR map"));
    };
    if !bytes.is_empty() {
        return Err(invalid("trailing bytes in HMC metadata"));
    }
    let mut map = BTreeMap::new();
    for (key, value) in fields {
        let ciborium::Value::Text(key) = key else {
            return Err(invalid("HMC metadata keys must be text"));
        };
        if map.insert(key, value).is_some() {
            return Err(invalid("duplicate HMC metadata key"));
        }
    }
    Ok(map)
}

fn salience(
    meta: &BTreeMap<String, ciborium::Value>, kind: &str,
) -> io::Result<HmcSalience> {
    match meta.get(SALIENCE_META) {
        Some(ciborium::Value::Text(value)) => {
            HmcSalience::parse(value).ok_or_else(|| invalid("unknown HMC stream salience"))
        }
        Some(_) => Err(invalid("HMC stream salience must be text")),
        None => Ok(HmcSalience::for_kind(kind)),
    }
}

fn merge_identity(
    meta: &mut BTreeMap<String, ciborium::Value>, identity_meta: &[u8],
) -> io::Result<()> {
    for (key, value) in read_meta(Some(identity_meta))? {
        if meta.get(&key).is_some_and(|previous| previous != &value) {
            return Err(invalid(format!("HMC metadata conflicts with resource field {key}")));
        }
        meta.insert(key, value);
    }
    Ok(())
}

pub(super) fn hex_digest(digest: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

impl SemanticModelBuilder {
    /// Bind a physical bundle key to its authored root and exact content.
    /// Entry selectors are bundle-relative, so scope their facts to the package graph.
    pub fn add_package_entry_selector(
        self,
        package_iri: &str,
        key: &str,
        root_iri: &str,
        digest: &[u8; 32],
    ) -> io::Result<Self> {
        if !valid_key(key) {
            return Err(invalid("invalid HMC entry key for semantic selector"));
        }
        let mut model = self.add_package_entry(package_iri, root_iri)?;
        let selector = hmc_entry_key_iri(key);
        let content = format!("urn:sha256:{}", hex_digest(digest));
        model.fact(package_iri, HAS_ENTRY_SELECTOR, &selector, package_iri)?;
        model.fact(&selector, ENTRY_ROOT, root_iri, package_iri)?;
        model.fact(&selector, ENTRY_CONTENT, &content, package_iri)?;
        Ok(model)
    }
}

fn package_iri<'a>(items: impl Iterator<Item = (&'a str, &'a str, &'a [u8])>) -> String {
    let mut sha = Sha256::new();
    sha.update(b"qualia:hmc:resources:v1");
    for (key, kind, digest) in items {
        for field in [key.as_bytes(), kind.as_bytes(), digest] {
            sha.update((field.len() as u64).to_le_bytes());
            sha.update(field);
        }
    }
    format!("urn:sha256:{}", hex_digest(&sha.finalize()))
}

struct Pending<'a> {
    asset: &'a HmcPackAsset<'a>,
    digest: [u8; 32],
    metadata: Option<Vec<u8>>,
    spatial: Option<SpatialEntryMeta>,
    salience: HmcSalience,
}

/// Seal canonical semantic content first, followed by intact resources in salience order.
/// The optional baseline must have been independently integrity-admitted. Diff equality
/// uses contents and metadata so physical reordering alone does not generate updates.
pub fn pack_semantic_hmc(
    assets: &[HmcPackAsset<'_>],
    baseline: Option<&BundleReader<'_>>,
    limits: HmcPackLimits,
    cancelled: &AtomicBool,
) -> io::Result<HmcPackOutput> {
    check_cancelled(cancelled)?;
    if assets.is_empty() || assets.len() > limits.bundle.entries {
        return Err(invalid("HMC asset count exceeds packing limit or is zero"));
    }
    let mut keys = BTreeSet::new();
    let mut found_root = false;
    let mut resource_bytes = 0usize;
    for asset in assets {
        if !valid_key(asset.key) || asset.key.len() > limits.bundle.key_bytes
            || asset.kind.is_empty() || asset.kind.len() > limits.bundle.kind_bytes
            || asset.kind.chars().any(char::is_control)
            || !keys.insert(asset.key)
        {
            return Err(invalid(format!("invalid or duplicate HMC resource {}", asset.key)));
        }
        if is_model(asset.key) {
            if found_root || asset.kind != "q42" || asset.salience.is_some_and(|s| s != HmcSalience::A) {
                return Err(invalid("HMC requires one primary class A Q42 semantic model"));
            }
            found_root = true;
        }
        let metadata_length = asset.metadata.map_or(0, <[u8]>::len);
        let byte_limit = if asset.kind == "10d" {
            limits.resource_bytes.min(128 * 1024 * 1024)
        } else {
            limits.resource_bytes
        };
        if asset.bytes.len() > byte_limit || metadata_length > limits.bundle.meta_bytes {
            return Err(invalid(format!("HMC resource exceeds byte budget: {}", asset.key)));
        }
        SemanticModelBuilder::reserve(&mut resource_bytes, limits.total_resource_bytes, asset.bytes.len())?;
    }
    let mut model = SemanticModelBuilder::new(limits.semantic_bytes);
    let mut pending = Vec::with_capacity(assets.len());
    for asset in assets {
        check_cancelled(cancelled)?;
        match asset.kind {
            "q42" => model = model.merge_q42(asset.bytes)?,
            "cbld" => model = model.merge_cbld(asset.bytes)?,
            _ => {}
        }
        if !is_model(asset.key) {
            pending.push(Pending {
                asset, digest: Sha256::digest(asset.bytes).into(), metadata: None, spatial: None,
                salience: HmcSalience::for_kind(asset.kind),
            });
        }
    }
    pending.sort_unstable_by_key(|item| item.asset.key);
    let package = package_iri(pending.iter().map(|item|
        (item.asset.key, item.asset.kind, item.digest.as_slice())));
    model = model.identify_package(&package)?;
    for item in pending.iter_mut().filter(|item| item.asset.kind == "10d") {
        check_cancelled(cancelled)?;
        let (next, spatial) = model.merge_10d(item.asset.bytes)?;
        model = next;
        item.spatial = Some(spatial.ok_or_else(|| invalid(format!(
            "10d resource has no canonical HOLO identity: {}", item.asset.key,
        )))?);
    }
    for item in &mut pending {
        check_cancelled(cancelled)?;
        let mut meta = read_meta(item.asset.metadata)?;
        let identity = if let Some(spatial) = item.spatial.take() {
            merge_identity(&mut meta, &spatial.encode()?)?;
            spatial.root_entity_iri
        } else if let Some(ciborium::Value::Text(iri)) = meta.get("root_entity_iri") {
            let iri = iri.clone();
            merge_identity(&mut meta, &resource_identity_meta(&iri)?)?;
            iri
        } else {
            let iri = format!("urn:sha256:{}", hex_digest(&item.digest));
            merge_identity(&mut meta, &resource_identity_meta(&iri)?)?;
            iri
        };
        model = model.merge_resource_addressability(
            item.asset.kind, item.asset.bytes, &identity, &item.digest,
        )?;
        model = model.add_package_entry_selector(&package, item.asset.key, &identity, &item.digest)?;
        item.salience = item.asset.salience.unwrap_or(salience(&meta, item.asset.kind)?);
        meta.insert(SALIENCE_META.to_owned(), ciborium::Value::Text(item.salience.as_str().to_owned()));
        let mut encoded = Vec::new();
        ciborium::into_writer(&meta, &mut encoded).map_err(|error| invalid(error.to_string()))?;
        if encoded.len() > limits.bundle.meta_bytes {
            return Err(invalid("HMC resource metadata exceeds byte budget"));
        }
        item.metadata = Some(encoded);
    }
    let root = model.finish()?;
    let root_meta = resource_identity_meta(&package)?;
    if pending.len() >= limits.bundle.entries || root.len() > limits.resource_bytes {
        return Err(invalid("HMC semantic model exceeds entry or resource limit"));
    }
    let mut extent = BUNDLE_PAGE_ALIGN_4096;
    let mut index = 16usize;
    for (key, kind, length, metadata) in std::iter::once(
        (SEMANTIC_MODEL_KEY, "q42", root.len(), root_meta.len())
    ).chain(pending.iter().map(|item| (
        item.asset.key, item.asset.kind, item.asset.bytes.len(),
        item.metadata.as_ref().map_or(0, Vec::len),
    ))) {
        let padded = length.checked_add(BUNDLE_PAGE_ALIGN_4096 - 1)
            .ok_or_else(|| invalid("HMC extent overflows"))?
            / BUNDLE_PAGE_ALIGN_4096 * BUNDLE_PAGE_ALIGN_4096;
        SemanticModelBuilder::reserve(&mut extent, limits.bundle.file_bytes, padded)?;
        let index_record = 192usize.saturating_add(key.len()).saturating_add(kind.len())
            .saturating_add(metadata.saturating_mul(2));
        SemanticModelBuilder::reserve(&mut index, limits.bundle.index_bytes, index_record)?;
    }
    SemanticModelBuilder::reserve(&mut extent, limits.bundle.file_bytes, index)?;

    pending.sort_unstable_by_key(|item| (item.salience, item.asset.key));
    let mut writer = BundleWriter::new().with_page_align_4096(true);
    writer.add_file(SEMANTIC_MODEL_KEY, "q42", root, Some(root_meta))
        .map_err(|error| invalid(error.to_string()))?;
    for item in pending {
        check_cancelled(cancelled)?;
        writer.add_file(item.asset.key, item.asset.kind, item.asset.bytes.to_vec(), item.metadata)
            .map_err(|error| invalid(error.to_string()))?;
    }
    let hmc = writer.build().map_err(|error| invalid(error.to_string()))?;
    let reader = BundleReader::parse_with_limits(&hmc, limits.bundle)
        .map_err(|error| invalid(error.to_string()))?;
    open_semantic_model(&reader)?;
    let ranges = resolve_ranges(&reader, limits.bundle.file_bytes, cancelled)?;
    let delta = baseline.map(|previous| {
        diff(previous, &reader, &ranges, limits.semantic_bytes, cancelled)
    }).transpose()?.flatten();
    check_cancelled(cancelled)?;
    Ok(HmcPackOutput { hmc, ranges, delta })
}

fn resolve_ranges(
    reader: &BundleReader<'_>, limit: usize, cancelled: &AtomicBool,
) -> io::Result<Vec<HmcPackedRange>> {
    let mut index = Vec::with_capacity(reader.entries().len());
    for (ordinal, row) in reader.entries().iter().enumerate() {
        check_cancelled(cancelled)?;
        let container_kind = match row.kind.as_str() {
            "10d" => CONTAINER_KIND_10D,
            "m64" => CONTAINER_KIND_M64,
            "p64" => CONTAINER_KIND_P64,
            "q42" => CONTAINER_KIND_Q42,
            _ => 0,
        };
        let bytes = reader.get(&row.key).ok_or_else(|| invalid("missing sealed HMC entry"))?;
        index.push(HmcIndexSliceEntry {
            entry_id: u32::try_from(ordinal).map_err(|_| invalid("HMC entry index exceeds u32"))?,
            container_kind,
            byte_offset: row.offset,
            byte_length: u32::try_from(row.length).map_err(|_| invalid("HTTP range exceeds u32"))?,
            expected_crc32c: crc32c(bytes),
            ..HmcIndexSliceEntry::default()
        });
    }
    let mut ranges = vec![ResolvedByteRange::default(); index.len()];
    let query = SelectiveStreamQuery::new().with_container_mask(0);
    let (written, _) = resolve_selective_hmc_ranges_zero_heap(
        &index, &query, limit as u64, cancelled.load(Ordering::Relaxed), &mut ranges,
    ).map_err(|error| invalid(error.to_string()))?;
    ranges.truncate(written);
    ranges.into_iter().map(|range| {
        let entry = &reader.entries()[range.entry_id as usize];
        let merit = salience(&read_meta(entry.meta.as_deref())?, &entry.kind)?;
        Ok(HmcPackedRange {
            key: entry.key.clone(), kind: entry.kind.clone(),
            offset: range.byte_offset, length: range.byte_length,
            sha256_hex: hex_digest(&entry.sha256), crc32c: range.expected_crc32c,
            salience: merit,
        })
    }).collect()
}

fn delta_fact(
    model: &mut SemanticModelBuilder, subject: &str, predicate: &str, object: &str,
) -> io::Result<()> {
    model.fact(subject, &format!("{VOCABULARY}{predicate}"), object, GRAPH)
}

fn literal(value: &str) -> io::Result<String> {
    serde_json::to_string(value).map_err(|error| invalid(error.to_string()))
}

fn diff(
    baseline: &BundleReader<'_>,
    current: &BundleReader<'_>,
    ranges: &[HmcPackedRange],
    budget: usize,
    cancelled: &AtomicBool,
) -> io::Result<Option<Vec<u8>>> {
    let mut changes = BTreeMap::<&str, (Option<&BundleEntry>, Option<&BundleEntry>)>::new();
    for entry in baseline.entries() {
        let key = if is_model(&entry.key) { SEMANTIC_MODEL_KEY } else { entry.key.as_str() };
        let slot = &mut changes.entry(key).or_default().0;
        if slot.replace(entry).is_some() {
            return Err(invalid("baseline contains multiple primary semantic models"));
        }
    }
    for entry in current.entries() {
        changes.entry(&entry.key).or_default().1 = Some(entry);
    }
    let mut model = SemanticModelBuilder::new(budget);
    let mut changed = false;
    let ranges: BTreeMap<_, _> = ranges.iter().map(|range| (range.key.as_str(), range)).collect();
    for (key, (prior, next)) in changes {
        check_cancelled(cancelled)?;
        let action = match (prior, next) {
            (Some(a), Some(b)) if a.sha256 == b.sha256 && a.kind == b.kind && a.meta == b.meta => continue,
            (Some(_), Some(_)) => "Update",
            (None, Some(_)) => "Add",
            (Some(_), None) => "Remove",
            (None, None) => continue,
        };
        changed = true;
        let subject = format!("urn:qualia:hmc:delta:{}", sha256_hex_bytes(key.as_bytes()));
        delta_fact(&mut model, &subject, "deltaAction", &format!("{VOCABULARY}{action}"))?;
        delta_fact(&mut model, &subject, "resourceKey", &literal(key)?)?;
        if let Some(old) = prior {
            delta_fact(&mut model, &subject, "previousDigest", &format!(
                "urn:sha256:{}", hex_digest(&old.sha256),
            ))?;
        }
        if let Some(new) = next {
            let range = ranges.get(key).ok_or_else(|| invalid("HMC delta range was not resolved"))?;
            delta_fact(&mut model, &subject, "currentDigest", &format!(
                "urn:sha256:{}", hex_digest(&new.sha256),
            ))?;
            delta_fact(&mut model, &subject, "streamSalience", &literal(range.salience.as_str())?)?;
            delta_fact(&mut model, &subject, "rangeOffset", &literal(&range.offset.to_string())?)?;
            delta_fact(&mut model, &subject, "rangeLength", &literal(&range.length.to_string())?)?;
        } else if let Some(old) = prior {
            let merit = salience(&read_meta(old.meta.as_deref())?, &old.kind)?;
            delta_fact(&mut model, &subject, "streamSalience", &literal(merit.as_str())?)?;
        }
    }
    if changed { model.finish().map(Some) } else { Ok(None) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{q_hash, q42_volume::WasmSliceQ42Reader};

    const LIMITS: HmcPackLimits = HmcPackLimits {
        bundle: BundleReadLimits {
            file_bytes: 4 * 1024 * 1024, index_bytes: 64 * 1024, entries: 16,
            key_bytes: 1024, kind_bytes: 64, meta_bytes: 1024, decoded_index_bytes: 64 * 1024,
        },
        resource_bytes: 1024 * 1024,
        total_resource_bytes: 2 * 1024 * 1024,
        semantic_bytes: 1024 * 1024,
    };

    fn file<'a>(key: &'a str, data: &'a [u8], salience: HmcSalience) -> HmcPackAsset<'a> {
        HmcPackAsset { key, kind: "txt", bytes: data, metadata: None, salience: Some(salience) }
    }

    fn lookup(reader: &WasmSliceQ42Reader<'_>, term: &str) {
        let mut page = [0; 4096];
        let mut output = [0; 256];
        let count = reader.lookup_lexicon_hash_into(q_hash(term), &mut page, &mut output)
            .unwrap().unwrap();
        assert_eq!(&output[..count], term.as_bytes());
    }

    #[test]
    fn packs_identical_bytes_in_merit_order_with_borrowed_model_and_companion_terms() {
        let cancel = AtomicBool::new(false);
        let subject = "urn:test:顎関節";
        let mut companion = SemanticModelBuilder::new(LIMITS.semantic_bytes);
        companion.fact(subject, "urn:test:label", "\"顎関節\"@ja", "urn:test:rig").unwrap();
        let companion = companion.finish().unwrap();
        let companion_asset = || HmcPackAsset {
            key: "rig.q42", kind: "q42", bytes: &companion, metadata: None, salience: None,
        };
        let first = pack_semantic_hmc(&[
            file("bulk.txt", b"bulk", HmcSalience::D), companion_asset(),
            file("focus.txt", b"focus", HmcSalience::A),
        ], None, LIMITS, &cancel).unwrap();
        let second = pack_semantic_hmc(&[
            companion_asset(), file("focus.txt", b"focus", HmcSalience::A),
            file("bulk.txt", b"bulk", HmcSalience::D),
        ], None, LIMITS, &cancel).unwrap();
        assert_eq!(first.hmc, second.hmc);
        assert_eq!(first.ranges.iter().map(|row| row.key.as_str()).collect::<Vec<_>>(),
            [SEMANTIC_MODEL_KEY, "focus.txt", "rig.q42", "bulk.txt"]);
        let reader = BundleReader::parse_with_limits(&first.hmc, LIMITS.bundle).unwrap();
        let model = open_semantic_model(&reader).unwrap();
        lookup(&model, subject);
        assert_eq!(reader.get("rig.q42"), Some(companion.as_slice()));
        for range in first.ranges {
            let start = range.offset as usize;
            let end = start + range.length as usize;
            assert_eq!(start % 4096, 0);
            assert_eq!(crc32c(&first.hmc[start..end]), range.crc32c);
        }
    }

    #[test]
    fn projects_distinct_selectors_for_entries_with_a_shared_root() {
        let root = "urn:test:shared-root";
        let metadata = resource_identity_meta(root).unwrap();
        let first_key = "outputs/%E9%A1%8E.txt";
        let second_key = "outputs/顎.txt";
        let assets = [
            HmcPackAsset {
                metadata: Some(&metadata),
                ..file(first_key, b"first", HmcSalience::B)
            },
            HmcPackAsset {
                metadata: Some(&metadata),
                ..file(second_key, b"second", HmcSalience::B)
            },
        ];
        let packed = pack_semantic_hmc(&assets, None, LIMITS, &AtomicBool::new(false)).unwrap();
        let reader = BundleReader::parse_with_limits(&packed.hmc, LIMITS.bundle).unwrap();
        let model = open_semantic_model(&reader).unwrap();
        let projected = SemanticModelBuilder::new(LIMITS.semantic_bytes)
            .merge_q42(reader.get(SEMANTIC_MODEL_KEY).unwrap())
            .unwrap();
        let package = package_iri(assets.iter().map(|asset| {
            let entry = reader.entry(asset.key).unwrap();
            (asset.key, asset.kind, entry.sha256.as_slice())
        }));
        let graph = q_hash(&package);
        let selectors = [hmc_entry_key_iri(first_key), hmc_entry_key_iri(second_key)];
        assert_ne!(selectors[0], selectors[1]);
        for (asset, selector) in assets.iter().zip(&selectors) {
            lookup(&model, selector);
            let digest = &reader.entry(asset.key).unwrap().sha256;
            let content = format!("urn:sha256:{}", hex_digest(digest));
            let edges = [
                (package.as_str(), HAS_ENTRY_SELECTOR, selector.as_str()),
                (selector.as_str(), ENTRY_ROOT, root),
                (selector.as_str(), ENTRY_CONTENT, content.as_str()),
            ];
            for (subject, predicate, object) in edges {
                let found = projected.snapshot_quins().iter().filter(|row| {
                    row.subject == q_hash(subject)
                        && row.predicate == q_hash(predicate)
                        && row.context == graph
                });
                if predicate == HAS_ENTRY_SELECTOR {
                    assert!(found.into_iter().any(|row| row.object == q_hash(object)));
                } else {
                    assert_eq!(found.map(|row| row.object).collect::<Vec<_>>(), [q_hash(object)]);
                }
            }
        }
    }

    #[test]
    fn delta_carries_removals_updates_and_additions_and_identical_repack_is_empty() {
        let cancel = AtomicBool::new(false);
        let before = [
            file("keep.txt", b"same", HmcSalience::B),
            file("update.txt", b"before", HmcSalience::D),
            file("remove.txt", b"old", HmcSalience::C),
        ];
        let packed = pack_semantic_hmc(&before, None, LIMITS, &cancel).unwrap();
        let baseline = BundleReader::parse_with_limits(&packed.hmc, LIMITS.bundle).unwrap();
        let identical = pack_semantic_hmc(&before, Some(&baseline), LIMITS, &cancel).unwrap();
        assert!(identical.delta.is_none());
        let changed = pack_semantic_hmc(&[
            file("keep.txt", b"same", HmcSalience::B),
            file("update.txt", b"after", HmcSalience::D),
            file("added.txt", b"new", HmcSalience::A),
        ], Some(&baseline), LIMITS, &cancel).unwrap();
        let delta = changed.delta.unwrap();
        let reader = WasmSliceQ42Reader::open(&delta).unwrap();
        for term in [
            "https://qualiadb.org/ontology/hmc#Remove",
            "https://qualiadb.org/ontology/hmc#Update",
            "https://qualiadb.org/ontology/hmc#Add",
            "\"remove.txt\"",
        ] {
            lookup(&reader, term);
        }
    }

    #[test]
    fn rejects_unresolved_primary_models_and_respects_cancellation() {
        let cancel = AtomicBool::new(false);
        let invalid = HmcPackAsset {
            key: MODEL_ALIAS, kind: "q42", bytes: b"not a volume", metadata: None, salience: None,
        };
        assert!(pack_semantic_hmc(&[invalid], None, LIMITS, &cancel).is_err());
        cancel.store(true, Ordering::Relaxed);
        let result = pack_semantic_hmc(
            &[file("x.txt", b"x", HmcSalience::A)], None, LIMITS, &cancel,
        );
        assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::Interrupted));
    }
}
