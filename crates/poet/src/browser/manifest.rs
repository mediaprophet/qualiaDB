//! ManifoldSeed CBOR-LD serialization and manifest persistence.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Provides serialization/deserialization of `ManifoldSeed` to/from CBOR-LD
//! bytes, and a manifest persistence layer that can POST to the daemon
//! `POST /manifest` endpoint when native, or to `localStorage` on public web.

use crate::tool_chest::core::{ConstructSeed, ManifoldSeed};
use base64::Engine;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// CBOR-LD serialization
// ---------------------------------------------------------------------------

/// Serialize a `ManifoldSeed` to CBOR-LD bytes.
pub fn serialize_seed(seed: &ManifoldSeed) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(seed, &mut bytes).map_err(|e| format!("cbor encode: {}", e))?;
    Ok(bytes)
}

/// Deserialize a `ManifoldSeed` from CBOR-LD bytes.
pub fn deserialize_seed(bytes: &[u8]) -> Result<ManifoldSeed, String> {
    let mut seed: ManifoldSeed =
        ciborium::de::from_reader(bytes).map_err(|e| format!("cbor decode: {}", e))?;
    crate::browser::canvas_state::normalise_seed_ids(std::slice::from_mut(&mut seed));
    Ok(seed)
}

/// Serialize a vector of `ManifoldSeed` to CBOR-LD bytes.
pub fn serialize_seeds(seeds: &[ManifoldSeed]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(seeds, &mut bytes).map_err(|e| format!("cbor encode: {}", e))?;
    Ok(bytes)
}

/// Deserialize a vector of `ManifoldSeed` from CBOR-LD bytes.
pub fn deserialize_seeds(bytes: &[u8]) -> Result<Vec<ManifoldSeed>, String> {
    let mut seeds: Vec<ManifoldSeed> =
        ciborium::de::from_reader(bytes).map_err(|e| format!("cbor decode: {}", e))?;
    crate::browser::canvas_state::normalise_seed_ids(&mut seeds);
    Ok(seeds)
}

/// Load the latest auto/checkpoint seed set used by the browser shell.
pub fn load_saved_seeds() -> Result<Option<Vec<ManifoldSeed>>, String> {
    let Some(storage) = web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
    else {
        return Ok(None);
    };
    let Some(encoded) = storage
        .get_item("qualia-ui:manifest:seeds")
        .map_err(|error| format!("localStorage read: {:?}", error))?
    else {
        return Ok(None);
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("base64 decode: {}", error))?;
    deserialize_seeds(&bytes).map(Some)
}

// ---------------------------------------------------------------------------
// Checkpoint metadata — actor, timestamp, save mode
// ---------------------------------------------------------------------------

/// Default actor identity for the principal / inventor.
/// In a multi-agent system, this would be replaced by the authenticated
/// actor's DID. For now, all saves are attributed to Timothy Charles Holborn.
pub const DEFAULT_ACTOR_DID: &str = "did:qualia:timothy_charles_holborn";

/// Save mode — determines what a checkpoint captures and how it's stored.
/// See `SAVE_ARCHITECTURE.md` for the full specification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveMode {
    /// Frequency-based automatic save (rolling buffer).
    Auto,
    /// User-named checkpoint with label.
    Checkpoint,
    /// Full state + complete provenance graph (archival).
    Snapshot,
    /// Pruned state with tombstones consolidated (distribution).
    Pruned,
}

/// Checkpoint metadata — recorded alongside the serialized seeds.
/// This is the Phase 1 minimal provenance: actor, timestamp, label,
/// parent checkpoint, and save mode. Full provenance graph (operations,
/// constituency, consent, Merkle root) is Phase 2+.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointMeta {
    /// Unique identifier (timestamp-based for Phase 1).
    pub id: String,
    /// User-provided label (empty for Auto saves).
    pub label: String,
    /// Actor who created this checkpoint (DID).
    pub actor: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Save mode that produced this checkpoint.
    pub save_mode: SaveMode,
    /// Previous checkpoint ID (forms a chain).
    pub parent_checkpoint: Option<String>,
}

