//! Versioned, replay-safe sync-operation protocol and quarantined inbox.
//!
//! This is the delivery layer that lets Projects/Finance (and any domain) converge across
//! nodes without duplicating money or obligations. It implements the master plan's SyncService
//! (§4.2), link-protocol framing (§9.5), and the money-safety invariants (§17):
//!
//! - every operation is **versioned** (protocol + schema) and **content-hashed**;
//! - the inbox is **quarantined**: untrusted frames are decoded into this DTO and validated
//!   before anything is admitted — oversized, malformed, unsigned, wrong-hash, wrong-version,
//!   and Sanctuary-classified frames are **rejected fail-closed**;
//! - admission is **idempotent**: a replayed `operation_id` is recorded as `Duplicate`, never
//!   applied twice;
//! - [`merge_operations`] is **add-wins by operation id** and **order-independent**, so
//!   duplicate/reordered/replayed frames converge to the same set — the same discipline the
//!   domain layers (`finance::derived_balance`, `projects::derive_obligations`) use to derive
//!   totals purely over the unique-id set.
//!
//! Full signature *verification* is the identity/key-vault layer's job (it holds the actor
//! public keys); this layer verifies presence + integrity and enforces the routing lane.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CURRENT_PROTOCOL_VERSION: u16 = 1;
pub const CURRENT_SCHEMA_VERSION: u16 = 1;
/// Hard cap on a single serialized operation (defends the quarantine against oversized frames).
pub const MAX_OPERATION_BYTES: usize = 64 * 1024;
/// Hard cap on the payload summary carried inline.
pub const MAX_SUMMARY_BYTES: usize = 16 * 1024;

pub const SYNC_INBOX_FILE: &str = "wellfair/sync_inbox.jsonl";

/// A single versioned, content-addressed sync operation (the wire DTO).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncOperation {
    pub protocol_version: u16,
    pub schema_version: u16,
    /// Stable operation identifier — the dedup/idempotency anchor.
    pub operation_id: String,
    pub record_id: String,
    pub kind: String,
    /// SHA-256 (hex) of `payload_summary` — integrity check on receipt.
    pub content_hash: String,
    /// Lamport clock for causal ordering across nodes.
    pub lamport: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_op_id: Option<String>,
    pub actor_did: String,
    /// Routing lane: "Public" | "Restricted" | "Classified".
    pub sensitivity: String,
    /// Approved projection / journal summary carried inline (no sensitive plaintext for Sanctuary).
    pub payload_summary: String,
    pub committed_unix: u32,
    /// Detached signature (hex) over [`SyncOperation::signing_payload`]. Presence is required
    /// (fail-closed); cryptographic verification is performed by the identity layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Outcome of validating/admitting an inbound operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "reason")]
pub enum AdmitOutcome {
    /// Passed all checks and is newly admitted.
    Validated,
    /// A prior operation with this id was already admitted; ignored (idempotent replay).
    Duplicate,
    /// Failed a validation check; carries the reason.
    Rejected(String),
}

impl AdmitOutcome {
    pub fn is_validated(&self) -> bool {
        matches!(self, AdmitOutcome::Validated)
    }
    pub fn is_rejected(&self) -> bool {
        matches!(self, AdmitOutcome::Rejected(_))
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

impl SyncOperation {
    /// Build a well-formed, content-hashed operation (signature filled in by the host layer).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: impl Into<String>,
        record_id: impl Into<String>,
        kind: impl Into<String>,
        actor_did: impl Into<String>,
        sensitivity: impl Into<String>,
        payload_summary: impl Into<String>,
        lamport: u64,
        committed_unix: u32,
    ) -> Self {
        let payload_summary = payload_summary.into();
        let content_hash = sha256_hex(payload_summary.as_bytes());
        Self {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            schema_version: CURRENT_SCHEMA_VERSION,
            operation_id: operation_id.into(),
            record_id: record_id.into(),
            kind: kind.into(),
            content_hash,
            lamport,
            parent_op_id: None,
            actor_did: actor_did.into(),
            sensitivity: sensitivity.into(),
            payload_summary,
            committed_unix,
            signature: None,
        }
    }

    /// The bytes a signature must cover: id + record + content hash (binds identity to content).
    pub fn signing_payload(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}",
            self.operation_id, self.record_id, self.content_hash
        )
        .into_bytes()
    }

    pub fn with_signature(mut self, signature_hex: impl Into<String>) -> Self {
        self.signature = Some(signature_hex.into());
        self
    }
}

