//! Provenance write helpers — labor DID labelling and contestability.
//!
//! This module provides functions that write PROV-O quins to `PROVENANCE_CONTEXT`
//! and wire contestability disputes into the `DagStore` Merkle-DAG.
//!
//! Named graph contexts:
//!   `PROVENANCE_CONTEXT = q_hash("urn:qualia:context:provenance")`
//!   `CONTEST_CONTEXT    = q_hash("urn:qualia:context:contest")`

use crate::temporal_graph::{P_DC_CREATOR, P_WAS_ATTRIBUTED_TO, P_WAS_GENERATED_BY, P_WAS_INVALIDATED_BY};
use crate::{q_hash, NQuin};

// ── Named-graph contexts ──────────────────────────────────────────────────────
pub const PROVENANCE_CONTEXT: u64 = q_hash("urn:qualia:context:provenance");
pub const CONTEST_CONTEXT: u64 = q_hash("urn:qualia:context:contest");

// ── Provenance predicates ─────────────────────────────────────────────────────
const P_LABELLED_BY: u64 = q_hash("urn:qualia:prov:labelledBy");
const P_LABELLED_AT: u64 = q_hash("urn:qualia:prov:labelledAt");
const P_MODERATED_BY: u64 = q_hash("urn:qualia:prov:moderatedBy");
const P_MODERATED_AT: u64 = q_hash("urn:qualia:prov:moderatedAt");
const P_CLEANED_BY: u64 = q_hash("urn:qualia:prov:cleanedBy");

// ── Contestability predicates ─────────────────────────────────────────────────
const P_CONTESTED_BY: u64 = q_hash("urn:qualia:prov:contestedBy");
const P_CONTEST_REASON: u64 = q_hash("urn:qualia:prov:contestReason");
const P_CONTEST_AT: u64 = q_hash("urn:qualia:prov:contestedAt");
const P_RESOLVED_BY: u64 = q_hash("urn:qualia:prov:resolvedBy");
const P_RESOLVED_AT: u64 = q_hash("urn:qualia:prov:resolvedAt");

// ── Activity predicates ───────────────────────────────────────────────────────
const P_ACTIVITY_LABEL: u64 = q_hash("urn:qualia:prov:activityLabel");
const P_ACTIVITY_START: u64 = q_hash("urn:qualia:prov:activityStart");
const P_ACTIVITY_END: u64 = q_hash("urn:qualia:prov:activityEnd");

// ── Labor provenance ──────────────────────────────────────────────────────────

/// Record that `data_hash` was labelled by `worker_did` at `ts` (ms since epoch).
///
/// Returns 2 quins in `PROVENANCE_CONTEXT`:
///   - `prov:labelledBy` = worker DID hash
///   - `prov:labelledAt` = timestamp
pub fn label_with_worker_did(data_hash: u64, worker_did: u64, ts: u64) -> [NQuin; 2] {
    [
        make_prov(data_hash, P_LABELLED_BY, worker_did, ts),
        make_prov(data_hash, P_LABELLED_AT, ts, ts),
    ]
}

/// Record that `data_hash` was moderated (reviewed + approved or rejected) by `moderator_did`.
pub fn record_moderation(data_hash: u64, moderator_did: u64, ts: u64) -> [NQuin; 2] {
    [
        make_prov(data_hash, P_MODERATED_BY, moderator_did, ts),
        make_prov(data_hash, P_MODERATED_AT, ts, ts),
    ]
}

/// Record that `data_hash` was cleaned (normalised/de-duplicated) by `agent_did`.
pub fn record_cleaning(data_hash: u64, agent_did: u64, ts: u64) -> NQuin {
    make_prov(data_hash, P_CLEANED_BY, agent_did, ts)
}

/// Full PROV-O labor attribution: creator + source + generated-at.
///
/// Returns 3 quins in `PROVENANCE_CONTEXT`:
///   - `dcterms:creator` = worker DID hash
///   - `prov:wasAttributedTo` = worker DID hash
///   - `prov:wasGeneratedBy` = activity hash
pub fn full_attribution(
    data_hash: u64,
    worker_did: u64,
    activity_hash: u64,
    ts: u64,
) -> [NQuin; 3] {
    [
        make_prov(data_hash, P_DC_CREATOR,        worker_did,    ts),
        make_prov(data_hash, P_WAS_ATTRIBUTED_TO, worker_did,    ts),
        make_prov(data_hash, P_WAS_GENERATED_BY,  activity_hash, ts),
    ]
}

// ── Contestability ────────────────────────────────────────────────────────────

/// Record a contestability dispute against `disputed_hash`.
///
/// Returns 3 quins in `CONTEST_CONTEXT`:
///   - `prov:contestedBy` = `agent_did`
///   - `prov:contestReason` = `reason_hash` (q_hash of the reason string)
///   - `prov:contestedAt` = `ts`
///
/// **Also** marks the disputed quin as invalidated in `PROVENANCE_CONTEXT`.
/// Callers that have access to a `DagStore` should additionally call
/// `dag_store.fork_node(disputed_dag_hash, ...)` to register the contestability branch.
pub fn contest_assertion(
    disputed_hash: u64,
    agent_did: u64,
    reason_hash: u64,
    ts: u64,
) -> [NQuin; 4] {
    [
        // Contest record in CONTEST_CONTEXT
        make_contest(disputed_hash, P_CONTESTED_BY,    agent_did,   ts),
        make_contest(disputed_hash, P_CONTEST_REASON,  reason_hash, ts),
        make_contest(disputed_hash, P_CONTEST_AT,      ts,          ts),
        // Invalidation in PROVENANCE_CONTEXT (SPARQL can check this)
        make_prov(disputed_hash, P_WAS_INVALIDATED_BY, agent_did, ts),
    ]
}