impl CheckpointMeta {
    /// Create a new checkpoint metadata with the default actor.
    pub fn new(label: &str, mode: SaveMode, parent: Option<&str>) -> Self {
        let timestamp = current_iso8601();
        let id = format!("cp-{}", timestamp.replace(':', "-"));
        Self {
            id,
            label: label.to_string(),
            actor: DEFAULT_ACTOR_DID.to_string(),
            timestamp,
            save_mode: mode,
            parent_checkpoint: parent.map(|s| s.to_string()),
        }
    }

    /// Serialize to a JSON string for localStorage storage.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

/// Get the current time as an ISO 8601 string.
/// Uses `js_sys::Date` in the browser.
fn current_iso8601() -> String {
    let date = js_sys::Date::new_0();
    date.to_iso_string()
        .as_string()
        .unwrap_or_else(|| "unknown".to_string())
}

/// Get the ID of the last checkpoint (for chaining).
fn get_last_checkpoint_id() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage
        .get_item("qualia-ui:manifest:last-checkpoint")
        .ok()?
}

/// Save all manifold seeds to localStorage as base64-encoded CBOR-LD,
/// with checkpoint metadata (actor, timestamp, label, save mode, parent).
///
/// This is the Phase 1 save function. It records:
/// - Who saved (actor DID, default: did:qualia:timothy_charles_holborn)
/// - When (ISO 8601 timestamp)
/// - What mode (Auto, Checkpoint, Snapshot, Pruned)
/// - What label (user-provided for Checkpoint/Snapshot)
/// - Parent checkpoint (for chaining)
///
/// See `SAVE_ARCHITECTURE.md` for the full specification.
pub fn save_checkpoint(label: &str, mode: SaveMode) -> Result<CheckpointMeta, String> {
    crate::browser::history::sync_persistence_state();
    let seeds = crate::browser::get_current_seeds();
    let bytes = serialize_seeds(&seeds)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let parent = get_last_checkpoint_id();
    let meta = CheckpointMeta::new(label, mode, parent.as_deref());

    let window = web_sys::window().ok_or("no window")?;
    let storage = window
        .local_storage()
        .map_err(|e| format!("localStorage access: {:?}", e))?
        .ok_or("localStorage not available")?;

    // Store the serialized seeds
    storage
        .set_item("qualia-ui:manifest:seeds", &b64)
        .map_err(|e| format!("localStorage set: {:?}", e))?;
    storage
        .set_item(&format!("qualia-ui:manifest:checkpoint:{}", meta.id), &b64)
        .map_err(|e| format!("localStorage set checkpoint: {:?}", e))?;

    // Store the checkpoint metadata
    storage
        .set_item("qualia-ui:manifest:checkpoint-meta", &meta.to_json())
        .map_err(|e| format!("localStorage set meta: {:?}", e))?;

    // Update the last checkpoint ID (for chaining)
    storage
        .set_item("qualia-ui:manifest:last-checkpoint", &meta.id)
        .map_err(|e| format!("localStorage set last: {:?}", e))?;

    // Append to the checkpoint history list
    let history_key = "qualia-ui:manifest:checkpoint-history";
    let raw_history = storage
        .get_item(history_key)
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut history = parse_checkpoint_history(&raw_history);
    history.push(meta.clone());
    if history.len() > 128 {
        history.drain(..history.len() - 128);
    }
    let history = serde_json::to_string(&history)
        .map_err(|error| format!("checkpoint history encode: {error}"))?;
    storage
        .set_item(history_key, &history)
        .map_err(|e| format!("localStorage set history: {:?}", e))?;

    Ok(meta)
}

