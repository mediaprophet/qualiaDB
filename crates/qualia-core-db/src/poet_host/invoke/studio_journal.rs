//! Bounded, typed in-memory source history for the existing Studio store.
//! This is undo state, not a codec, project ledger or authorization mechanism.

use super::{StoredArtifact, StudioArtifact};
use std::collections::BTreeMap;

pub const MAX_REVISION: u32 = 0x1fff_ffff;
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_JOURNAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_JOURNAL_STEPS: usize = 64;
// Conservative cold-state accounting for owned strings/tree entries. This is a
// declared admission charge, not a measured allocator or process-memory claim.
const FIELD_BYTE_CHARGE: usize = 256;

/// Explicit host-selected cold-state budgets. Scripts cannot raise these through
/// Studio.edit. Defaults bound this legacy repair, not production master quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceJournalLimits {
    pub source_bytes: usize,
    pub journal_bytes: usize,
    pub steps: usize,
    pub audit_entries: usize,
    pub source_fields: usize,
}

impl Default for SourceJournalLimits {
    fn default() -> Self {
        Self { source_bytes: MAX_SOURCE_BYTES, journal_bytes: MAX_JOURNAL_BYTES,
            steps: MAX_JOURNAL_STEPS, audit_entries: 256, source_fields: 16_384 }
    }
}

/// Only editable source is restored; identifiers, domain, routing, sensitivity,
/// context and append-only audit history are never rewound by undo/redo.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceState {
    title: String,
    nodes: BTreeMap<String, BTreeMap<String, String>>,
    properties: BTreeMap<String, String>,
}

/// Actual retained undo source contents, not a fabricated historical revision.
/// The caller binds it to the current StoreStamp and supplies read authority.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EditableSourceSnapshot {
    pub title: String,
    pub nodes: BTreeMap<String, BTreeMap<String, String>>,
    pub properties: BTreeMap<String, String>,
}

pub(super) fn previous_source_snapshot(art: &StoredArtifact) -> Option<EditableSourceSnapshot> {
    art.journal
        .undo
        .last()
        .map(|source| EditableSourceSnapshot {
            title: source.title.clone(),
            nodes: source.nodes.clone(),
            properties: source.properties.clone(),
        })
}

impl SourceState {
    fn capture(art: &StudioArtifact) -> Self {
        Self { title: art.title.clone(), nodes: art.nodes.clone(), properties: art.properties.clone() }
    }

    fn restore(self, art: &mut StudioArtifact) {
        art.title = self.title;
        art.nodes = self.nodes;
        art.properties = self.properties;
    }