/// Validate an inbound operation fail-closed. `seen_ids` is the set of already-admitted
/// operation ids (for replay detection). Returns the admission outcome without persisting.
pub fn validate_operation(op: &SyncOperation, seen_ids: &HashSet<String>) -> AdmitOutcome {
    // Version gate: refuse anything we don't understand.
    if op.protocol_version != CURRENT_PROTOCOL_VERSION {
        return AdmitOutcome::Rejected(format!(
            "unsupported protocol version {} (expected {CURRENT_PROTOCOL_VERSION})",
            op.protocol_version
        ));
    }
    if op.schema_version != CURRENT_SCHEMA_VERSION {
        return AdmitOutcome::Rejected(format!(
            "unsupported schema version {} (expected {CURRENT_SCHEMA_VERSION})",
            op.schema_version
        ));
    }
    // Size bounds (quarantine defense).
    if op.payload_summary.len() > MAX_SUMMARY_BYTES {
        return AdmitOutcome::Rejected("payload summary exceeds size cap".into());
    }
    match serde_json::to_string(op) {
        Ok(s) if s.len() > MAX_OPERATION_BYTES => {
            return AdmitOutcome::Rejected("operation exceeds size cap".into());
        }
        Err(e) => return AdmitOutcome::Rejected(format!("operation not serializable: {e}")),
        _ => {}
    }
    // Signature must be present (fail closed); full verification is the identity layer's job.
    match &op.signature {
        Some(sig) if !sig.is_empty() => {}
        _ => return AdmitOutcome::Rejected("missing signature (fail closed)".into()),
    }
    // Integrity: the content hash must match the carried payload.
    if op.content_hash != sha256_hex(op.payload_summary.as_bytes()) {
        return AdmitOutcome::Rejected("content hash does not match payload".into());
    }
    // Routing lane: Sanctuary/Classified operations must never traverse the ordinary inbox (§5.2).
    if op.sensitivity == "Classified" {
        return AdmitOutcome::Rejected(
            "Classified/Sanctuary operations are excluded from the ordinary sync lane".into(),
        );
    }
    // Replay: a previously-admitted id is idempotently ignored.
    if seen_ids.contains(&op.operation_id) {
        return AdmitOutcome::Duplicate;
    }
    AdmitOutcome::Validated
}

/// Next Lamport clock value given the local counter and an observed remote value.
pub fn lamport_next(local: u64, observed: u64) -> u64 {
    local.max(observed).saturating_add(1)
}

/// Merge two operation sets **add-wins by operation id** (never re-apply), returning a
/// deterministically ordered union (by Lamport clock, then id). Idempotent and order-independent.
pub fn merge_operations(
    existing: &[SyncOperation],
    incoming: &[SyncOperation],
) -> Vec<SyncOperation> {
    let mut merged: Vec<SyncOperation> = Vec::with_capacity(existing.len() + incoming.len());
    for op in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.operation_id == op.operation_id) {
            merged.push(op.clone());
        }
    }
    merged.sort_by(|a, b| {
        a.lamport
            .cmp(&b.lamport)
            .then_with(|| a.operation_id.cmp(&b.operation_id))
    });
    merged
}

/// A persisted inbox record: the operation plus its admission outcome and time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxRecord {
    pub operation: SyncOperation,
    pub outcome: AdmitOutcome,
    pub admitted_unix: u32,
}