/// Record resolution of a dispute.
///
/// Returns 2 quins in `CONTEST_CONTEXT`:
///   - `prov:resolvedBy` = `resolver_did`
///   - `prov:resolvedAt` = `ts`
pub fn resolve_contest(disputed_hash: u64, resolver_did: u64, ts: u64) -> [NQuin; 2] {
    [
        make_contest(disputed_hash, P_RESOLVED_BY, resolver_did, ts),
        make_contest(disputed_hash, P_RESOLVED_AT, ts,           ts),
    ]
}

// ── Activity records ──────────────────────────────────────────────────────────

/// Write an activity record (labelling session, cleaning run, etc.) to `PROVENANCE_CONTEXT`.
///
/// Returns 3 quins:
///   - activity type label
///   - activity start time
///   - activity end time
pub fn write_activity(
    activity_hash: u64,
    label_hash: u64,
    start_ms: u64,
    end_ms: u64,
) -> [NQuin; 3] {
    [
        make_prov(activity_hash, P_ACTIVITY_LABEL, label_hash, start_ms),
        make_prov(activity_hash, P_ACTIVITY_START, start_ms,   start_ms),
        make_prov(activity_hash, P_ACTIVITY_END,   end_ms,     end_ms),
    ]
}

// ── Query helpers ─────────────────────────────────────────────────────────────

/// Return `true` if the quin slice contains any contestability record for `data_hash`.
///
/// Scans `CONTEST_CONTEXT` for a `prov:contestedBy` quin with the given subject.
/// This is an O(n) linear scan — use a proper index in production queries.
pub fn is_contested(quins: &[NQuin], data_hash: u64) -> bool {
    quins.iter().any(|q| {
        q.context == CONTEST_CONTEXT
            && q.subject == data_hash
            && q.predicate == P_CONTESTED_BY
    })
}

/// Collect all worker DIDs that labelled `data_hash`.
pub fn labellers(quins: &[NQuin], data_hash: u64) -> Vec<u64> {
    quins
        .iter()
        .filter(|q| q.context == PROVENANCE_CONTEXT && q.subject == data_hash && q.predicate == P_LABELLED_BY)
        .map(|q| q.object)
        .collect()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

#[inline]
fn make_prov(subject: u64, predicate: u64, object: u64, lamport: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context:  PROVENANCE_CONTEXT,
        metadata: lamport & 0xFFFF_FFFF,
        parity:   0,
    }
}

#[inline]
fn make_contest(subject: u64, predicate: u64, object: u64, lamport: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context:  CONTEST_CONTEXT,
        metadata: lamport & 0xFFFF_FFFF,
        parity:   0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA_HASH: u64 = 0xDA7A_1234_u64;
    const WORKER_DID: u64 = 0x9999_B0B0_u64;
    const DISPUTED: u64 = 0xD157_FEED_u64;
    const AGENT_DID: u64 = 0xA6E4_7777_u64;
    const REASON: u64 = 0x4EA5_0000_u64;
    const WORKER1: u64 = 0xB0B1_u64;
    const WORKER2: u64 = 0xB0B2_u64;

    #[test]
    fn label_produces_correct_context() {
        let qs = label_with_worker_did(DATA_HASH, WORKER_DID, 12345);
        for q in &qs {
            assert_eq!(q.context, PROVENANCE_CONTEXT);
            assert_eq!(q.subject, DATA_HASH);
        }
        assert_eq!(qs[0].predicate, P_LABELLED_BY);
        assert_eq!(qs[0].object, WORKER_DID);
    }

    #[test]
    fn contest_produces_four_quins() {
        let qs = contest_assertion(DISPUTED, AGENT_DID, REASON, 99999);
        assert_eq!(qs.len(), 4);
        assert_eq!(qs[0].context, CONTEST_CONTEXT);
        assert_eq!(qs[1].context, CONTEST_CONTEXT);
        assert_eq!(qs[2].context, CONTEST_CONTEXT);
        assert_eq!(qs[3].context, PROVENANCE_CONTEXT);
    }

    #[test]
    fn is_contested_detects_dispute() {
        let qs = contest_assertion(DISPUTED, AGENT_DID, REASON, 1);
        assert!(is_contested(&qs, DISPUTED));
        assert!(!is_contested(&qs, DATA_HASH));
    }

    #[test]
    fn labellers_returns_worker_dids() {
        let mut all = Vec::new();
        all.extend_from_slice(&label_with_worker_did(DATA_HASH, WORKER1, 1));
        all.extend_from_slice(&label_with_worker_did(DATA_HASH, WORKER2, 2));
        let workers = labellers(&all, DATA_HASH);
        assert_eq!(workers.len(), 2);
        assert!(workers.contains(&WORKER1));
        assert!(workers.contains(&WORKER2));
    }
}