    fn bytes(&self) -> usize {
        source_size(&self.title, &self.nodes, &self.properties).0
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceJournal {
    undo: Vec<SourceState>,
    redo: Vec<SourceState>,
    limits: SourceJournalLimits,
}

fn source_size(
    title: &str,
    nodes: &BTreeMap<String, BTreeMap<String, String>>,
    properties: &BTreeMap<String, String>,
) -> (usize, usize) {
    let mut bytes = title.len();
    let mut fields = properties.len() + nodes.len();
    for (key, value) in properties {
        bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
    }
    for (id, attributes) in nodes {
        bytes = bytes.saturating_add(id.len());
        fields = fields.saturating_add(attributes.len());
        for (key, value) in attributes {
            bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
        }
    }
    (bytes.saturating_add(fields.saturating_mul(FIELD_BYTE_CHARGE)), fields)
}

fn validate_source(art: &StudioArtifact, limits: SourceJournalLimits) -> Result<usize, String> {
    let (bytes, fields) = source_size(&art.title, &art.nodes, &art.properties);
    if bytes > limits.source_bytes || fields > limits.source_fields {
        return Err("Studio source exceeds bounded undo payload/field budget".into());
    }
    Ok(bytes)
}

fn admit(art: &StoredArtifact, expected: Option<u32>) -> Result<u32, String> {
    if expected.is_some_and(|revision| revision != art.lamport_clock) {
        return Err(format!("Stale Studio revision: expected {:?}, current {}", expected, art.lamport_clock));
    }
    if art.lamport_clock >= MAX_REVISION || art.lamport_clock == 0 {
        return Err("Studio revision exhausted or invalid; source preserved".into());
    }
    if art.history.len() >= art.journal.limits.audit_entries
        || art.history.iter().map(String::len).sum::<usize>() > art.journal.limits.source_bytes
    {
        return Err("Studio append-only history budget exhausted; source preserved".into());
    }
    validate_source(art, art.journal.limits)?;
    Ok(art.lamport_clock + 1)
}

fn journal_bytes(journal: &SourceJournal) -> usize {
    journal.undo.iter().chain(&journal.redo).map(SourceState::bytes).sum()
}

fn admit_audit(art: &StudioArtifact, limits: SourceJournalLimits, entry: &str) -> Result<(), String> {
    let bytes = art.history.iter().fold(0usize, |sum, value| sum.saturating_add(value.len()));
    if art.history.len().saturating_add(1) > limits.audit_entries
        || bytes.saturating_add(entry.len()) > limits.source_bytes
    {
        return Err("Studio prospective audit entry exceeds history budget; source preserved".into());
    }
    Ok(())
}

/// Checked mutations use a detached candidate. A rejected edit leaves source,
/// revision, audit and both undo stacks byte-for-byte unchanged.
pub(super) fn edit<F>(art: &mut StoredArtifact, expected: Option<u32>, f: F) -> Result<(), String>
where F: FnOnce(&mut StudioArtifact) {
    try_edit(art, expected, |candidate| { f(candidate); Ok(()) })
}

pub(super) fn try_edit<F>(art: &mut StoredArtifact, expected: Option<u32>, f: F) -> Result<(), String>
where F: FnOnce(&mut StudioArtifact) -> Result<(), String> {
    let revision = admit(art, expected)?;
    let previous = art.lamport_clock;
    let audit = format!("edit:{}->{}", previous, revision);
    admit_audit(art, art.journal.limits, &audit)?;
    if art.journal.undo.len() >= art.journal.limits.steps {
        return Err("Studio undo step budget exhausted; checkpoint required".into());
    }
    let old_bytes = validate_source(art, art.journal.limits)?;
    let retained_bytes: usize = art.journal.undo.iter().map(SourceState::bytes).sum();
    if retained_bytes.saturating_add(old_bytes) > art.journal.limits.journal_bytes {
        return Err("Studio undo payload budget exhausted; checkpoint required".into());
    }
    // Do not recursively copy the journal into a candidate or a snapshot.
    let mut candidate = StudioArtifact {
        id: art.id.clone(), did_q42: art.did_q42, domain: art.domain.clone(),
        title: art.title.clone(), lamport_clock: art.lamport_clock,
        routing_lane: art.routing_lane, sensitivity: art.sensitivity,
        context_did_hash: art.context_did_hash, nodes: art.nodes.clone(),
        properties: art.properties.clone(), history: art.history.clone(),
        redo_stack: art.redo_stack.clone(),
    };
    f(&mut candidate)?;
    if candidate.id != art.id || candidate.did_q42 != art.did_q42 || candidate.domain != art.domain
        || candidate.routing_lane != art.routing_lane || candidate.sensitivity != art.sensitivity
        || candidate.context_did_hash != art.context_did_hash || candidate.history != art.history
        || candidate.redo_stack != art.redo_stack || candidate.lamport_clock != art.lamport_clock
    {
        return Err("Studio source edit cannot change identifiers, policy, revision or audit".into());
    }
    validate_source(&candidate, art.journal.limits)?;
    let prior = SourceState::capture(art);
    let next = SourceState { title: candidate.title, nodes: candidate.nodes, properties: candidate.properties };
    art.journal.undo.push(prior);
    art.journal.redo.clear();
    art.redo_stack.clear();
    next.restore(art);
    art.history.push(audit);
    art.lamport_clock = revision;
    Ok(())
}

pub(super) fn admit_initial(art: &StoredArtifact) -> Result<(), String> {
    if !art.redo_stack.is_empty() {
        return Err("initial Studio redo markers have no source snapshots".into());
    }
    if !(1..=MAX_REVISION).contains(&art.lamport_clock) {
        return Err("invalid initial Studio revision".into());
    }
    validate_source(art, art.journal.limits)?;
    let bytes = art.history.iter().fold(0usize, |sum, value| sum.saturating_add(value.len()));
    if art.history.len() > art.journal.limits.audit_entries || bytes > art.journal.limits.source_bytes {
        return Err("initial Studio history exceeds admitted budget".into());
    }
    Ok(())
}

pub(super) fn restore(art: &mut StoredArtifact, expected: Option<u32>, redo: bool) -> Result<(), String> {
    let revision = admit(art, expected)?;
    let previous = art.lamport_clock;
    let audit = format!("{}:{}->{}", if redo { "redo" } else { "undo" }, previous, revision);
    admit_audit(art, art.journal.limits, &audit)?;
    let source = if redo { &art.journal.redo } else { &art.journal.undo };
    let target = source.last().ok_or_else(|| "Studio has no source snapshot to restore".to_string())?;
    let current_bytes = validate_source(art, art.journal.limits)?;
    // Replacing one retained snapshot with the current source must also fit.
    if journal_bytes(&art.journal).saturating_sub(target.bytes()).saturating_add(current_bytes) > art.journal.limits.journal_bytes {
        return Err("Studio undo payload budget exhausted; source preserved".into());
    }
    let current = SourceState::capture(art);
    let next = if redo {
        let next = art.journal.redo.pop().expect("snapshot admitted");
        art.journal.undo.push(current);
        art.redo_stack.pop();
        next
    } else {
        let next = art.journal.undo.pop().expect("snapshot admitted");
        art.journal.redo.push(current);
        art.redo_stack.push(format!("clock:{}", previous));
        next
    };
    next.restore(art);
    art.history.push(audit);
    art.lamport_clock = revision;
    Ok(())
}

/// Host-only resource admission. It changes no source or authority envelope and
/// never discards undo/audit entries to make a smaller budget appear satisfied.
pub(super) fn configure(art: &mut StoredArtifact, expected: Option<u32>, limits: SourceJournalLimits) -> Result<(), String> {
    if expected.is_some_and(|revision| revision != art.lamport_clock)
        || art.lamport_clock == 0 || art.lamport_clock >= MAX_REVISION
    {
        return Err("Stale, invalid or exhausted Studio revision; limits preserved".into());
    }
    if [limits.source_bytes, limits.journal_bytes, limits.steps, limits.audit_entries, limits.source_fields].contains(&0)
        || art.journal.undo.len() + art.journal.redo.len() > limits.steps
        || art.history.len() >= limits.audit_entries
        || art.history.iter().map(String::len).sum::<usize>() > limits.source_bytes
        || journal_bytes(&art.journal) > limits.journal_bytes
        || art.journal.undo.iter().chain(&art.journal.redo).any(|state| {
            let (bytes, fields) = source_size(&state.title, &state.nodes, &state.properties);
            bytes > limits.source_bytes || fields > limits.source_fields
        })
    {
        return Err("Studio journal limits do not admit retained source/history".into());
    }
    validate_source(art, limits)?;
    let revision = art.lamport_clock + 1;
    let previous = art.lamport_clock;
    let audit = format!("journal_limits:{}->{}", previous, revision);
    admit_audit(art, limits, &audit)?;
    art.journal.limits = limits;
    art.history.push(audit);
    art.lamport_clock = revision;
    Ok(())
}

/// Validate imported journal without trusting its serialized resource limits.
/// Host caps may admit larger masters; checkpoint data cannot enlarge them.
pub(super) fn validate_checkpoint(
    art: &StoredArtifact,
    caps: SourceJournalLimits,
    validate_nodes: impl Fn(&BTreeMap<String, BTreeMap<String, String>>) -> Result<(), String>,
) -> Result<(), String> {
    let limits = art.journal.limits;
    if [limits.source_bytes, limits.journal_bytes, limits.steps, limits.audit_entries, limits.source_fields].contains(&0)
        || limits.source_bytes > caps.source_bytes || limits.journal_bytes > caps.journal_bytes
        || limits.steps > caps.steps || limits.audit_entries > caps.audit_entries
        || limits.source_fields > caps.source_fields
        || art.journal.undo.len().saturating_add(art.journal.redo.len()) > limits.steps
        || art.journal.redo.len() != art.redo_stack.len()
        || journal_bytes(&art.journal) > limits.journal_bytes
        || !(1..=MAX_REVISION).contains(&art.lamport_clock)
    {
        return Err("checkpoint journal exceeds host caps or has inconsistent state".into());
    }
    validate_source(art, limits)?;
    validate_nodes(&art.nodes)?;
    let audit_bytes = art.history.iter().fold(0usize, |sum, text| sum.saturating_add(text.len()));
    if art.history.len() > limits.audit_entries || audit_bytes > limits.source_bytes {
        return Err("checkpoint audit exceeds admitted budget".into());
    }
    for state in art.journal.undo.iter().chain(&art.journal.redo) {
        let (bytes, fields) = source_size(&state.title, &state.nodes, &state.properties);
        if bytes > limits.source_bytes || fields > limits.source_fields {
            return Err("checkpoint retained source exceeds admitted budget".into());
        }
        validate_nodes(&state.nodes)?;
    }
    Ok(())
}