/// Durable quarantined inbox (append-only jsonl). Admission validates, dedupes by operation id,
/// and records the outcome; only `Validated` records represent applicable operations.
pub struct SyncInbox {
    path: PathBuf,
}

impl SyncInbox {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(SYNC_INBOX_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        Ok(Self { path })
    }

    fn load_all(&self) -> std::io::Result<Vec<InboxRecord>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(rec) = serde_json::from_str::<InboxRecord>(&line) {
                records.push(rec);
            }
        }
        Ok(records)
    }

    /// The set of operation ids already admitted as Validated (for replay detection).
    fn admitted_ids(records: &[InboxRecord]) -> HashSet<String> {
        records
            .iter()
            .filter(|r| r.outcome.is_validated())
            .map(|r| r.operation.operation_id.clone())
            .collect()
    }

    /// Validate and durably record an inbound operation. Idempotent: a replayed id yields
    /// `Duplicate` and is not applied again. Returns the admission outcome.
    pub fn admit(&self, op: &SyncOperation, now_unix: u32) -> std::io::Result<AdmitOutcome> {
        let existing = self.load_all()?;
        let seen = Self::admitted_ids(&existing);
        let outcome = validate_operation(op, &seen);
        let record = InboxRecord {
            operation: op.clone(),
            outcome: outcome.clone(),
            admitted_unix: now_unix,
        };
        let line =
            serde_json::to_string(&record).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(outcome)
    }

    /// All admitted-`Validated` operations, in Lamport order (the applicable set).
    pub fn validated_operations(&self) -> std::io::Result<Vec<SyncOperation>> {
        let mut ops: Vec<SyncOperation> = self
            .load_all()?
            .into_iter()
            .filter(|r| r.outcome.is_validated())
            .map(|r| r.operation)
            .collect();
        // Collapse any accidental duplicates and order deterministically.
        Ok(merge_operations(&ops.split_off(0), &[]))
    }

    pub fn list_recent(&self, limit: usize) -> std::io::Result<Vec<InboxRecord>> {
        let mut all = self.load_all()?;
        if all.len() > limit {
            all.drain(0..all.len() - limit);
        }
        all.reverse();
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed(id: &str, kind: &str, summary: &str, lamport: u64) -> SyncOperation {
        SyncOperation::new(
            id,
            format!("urn:wellfair:{kind}:{id}"),
            kind,
            "did:wf:remote",
            "Restricted",
            summary,
            lamport,
            1_700_000_000,
        )
        .with_signature("deadbeef")
    }

    #[test]
    fn valid_operation_admitted() {
        let seen = HashSet::new();
        let op = signed("op1", "ledger_entry", "{\"amount_cents\":100}", 1);
        assert_eq!(validate_operation(&op, &seen), AdmitOutcome::Validated);
    }

    #[test]
    fn replayed_id_is_duplicate() {
        let mut seen = HashSet::new();
        seen.insert("op1".to_string());
        let op = signed("op1", "ledger_entry", "x", 1);
        assert_eq!(validate_operation(&op, &seen), AdmitOutcome::Duplicate);
    }

    #[test]
    fn missing_signature_rejected() {
        let seen = HashSet::new();
        let mut op = signed("op2", "ledger_entry", "x", 1);
        op.signature = None;
        assert!(validate_operation(&op, &seen).is_rejected());
    }

    #[test]
    fn tampered_content_hash_rejected() {
        let seen = HashSet::new();
        let mut op = signed("op3", "ledger_entry", "original", 1);
        op.payload_summary = "tampered".into(); // hash no longer matches
        assert!(validate_operation(&op, &seen).is_rejected());
    }

    #[test]
    fn classified_operation_rejected_from_ordinary_lane() {
        let seen = HashSet::new();
        let mut op = signed("op4", "sanctuary_note", "x", 1);
        op.sensitivity = "Classified".into();
        assert!(validate_operation(&op, &seen).is_rejected());
    }

    #[test]
    fn wrong_protocol_version_rejected() {
        let seen = HashSet::new();
        let mut op = signed("op5", "ledger_entry", "x", 1);
        op.protocol_version = 99;
        assert!(validate_operation(&op, &seen).is_rejected());
    }

    #[test]
    fn oversized_summary_rejected() {
        let seen = HashSet::new();
        let big = "a".repeat(MAX_SUMMARY_BYTES + 1);
        let op = signed("op6", "ledger_entry", &big, 1);
        assert!(validate_operation(&op, &seen).is_rejected());
    }

    #[test]
    fn lamport_next_is_monotonic() {
        assert_eq!(lamport_next(3, 5), 6);
        assert_eq!(lamport_next(7, 2), 8);
        assert!(lamport_next(u64::MAX, u64::MAX) >= u64::MAX);
    }

    #[test]
    fn merge_is_idempotent_and_order_independent() {
        let a = vec![
            signed("a", "ledger_entry", "1", 2),
            signed("b", "ledger_entry", "2", 1),
        ];
        let incoming = vec![
            signed("b", "ledger_entry", "2", 1),
            signed("c", "ledger_entry", "3", 3),
            signed("b", "ledger_entry", "2", 1), // duplicate
        ];
        let merged = merge_operations(&a, &incoming);
        assert_eq!(merged.len(), 3);
        // Lamport order: b(1), a(2), c(3)
        assert_eq!(merged[0].operation_id, "b");
        assert_eq!(merged[1].operation_id, "a");
        assert_eq!(merged[2].operation_id, "c");
        // Order independence + idempotency.
        let other = merge_operations(&incoming, &a);
        assert_eq!(merged, other);
        let twice = merge_operations(&merge_operations(&a, &incoming), &incoming);
        assert_eq!(merged, twice);
    }

    #[test]
    fn inbox_dedupes_replayed_operations() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = SyncInbox::open(dir.path()).unwrap();
        let op = signed("op-replay", "ledger_entry", "{\"amount_cents\":500}", 1);

        assert_eq!(inbox.admit(&op, 10).unwrap(), AdmitOutcome::Validated);
        // Replay of the same op id is idempotent.
        assert_eq!(inbox.admit(&op, 11).unwrap(), AdmitOutcome::Duplicate);
        assert_eq!(inbox.admit(&op, 12).unwrap(), AdmitOutcome::Duplicate);

        // Only one validated operation exists despite three admissions.
        assert_eq!(inbox.validated_operations().unwrap().len(), 1);
    }

    #[test]
    fn inbox_survives_reopen_and_orders_by_lamport() {
        let dir = tempfile::tempdir().unwrap();
        {
            let inbox = SyncInbox::open(dir.path()).unwrap();
            inbox
                .admit(&signed("z", "ledger_entry", "z", 5), 1)
                .unwrap();
            inbox
                .admit(&signed("y", "ledger_entry", "y", 2), 1)
                .unwrap();
        }
        let reopened = SyncInbox::open(dir.path()).unwrap();
        let ops = reopened.validated_operations().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_id, "y"); // lamport 2 before 5
        assert_eq!(ops[1].operation_id, "z");
    }

    #[test]
    fn two_node_partition_rejoin_converges() {
        // Node A and Node B each admit a disjoint op plus a shared op; after exchanging
        // operation sets both nodes hold the identical validated set.
        let shared = signed("shared", "contribution", "s", 1);
        let a_only = signed("a1", "contribution", "a", 2);
        let b_only = signed("b1", "contribution", "b", 3);

        let node_a = vec![shared.clone(), a_only.clone()];
        let node_b = vec![shared.clone(), b_only.clone()];

        let a_after = merge_operations(&node_a, &node_b);
        let b_after = merge_operations(&node_b, &node_a);
        assert_eq!(a_after, b_after);
        assert_eq!(a_after.len(), 3);
    }
}