fn parse_checkpoint_history(raw: &str) -> Vec<CheckpointMeta> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(raw)
        .or_else(|_| serde_json::from_str(&format!("[{raw}]")))
        .unwrap_or_default()
}

pub fn checkpoint_history() -> Result<Vec<CheckpointMeta>, String> {
    let storage = web_sys::window()
        .ok_or("no window")?
        .local_storage()
        .map_err(|error| format!("localStorage access: {error:?}"))?
        .ok_or("localStorage not available")?;
    let raw = storage
        .get_item("qualia-ui:manifest:checkpoint-history")
        .map_err(|error| format!("localStorage read history: {error:?}"))?
        .unwrap_or_default();
    Ok(parse_checkpoint_history(&raw))
}

pub fn load_checkpoint_seeds(id: &str) -> Result<Vec<ManifoldSeed>, String> {
    if id.is_empty() || id.len() > 256 {
        return Err("invalid checkpoint id".into());
    }
    let storage = web_sys::window()
        .ok_or("no window")?
        .local_storage()
        .map_err(|error| format!("localStorage access: {error:?}"))?
        .ok_or("localStorage not available")?;
    let encoded = storage
        .get_item(&format!("qualia-ui:manifest:checkpoint:{id}"))
        .map_err(|error| format!("localStorage read checkpoint: {error:?}"))?
        .ok_or_else(|| format!("checkpoint `{id}` has no stored seed snapshot"))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("checkpoint base64 decode: {error}"))?;
    deserialize_seeds(&bytes)
}

/// Convenience: save with Auto mode (no label).
pub fn save_all_manifolds() -> Result<(), String> {
    let _ = save_checkpoint("", SaveMode::Auto)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest persistence
// ---------------------------------------------------------------------------

/// Manifest persistence backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceBackend {
    /// Daemon HTTP endpoint (`POST /manifest`).
    Daemon,
    /// Browser `localStorage`.
    LocalStorage,
    /// In-memory only (testing).
    Memory,
}

/// A manifest record — a checkpoint of canvas state.
#[derive(Clone, Debug)]
pub struct ManifestRecord {
    /// The manifold seeds at this checkpoint.
    pub seeds: Vec<ManifoldSeed>,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Revision number (monotonically increasing).
    pub revision: u64,
}

/// Manifest persistence trait.
pub trait ManifestPersistence {
    /// Save a manifest checkpoint.
    fn save(&self, record: &ManifestRecord) -> Result<(), String>;
    /// Load the latest manifest checkpoint.
    fn load_latest(&self) -> Result<Option<ManifestRecord>, String>;
    /// Load a specific revision.
    fn load_revision(&self, revision: u64) -> Result<Option<ManifestRecord>, String>;
}

/// In-memory manifest store (for testing and fallback).
pub struct MemoryManifestStore {
    records: std::cell::RefCell<Vec<ManifestRecord>>,
}

