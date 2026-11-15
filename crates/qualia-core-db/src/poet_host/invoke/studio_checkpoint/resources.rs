//! Inert, byte-preserving resource authoring over the canonical HMC codec.
//! A resource inventory is not a species, rights or execution admission claim.
use super::{codec, snapshot, Checkpoint, CheckpointLimits};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceData {
    pub key: String,
    pub kind: String,
    pub bytes: Vec<u8>,
    /// Opaque CBOR metadata remains byte-exact, including unknown schemas.
    pub meta: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSummary {
    pub key: String,
    pub kind: String,
    pub bytes: usize,
    pub digest: [u8; 32],
    pub metadata_bytes: usize,
    pub metadata_digest: Option<[u8; 32]>,
}

#[derive(Clone, Debug)]
pub enum ResourceEdit {
    Add(ResourceData),
    Replace {
        expected: [u8; 32],
        resource: ResourceData,
    },
    /// Retain the selected entry's exact kind and opaque metadata.
    ReplaceBytes {
        key: String,
        expected: [u8; 32],
        bytes: Vec<u8>,
    },
    Remove {
        key: String,
        expected: [u8; 32],
    },
}

impl ResourceData {
    pub fn summary(&self) -> ResourceSummary {
        ResourceSummary {
            key: self.key.clone(),
            kind: self.kind.clone(),
            bytes: self.bytes.len(),
            digest: codec::digest(&self.bytes),
            metadata_bytes: self.meta.as_ref().map_or(0, Vec::len),
            metadata_digest: self.meta.as_deref().map(codec::digest),
        }
    }
}

pub(super) fn validate(resources: &[ResourceData], limits: CheckpointLimits) -> Result<(), String> {
    if resources.len().saturating_add(1) > limits.entries {
        return Err("checkpoint resource entry budget exceeded".into());
    }
    let mut total = 128usize;
    let mut keys = BTreeSet::new();
    for resource in resources {
        if resource.key.is_empty()
            || resource.key == snapshot::SOURCE_KEY
            || resource.key.as_bytes().contains(&0)
            || !keys.insert(resource.key.as_str())
        {
            return Err("invalid, reserved or duplicate checkpoint resource key".into());
        }
        codec::charge(
            &mut total,
            &resource.key,
            &resource.kind,
            resource.bytes.len(),
            resource.meta.as_ref().map_or(0, Vec::len),
            limits.file_bytes,
        )?;
        super::carrier::validate_origin(
            &resource.key,
            &resource.kind,
            &resource.bytes,
            resource.meta.as_deref(),
        )?;
    }
    Ok(())
}

impl Checkpoint {
    pub fn resources(&self) -> &[ResourceData] {
        &self.resources
    }

    pub fn inventory(&self) -> Vec<ResourceSummary> {
        self.resources.iter().map(ResourceData::summary).collect()
    }

    /// Explicit same-source carrier for a fresh capture. Nothing is executed or
    /// restored to the interpreter, and no authority is copied from the input.
    pub fn with_resources_from(
        mut self,
        base: &Checkpoint,
        limits: CheckpointLimits,
    ) -> Result<Self, String> {
        if self.source_meta.is_some() || base.source_meta.is_some() {
            return Err("opaque source metadata requires a verified source carrier; legacy resource copy cannot preserve original source bytes".into());
        }
        if self.artifact().id != base.artifact().id
            || self.artifact().domain != base.artifact().domain
        {
            return Err("checkpoint resource base belongs to another source/domain".into());
        }
        validate(&base.resources, limits)?;
        if base
            .source_meta
            .as_ref()
            .is_some_and(|meta| meta.len() > limits.file_bytes)
        {
            return Err("checkpoint source metadata exceeds caller budget".into());
        }
        self.resources = base.resources.clone();
        self.source_meta = base.source_meta.clone();
        Ok(self)
    }

    /// Transactional detached edit. Package SHA is guarded by native save; each
    /// replace/remove also guards the exact old payload digest. A failed edit
    /// leaves the original carrier unchanged. Reserved source is edited through
    /// the stamped source/journal owner, never this raw-resource interface.
    pub fn apply_resource_edits(
        &self,
        edits: Vec<ResourceEdit>,
        limits: CheckpointLimits,
    ) -> Result<Self, String> {
        validate(&self.resources, limits)?;
        if edits.len() > limits.entries {
            return Err("checkpoint resource edit count exceeds caller budget".into());
        }
        // Expand byte-only replacement against this exact detached base.
        let mut expanded = Vec::with_capacity(edits.len());
        for edit in edits {
            expanded.push(match edit {
                ResourceEdit::ReplaceBytes {
                    key,
                    expected,
                    bytes,
                } => {
                    if super::carrier::origin_namespace(&key) {
                        return Err(
                            "source origins are immutable through generic resource edits".into(),
                        );
                    }
                    let old = self
                        .resources
                        .iter()
                        .find(|entry| entry.key == key)
                        .ok_or("replacement resource absent")?;
                    if bytes.len() > limits.file_bytes {
                        return Err("replacement resource exceeds caller budget".into());
                    }
                    ResourceEdit::Replace {
                        expected,
                        resource: ResourceData {
                            key,
                            kind: old.kind.clone(),
                            bytes,
                            meta: old.meta.clone(),
                        },
                    }
                }
                other => other,
            });
        }
        let edits = expanded;
        let mut affected = BTreeSet::new();
        let mut additions = 0usize;
        let mut removals = 0usize;
        for edit in &edits {
            let (key, expected, resource) = match edit {
                ResourceEdit::Add(resource) => (&resource.key, None, Some(resource)),
                ResourceEdit::Replace { expected, resource } => {
                    (&resource.key, Some(expected), Some(resource))
                }
                ResourceEdit::Remove { key, expected } => (key, Some(expected), None),
                ResourceEdit::ReplaceBytes { .. } => {
                    return Err("unexpanded resource replacement".into())
                }
            };
            if key == snapshot::SOURCE_KEY
                || super::carrier::origin_namespace(key)
                || key.is_empty()
                || key.as_bytes().contains(&0)
                || !affected.insert(key)
            {
                return Err("reserved, invalid or repeated resource edit key".into());
            }
            let existing = self.resources.iter().find(|entry| &entry.key == key);
            match (existing, expected) {
                (None, None) => additions += 1,
                (Some(old), Some(expected)) if codec::digest(&old.bytes) == *expected => {
                    if resource.is_none() {
                        removals += 1;
                    }
                }
                _ => {
                    return Err(format!(
                        "resource '{key}' is absent, already exists or has a stale digest"
                    ))
                }
            }
            if let Some(resource) = resource {
                validate(std::slice::from_ref(resource), limits)?;
                if resource.kind == "q42" {
                    super::graph::validate(&resource.bytes, limits.file_bytes)?;
                }
            }
        }
        if self
            .resources
            .len()
            .saturating_sub(removals)
            .saturating_add(additions)
            .saturating_add(1)
            > limits.entries
        {
            return Err("edited checkpoint resource count exceeds caller budget".into());
        }
        // Preflight the whole resulting set before cloning any retained bytes.
        let mut total = 128usize;
        for old in &self.resources {
            if !affected.contains(&old.key) {
                codec::charge(
                    &mut total,
                    &old.key,
                    &old.kind,
                    old.bytes.len(),
                    old.meta.as_ref().map_or(0, Vec::len),
                    limits.file_bytes,
                )?;
            }
        }
        for edit in &edits {
            if let ResourceEdit::Add(resource) | ResourceEdit::Replace { resource, .. } = edit {
                codec::charge(
                    &mut total,
                    &resource.key,
                    &resource.kind,
                    resource.bytes.len(),
                    resource.meta.as_ref().map_or(0, Vec::len),
                    limits.file_bytes,
                )?;
            }
        }
        let has_edits = !edits.is_empty();
        let guarded_replacements = edits
            .iter()
            .filter_map(|edit| match edit {
                ResourceEdit::Replace { resource, .. } => Some(resource.key.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut candidate = self.clone();
        for edit in edits {
            match edit {
                ResourceEdit::Add(resource) => candidate.resources.push(resource),
                ResourceEdit::Replace { resource, .. } => {
                    let index = candidate
                        .resources
                        .iter()
                        .position(|entry| entry.key == resource.key)
                        .ok_or("resource disappeared")?;
                    candidate.resources[index] = resource;
                }
                ResourceEdit::Remove { key, .. } => {
                    candidate.resources.retain(|entry| entry.key != key)
                }
                ResourceEdit::ReplaceBytes { .. } => {
                    return Err("unexpanded resource replacement".into())
                }
            }
        }
        candidate.file_digest = None;
        if has_edits {
            super::dependencies::validate(
                &self.resources,
                &candidate.resources,
                &guarded_replacements,
                limits.file_bytes,
            )?;
        }
        Ok(candidate)
    }
}
