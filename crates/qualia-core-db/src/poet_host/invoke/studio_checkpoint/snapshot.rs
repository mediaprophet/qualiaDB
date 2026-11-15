use super::super::{journal, stamped, store, StoreStamp, StoredArtifact, StudioArtifact};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) const SCHEMA: &str = "qualia.studio.native-source-checkpoint/1";
pub(super) const DIALECT: &str = "vibescript/0.1";
pub(super) const SOURCE_KEY: &str = "authoring/native-source.json";
pub(super) const SOURCE_KIND: &str = "application/vnd.qualia.studio-source+json";
pub(super) const PREFIX: &str = "poet.native.submission/";

/// File/output limits are distinct from retained authoring/journal caps.
#[derive(Clone, Copy, Debug)]
pub struct CheckpointLimits {
    pub file_bytes: usize,
    pub source_bytes: usize,
    pub entries: usize,
    pub cells: usize,
    pub journal: super::super::SourceJournalLimits,
}
impl Default for CheckpointLimits {
    fn default() -> Self {
        Self {
            file_bytes: 256 * 1024 * 1024,
            source_bytes: 32 * 1024 * 1024,
            entries: 1024,
            cells: 64,
            journal: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub schema: String,
    pub dialect: String,
    pub artifact: StudioArtifact,
    pub journal: journal::SourceJournal,
    pub parent_digest: Option<[u8; 32]>,
}

/// Detached source and actual private undo/redo states. It carries no authority
/// or runtime StoreStamp. `file_digest` identifies input bytes, not the subject.
#[derive(Clone)]
pub struct Checkpoint {
    pub(super) record: Record,
    pub(super) file_digest: Option<[u8; 32]>,
    pub(super) resources: Vec<super::ResourceData>,
    pub(super) source_meta: Option<Vec<u8>>,
}
impl Checkpoint {
    pub fn artifact(&self) -> &StudioArtifact {
        &self.record.artifact
    }
    pub fn file_digest(&self) -> Option<[u8; 32]> {
        self.file_digest
    }
    pub fn parent_digest(&self) -> Option<[u8; 32]> {
        self.record.parent_digest
    }
    /// Host carries the last verified file digest across live edits. This link
    /// records derivation only; it is not proof of review, authority or adoption.
    pub fn with_parent_digest(mut self, parent: [u8; 32]) -> Self {
        self.record.parent_digest = Some(parent);
        self
    }
}

pub(super) fn validate_nodes(
    nodes: &BTreeMap<String, BTreeMap<String, String>>,
    limits: CheckpointLimits,
) -> Result<(), String> {
    let mut count = 0usize;
    for (key, attrs) in nodes.iter().filter(|(key, _)| key.starts_with(PREFIX)) {
        count = count.checked_add(1).ok_or("source cell count overflow")?;
        if count > limits.cells
            || key != &format!("{PREFIX}{count:020}")
            || !attrs.contains_key("text")
        {
            return Err(
                "checkpoint source cells are not the bounded contiguous identified sequence".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn validate_record(record: &Record, limits: CheckpointLimits) -> Result<(), String> {
    if record.schema != SCHEMA || record.dialect != DIALECT {
        return Err("unsupported native source schema or pinned Vibe dialect".into());
    }
    let art = &record.artifact;
    if art.id.is_empty()
        || art.domain.is_empty()
        || art.id.as_bytes().contains(&0)
        || art.domain.as_bytes().contains(&0)
        || art.routing_lane > 3
    {
        return Err("invalid source identifiers/domain/routing metadata".into());
    }
    // No equality of string/digest is used as an authority or identifier mapping.
    let stored = StoredArtifact {
        artifact: art.clone(),
        journal: record.journal.clone(),
        publication: Arc::new(stamped::PublicationState::new(&art.id, art.lamport_clock)),
    };
    journal::validate_checkpoint(&stored, limits.journal, |nodes| {
        validate_nodes(nodes, limits)
    })
}

/// Capture accepted source + its journal under the same store mutex and stamp.
/// This function never infers that existence/stamp equality grants permission.
pub fn capture(
    id: &str,
    expected: &StoreStamp,
    limits: CheckpointLimits,
) -> Result<Checkpoint, String> {
    let source = store().lock().map_err(|_| "Studio store poisoned")?;
    let stored = source.get(id).ok_or("checkpoint source absent")?;
    // StoreStamp equality remains owned by the existing stamped API.
    if !expected.matches(stored) {
        return Err("stale checkpoint source; no capture".into());
    }
    journal::validate_checkpoint(stored, limits.journal, |nodes| {
        validate_nodes(nodes, limits)
    })?;
    let record = Record {
        schema: SCHEMA.into(),
        dialect: DIALECT.into(),
        artifact: stored.artifact.clone(),
        journal: stored.journal.clone(),
        parent_digest: None,
    };
    Ok(Checkpoint {
        record,
        file_digest: None,
        resources: Vec::new(),
        source_meta: None,
    })
}

/// Activate only after the host's current admission decision. Creates a fresh
/// incarnation and restores real source journal; refuses any existing record.
/// Source remains held until an explicit separately authorized native rebuild.
pub fn activate(
    checkpoint: Checkpoint,
    limits: CheckpointLimits,
) -> Result<(StudioArtifact, StoreStamp), String> {
    validate_record(&checkpoint.record, limits)?;
    let publication = Arc::new(stamped::PublicationState::new(
        &checkpoint.record.artifact.id,
        checkpoint.record.artifact.lamport_clock,
    ));
    let stored = StoredArtifact {
        artifact: checkpoint.record.artifact,
        journal: checkpoint.record.journal,
        publication,
    };
    let mut source = store().lock().map_err(|_| "Studio store poisoned")?;
    if source.contains_key(&stored.id) {
        return Err("Studio artifact already exists; source preserved".into());
    }
    let result = (stored.artifact.clone(), StoreStamp::of(&stored));
    source.insert(stored.id.clone(), stored);
    Ok(result)
}