impl MemoryManifestStore {
    pub fn new() -> Self {
        Self {
            records: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl ManifestPersistence for MemoryManifestStore {
    fn save(&self, record: &ManifestRecord) -> Result<(), String> {
        self.records.borrow_mut().push(record.clone());
        Ok(())
    }

    fn load_latest(&self) -> Result<Option<ManifestRecord>, String> {
        Ok(self.records.borrow().last().cloned())
    }

    fn load_revision(&self, revision: u64) -> Result<Option<ManifestRecord>, String> {
        Ok(self
            .records
            .borrow()
            .iter()
            .find(|r| r.revision == revision)
            .cloned())
    }
}

/// localStorage manifest store — persists manifest checkpoints to the
/// browser's `localStorage` as base64-encoded CBOR-LD. Used on public web
/// when the daemon is unreachable. The daemon `POST /manifest` endpoint
/// is used instead when `is_native_host()` is true (plumbed in the qualia repo).
pub struct LocalStorageManifestStore {
    /// localStorage key prefix.
    key_prefix: String,
}

impl LocalStorageManifestStore {
    /// Create a store with the given key prefix (e.g. `"qualia-ui:manifest"`).
    pub fn new(key_prefix: impl Into<String>) -> Self {
        Self {
            key_prefix: key_prefix.into(),
        }
    }

    /// The localStorage key for the latest checkpoint.
    fn latest_key(&self) -> String {
        format!("{}:latest", self.key_prefix)
    }

    /// The localStorage key for a specific revision.
    fn revision_key(&self, revision: u64) -> String {
        format!("{}:rev:{}", self.key_prefix, revision)
    }

    /// Encode a manifest record to a base64 CBOR-LD string.
    fn encode(record: &ManifestRecord) -> Result<String, String> {
        let bytes = serialize_seeds(&record.seeds)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Decode a base64 CBOR-LD string to a manifest record.
    /// The timestamp and revision are stored alongside in a second key.
    fn decode_seeds(encoded: &str) -> Result<Vec<ManifoldSeed>, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| format!("base64 decode: {}", e))?;
        deserialize_seeds(&bytes)
    }
}

impl ManifestPersistence for LocalStorageManifestStore {
    fn save(&self, record: &ManifestRecord) -> Result<(), String> {
        let window = web_sys::window().ok_or("no window")?;
        let storage = window
            .local_storage()
            .map_err(|e| format!("localStorage access: {:?}", e))?
            .ok_or("localStorage not available")?;

        let encoded = Self::encode(record)?;
        let latest_key = self.latest_key();
        let rev_key = self.revision_key(record.revision);

        // Store the seeds (base64 CBOR-LD) under both the latest and revision keys.
        storage
            .set_item(&latest_key, &encoded)
            .map_err(|e| format!("localStorage set: {:?}", e))?;
        storage
            .set_item(&rev_key, &encoded)
            .map_err(|e| format!("localStorage set: {:?}", e))?;

        // Store metadata (timestamp + revision) under a parallel key.
        let meta_key = format!("{}:meta:{}", self.key_prefix, record.revision);
        let meta = format!("{}|{}", record.timestamp, record.revision);
        storage
            .set_item(&meta_key, &meta)
            .map_err(|e| format!("localStorage set: {:?}", e))?;

        Ok(())
    }

    fn load_latest(&self) -> Result<Option<ManifestRecord>, String> {
        let window = web_sys::window().ok_or("no window")?;
        let storage = window
            .local_storage()
            .map_err(|e| format!("localStorage access: {:?}", e))?
            .ok_or("localStorage not available")?;

        let encoded = storage
            .get_item(&self.latest_key())
            .map_err(|e| format!("localStorage get: {:?}", e))?;
        match encoded {
            None => Ok(None),
            Some(enc) => {
                let seeds = Self::decode_seeds(&enc)?;
                // Read metadata from the parallel key — try to find the highest revision.
                let meta_key = format!("{}:meta:", self.key_prefix);
                let mut best: Option<(String, u64)> = None;
                for i in 0..storage.length().unwrap_or(0) {
                    if let Some(key) = storage.key(i).unwrap_or(None) {
                        if let Some(suffix) = key.strip_prefix(&meta_key) {
                            if let Ok(rev) = suffix.parse::<u64>() {
                                if let Some(val) = storage.get_item(&key).unwrap_or(None) {
                                    let parts: Vec<&str> = val.split('|').collect();
                                    if parts.len() == 2 {
                                        let ts = parts[0].to_string();
                                        if best.as_ref().map_or(true, |(_, r)| rev > *r) {
                                            best = Some((ts, rev));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let (timestamp, revision) = best.unwrap_or(("unknown".into(), 0));
                Ok(Some(ManifestRecord {
                    seeds,
                    timestamp,
                    revision,
                }))
            }
        }
    }

    fn load_revision(&self, revision: u64) -> Result<Option<ManifestRecord>, String> {
        let window = web_sys::window().ok_or("no window")?;
        let storage = window
            .local_storage()
            .map_err(|e| format!("localStorage access: {:?}", e))?
            .ok_or("localStorage not available")?;

        let rev_key = self.revision_key(revision);
        let encoded = storage
            .get_item(&rev_key)
            .map_err(|e| format!("localStorage get: {:?}", e))?;
        match encoded {
            None => Ok(None),
            Some(enc) => {
                let seeds = Self::decode_seeds(&enc)?;
                // Read metadata.
                let meta_key = format!("{}:meta:{}", self.key_prefix, revision);
                let (timestamp, revision) = storage
                    .get_item(&meta_key)
                    .unwrap_or(None)
                    .and_then(|val| {
                        let parts: Vec<&str> = val.split('|').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].parse().unwrap_or(revision)))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(("unknown".into(), revision));
                Ok(Some(ManifestRecord {
                    seeds,
                    timestamp,
                    revision,
                }))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cryptographic .hcf and .hmc Container Serialization (Subsystem 1.8)
// ---------------------------------------------------------------------------

use crate::tool_chest::core::registry::SeedContainer;

/// FNV-1a 64-bit deterministic hash for cryptographic envelope fingerprinting.
fn compute_envelope_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Hypermedia Container Format (.hcf) standalone envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HcfContainerEnvelope {
    pub format_version: String,
    pub author_did: String,
    pub created_at: String,
    pub checksum: String,
    pub container: SeedContainer,
}

/// Hypermedia Manifold Container (.hmc) full world snapshot envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HmcManifoldEnvelope {
    pub format_version: String,
    pub author_did: String,
    pub created_at: String,
    pub checksum: String,
    pub provenance_chain: Vec<String>,
    pub manifold: ManifoldSeed,
}

/// Complete observer-scope package: construct metadata plus its authored lenses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstructPackage {
    pub construct: ConstructSeed,
    pub manifolds: Vec<ManifoldSeed>,
}

/// Interactive HCF export of a complete construct composition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HcfConstructEnvelope {
    pub format_version: String,
    pub author_did: String,
    pub created_at: String,
    pub checksum: String,
    pub package: ConstructPackage,
}

/// Archival HMC export of a complete construct composition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HmcConstructEnvelope {
    pub format_version: String,
    pub author_did: String,
    pub created_at: String,
    pub checksum: String,
    pub provenance_chain: Vec<String>,
    pub package: ConstructPackage,
}

fn construct_package(
    construct: &ConstructSeed,
    manifolds: &[ManifoldSeed],
) -> Result<ConstructPackage, String> {
    if construct.id.trim().is_empty() {
        return Err("construct id cannot be empty".into());
    }
    if construct.default_manifold.trim().is_empty() {
        return Err("stub constructs without a default manifold cannot be exported".into());
    }
    let mut packaged = Vec::new();
    for manifold in manifolds {
        if construct.contains_manifold(&manifold.id)
            && !packaged
                .iter()
                .any(|saved: &ManifoldSeed| saved.id == manifold.id)
        {
            packaged.push(manifold.clone());
        }
    }
    if !packaged
        .iter()
        .any(|manifold| manifold.id == construct.default_manifold)
    {
        return Err(format!(
            "construct `{}` is missing its default manifold `{}`",
            construct.id, construct.default_manifold
        ));
    }
    let mut saved_construct = construct.clone();
    saved_construct.manifold_ids = packaged.iter().map(|seed| seed.id.clone()).collect();
    Ok(ConstructPackage {
        construct: saved_construct,
        manifolds: packaged,
    })
}

/// Export a complete construct to the interactive HCF package format.
pub fn export_construct_hcf(
    construct: &ConstructSeed,
    manifolds: &[ManifoldSeed],
    author_did: &str,
) -> Result<Vec<u8>, String> {
    let package = construct_package(construct, manifolds)?;
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&package, &mut payload)
        .map_err(|error| format!("construct HCF payload encode error: {error}"))?;
    let envelope = HcfConstructEnvelope {
        format_version: "1.0-construct".into(),
        author_did: author_did.to_string(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checksum: compute_envelope_hash(&payload),
        package,
    };
    let mut out = b"HCF\x01".to_vec();
    ciborium::ser::into_writer(&envelope, &mut out)
        .map_err(|error| format!("construct HCF envelope encode error: {error}"))?;
    Ok(out)
}

/// Verify and import a complete construct HCF package.
pub fn import_construct_hcf(bytes: &[u8]) -> Result<HcfConstructEnvelope, String> {
    if bytes.len() < 4 || &bytes[..4] != b"HCF\x01" {
        return Err("Invalid construct .hcf header magic bytes".into());
    }
    let envelope: HcfConstructEnvelope = ciborium::de::from_reader(&bytes[4..])
        .map_err(|error| format!("construct HCF envelope decode error: {error}"))?;
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&envelope.package, &mut payload)
        .map_err(|error| format!("construct HCF verification encode error: {error}"))?;
    if envelope.checksum != compute_envelope_hash(&payload) {
        return Err("Construct HCF payload checksum verification failed".into());
    }
    construct_package(&envelope.package.construct, &envelope.package.manifolds)?;
    Ok(envelope)
}

/// Export a complete construct to the archival HMC package format.
pub fn export_construct_hmc(
    construct: &ConstructSeed,
    manifolds: &[ManifoldSeed],
    author_did: &str,
) -> Result<Vec<u8>, String> {
    let package = construct_package(construct, manifolds)?;
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&package, &mut payload)
        .map_err(|error| format!("construct HMC payload encode error: {error}"))?;
    let envelope = HmcConstructEnvelope {
        format_version: "1.0-construct".into(),
        author_did: author_did.to_string(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checksum: compute_envelope_hash(&payload),
        provenance_chain: vec![author_did.to_string()],
        package,
    };
    let mut out = b"HMC\x01".to_vec();
    ciborium::ser::into_writer(&envelope, &mut out)
        .map_err(|error| format!("construct HMC envelope encode error: {error}"))?;
    Ok(out)
}

/// Verify and import a complete construct HMC package.
pub fn import_construct_hmc(bytes: &[u8]) -> Result<HmcConstructEnvelope, String> {
    if bytes.len() < 4 || &bytes[..4] != b"HMC\x01" {
        return Err("Invalid construct .hmc header magic bytes".into());
    }
    let envelope: HmcConstructEnvelope = ciborium::de::from_reader(&bytes[4..])
        .map_err(|error| format!("construct HMC envelope decode error: {error}"))?;
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&envelope.package, &mut payload)
        .map_err(|error| format!("construct HMC verification encode error: {error}"))?;
    if envelope.checksum != compute_envelope_hash(&payload) {
        return Err("Construct HMC payload checksum verification failed".into());
    }
    construct_package(&envelope.package.construct, &envelope.package.manifolds)?;
    Ok(envelope)
}

/// Export a single `SeedContainer` to `.hcf` binary bytes.
pub fn export_hcf(container: &SeedContainer, author_did: &str) -> Result<Vec<u8>, String> {
    let mut payload_bytes = Vec::new();
    ciborium::ser::into_writer(container, &mut payload_bytes)
        .map_err(|e| format!("hcf payload encode error: {}", e))?;

    let checksum = compute_envelope_hash(&payload_bytes);
    let envelope = HcfContainerEnvelope {
        format_version: "1.0".into(),
        author_did: author_did.to_string(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checksum,
        container: container.clone(),
    };

    let mut out = Vec::new();
    out.extend_from_slice(b"HCF\x01");
    ciborium::ser::into_writer(&envelope, &mut out)
        .map_err(|e| format!("hcf envelope encode error: {}", e))?;
    Ok(out)
}

/// Import a `.hcf` binary byte array into a `SeedContainer` and `HcfContainerEnvelope`.
pub fn import_hcf(bytes: &[u8]) -> Result<(SeedContainer, HcfContainerEnvelope), String> {
    if bytes.len() < 4 || &bytes[0..4] != b"HCF\x01" {
        return Err("Invalid .hcf header magic bytes".into());
    }

    let envelope: HcfContainerEnvelope = ciborium::de::from_reader(&bytes[4..])
        .map_err(|e| format!("hcf envelope decode error: {}", e))?;

    // Verify integrity
    let mut payload_bytes = Vec::new();
    ciborium::ser::into_writer(&envelope.container, &mut payload_bytes)
        .map_err(|e| format!("hcf payload verification encode error: {}", e))?;

    let expected_hash = compute_envelope_hash(&payload_bytes);
    if envelope.checksum != expected_hash {
        return Err("HCF payload checksum verification failed".into());
    }

    let container = envelope.container.clone();
    Ok((container, envelope))
}

/// Export a `ManifoldSeed` to `.hmc` binary snapshot bytes.
pub fn export_hmc(manifold: &ManifoldSeed, author_did: &str) -> Result<Vec<u8>, String> {
    let mut payload_bytes = Vec::new();
    ciborium::ser::into_writer(manifold, &mut payload_bytes)
        .map_err(|e| format!("hmc payload encode error: {}", e))?;

    let checksum = compute_envelope_hash(&payload_bytes);
    let envelope = HmcManifoldEnvelope {
        format_version: "1.0".into(),
        author_did: author_did.to_string(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checksum,
        provenance_chain: vec![author_did.to_string()],
        manifold: manifold.clone(),
    };

    let mut out = Vec::new();
    out.extend_from_slice(b"HMC\x01");
    ciborium::ser::into_writer(&envelope, &mut out)
        .map_err(|e| format!("hmc envelope encode error: {}", e))?;
    Ok(out)
}

/// Import a `.hmc` binary snapshot into a `ManifoldSeed` and `HmcManifoldEnvelope`.
pub fn import_hmc(bytes: &[u8]) -> Result<(ManifoldSeed, HmcManifoldEnvelope), String> {
    if bytes.len() < 4 || &bytes[0..4] != b"HMC\x01" {
        return Err("Invalid .hmc header magic bytes".into());
    }

    let envelope: HmcManifoldEnvelope = ciborium::de::from_reader(&bytes[4..])
        .map_err(|e| format!("hmc envelope decode error: {}", e))?;

    // Verify integrity
    let mut payload_bytes = Vec::new();
    ciborium::ser::into_writer(&envelope.manifold, &mut payload_bytes)
        .map_err(|e| format!("hmc payload verification encode error: {}", e))?;

    let expected_hash = compute_envelope_hash(&payload_bytes);
    if envelope.checksum != expected_hash {
        return Err("HMC payload checksum verification failed".into());
    }

    let manifold = envelope.manifold.clone();
    Ok((manifold, envelope))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::registry::{DockPosition, SeedContainer, SeedPanel};

    fn test_seed() -> ManifoldSeed {
        ManifoldSeed {
            id: "test".into(),
            label: "Test".into(),
            icon: "test".into(),
            ontology_prefix: "test".into(),
            description: "Test manifold".into(),
            containers: vec![SeedContainer {
                container_type: "social".into(),
                title: "Social".into(),
                x: 100.0,
                y: 70.0,
                width: 420.0,
                height: 320.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            }],
            connections: vec![],
            panels: vec![SeedPanel {
                panel_type: "pulse-panel".into(),
                dock: DockPosition::Bottom,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_seed_cbor_roundtrip() {
        let seed = test_seed();
        let bytes = serialize_seed(&seed).unwrap();
        assert!(!bytes.is_empty());
        let decoded = deserialize_seed(&bytes).unwrap();
        assert_eq!(decoded.id, "test");
        assert_eq!(decoded.containers.len(), 1);
        assert_eq!(decoded.containers[0].container_type, "social");
        assert_eq!(decoded.panels.len(), 1);
    }

    #[test]
    fn test_seeds_cbor_roundtrip() {
        let seeds = vec![test_seed(), test_seed()];
        let bytes = serialize_seeds(&seeds).unwrap();
        assert!(!bytes.is_empty());
        let decoded = deserialize_seeds(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].id, "test");
    }

    #[test]
    fn test_memory_manifest_store() {
        let store = MemoryManifestStore::new();
        let record = ManifestRecord {
            seeds: vec![test_seed()],
            timestamp: "2026-08-18T00:00:00Z".into(),
            revision: 1,
        };
        store.save(&record).unwrap();

        let latest = store.load_latest().unwrap().unwrap();
        assert_eq!(latest.revision, 1);
        assert_eq!(latest.seeds.len(), 1);

        let rev0 = store.load_revision(0).unwrap();
        assert!(rev0.is_none());

        let rev1 = store.load_revision(1).unwrap().unwrap();
        assert_eq!(rev1.timestamp, "2026-08-18T00:00:00Z");
    }

    #[test]
    fn test_deserialize_invalid_bytes() {
        let result = deserialize_seed(&[0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_hcf_roundtrip() {
        let seed = test_seed();
        let container = &seed.containers[0];
        let bytes = export_hcf(container, DEFAULT_ACTOR_DID).unwrap();
        assert_eq!(&bytes[0..4], b"HCF\x01");

        let (decoded_container, envelope) = import_hcf(&bytes).unwrap();
        assert_eq!(decoded_container.container_type, "social");
        assert_eq!(envelope.author_did, DEFAULT_ACTOR_DID);
        assert!(!envelope.checksum.is_empty());
    }

    #[test]
    fn test_hmc_roundtrip() {
        let seed = test_seed();
        let bytes = export_hmc(&seed, DEFAULT_ACTOR_DID).unwrap();
        assert_eq!(&bytes[0..4], b"HMC\x01");

        let (decoded_manifold, envelope) = import_hmc(&bytes).unwrap();
        assert_eq!(decoded_manifold.id, "test");
        assert_eq!(decoded_manifold.containers.len(), 1);
        assert_eq!(envelope.author_did, DEFAULT_ACTOR_DID);
        assert_eq!(envelope.provenance_chain.len(), 1);
    }

    #[test]
    fn construct_hcf_and_hmc_roundtrip_include_lenses() {
        let seed = test_seed();
        let construct = ConstructSeed {
            id: "test-scope".into(),
            label: "Test scope".into(),
            description: "Observer-scoped test construct".into(),
            icon: "test".into(),
            observer: DEFAULT_ACTOR_DID.into(),
            honesty: "live".into(),
            default_manifold: seed.id.clone(),
            manifold_ids: vec![seed.id.clone()],
            library_uri: "urn:poet:construct:test-scope".into(),
            required_shapes: vec![],
            source: crate::tool_chest::core::ConstructSource::Authored,
        };

        let hcf = export_construct_hcf(&construct, std::slice::from_ref(&seed), DEFAULT_ACTOR_DID)
            .unwrap();
        let interactive = import_construct_hcf(&hcf).unwrap();
        assert_eq!(interactive.package.construct.id, "test-scope");
        assert_eq!(interactive.package.manifolds, vec![seed.clone()]);

        let hmc = export_construct_hmc(&construct, std::slice::from_ref(&seed), DEFAULT_ACTOR_DID)
            .unwrap();
        let archive = import_construct_hmc(&hmc).unwrap();
        assert_eq!(archive.package.manifolds, vec![seed]);
        assert_eq!(archive.provenance_chain, vec![DEFAULT_ACTOR_DID]);
    }
}
