//! Temporal overlay graph helpers — PROV-O + Dublin Core bi-temporal quins.
//!
//! All temporal quins are written to `T_CONTEXT`. Predicates are FNV-1a hashes
//! of the standard W3C URIs, computed at compile time via `q_hash()`.
//!
//! Named graph context:
//!   `T_CONTEXT = q_hash("urn:qualia:context:temporal")`
//!
//! Bi-temporal model:
//! - **Assertion time** (`t_assert`) — when the system recorded the fact.
//!   Stored as `prov:generatedAtTime`.
//! - **Valid time** (`t_valid_start` / `t_valid_end`) — when the fact was true in reality.
//!   Stored as `prov:startedAtTime` / `prov:endedAtTime`.
//!
//! All timestamps are milliseconds since Unix epoch (u64).

use crate::{q_hash, NQuin};

// ── Named-graph context ───────────────────────────────────────────────────────
pub const T_CONTEXT: u64 = q_hash("urn:qualia:context:temporal");
pub const AGENT_CONTEXT: u64 = q_hash("urn:qualia:context:agent");

// ── PROV-O predicate hashes ───────────────────────────────────────────────────
pub const P_GENERATED_AT: u64 = q_hash("http://www.w3.org/ns/prov#generatedAtTime");
pub const P_STARTED_AT: u64 = q_hash("http://www.w3.org/ns/prov#startedAtTime");
pub const P_ENDED_AT: u64 = q_hash("http://www.w3.org/ns/prov#endedAtTime");
pub const P_WAS_ATTRIBUTED_TO: u64 = q_hash("http://www.w3.org/ns/prov#wasAttributedTo");
pub const P_WAS_GENERATED_BY: u64 = q_hash("http://www.w3.org/ns/prov#wasGeneratedBy");
pub const P_WAS_INVALIDATED_BY: u64 = q_hash("http://www.w3.org/ns/prov#wasInvalidatedBy");
pub const P_INVALIDATED_AT: u64 = q_hash("http://www.w3.org/ns/prov#invalidatedAtTime");
pub const P_HAD_PRIMARY_SOURCE: u64 = q_hash("http://www.w3.org/ns/prov#hadPrimarySource");

// ── Dublin Core predicate hashes ─────────────────────────────────────────────
pub const P_DC_VALID: u64 = q_hash("http://purl.org/dc/terms/valid");
pub const P_DC_CREATOR: u64 = q_hash("http://purl.org/dc/terms/creator");
pub const P_DC_DATE: u64 = q_hash("http://purl.org/dc/terms/date");

// ── RDF type ─────────────────────────────────────────────────────────────────
pub const P_RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");

// ── CogAI class hashes ────────────────────────────────────────────────────────
pub const C_COG_AGENT: u64 = q_hash("https://www.w3.org/community/cogai/ont#Agent");
pub const C_COG_GOAL: u64 = q_hash("https://www.w3.org/community/cogai/ont#Goal");
pub const C_COG_BELIEF: u64 = q_hash("https://www.w3.org/community/cogai/ont#Belief");
pub const C_COG_PLAN: u64 = q_hash("https://www.w3.org/community/cogai/ont#Plan");
pub const P_COG_HOLDS_BELIEF: u64 = q_hash("https://www.w3.org/community/cogai/ont#holdsBelief");
pub const P_COG_HAS_GOAL: u64 = q_hash("https://www.w3.org/community/cogai/ont#hasGoal");
pub const P_COG_HAS_PLAN: u64 = q_hash("https://www.w3.org/community/cogai/ont#hasPlan");

// ── Q42 extension predicates ──────────────────────────────────────────────────
pub const P_Q42_INFERENCE_AUTHORIZED_BY: u64 = q_hash("urn:qualia:ontology:rights:inferenceAuthorizedBy");
pub const P_Q42_PROVENANCE_CITATIONS: u64 = q_hash("urn:qualia:ontology:rights:provenanceCitations");

// ── assert_temporal ───────────────────────────────────────────────────────────

/// Write PROV-O bi-temporal quins for `entity` to `T_CONTEXT`.
///
/// Returns up to 4 quins:
///  1. `prov:generatedAtTime` = `t_assert` (always)
///  2. `prov:startedAtTime` = `t_valid_start` (always)
///  3. `prov:endedAtTime` = `t_valid_end` (if `Some`)
///  4. `prov:wasAttributedTo` = `author_did` (if `Some`)
///
/// All timestamps are ms since Unix epoch.
pub fn assert_temporal(
    entity: u64,
    t_valid_start: u64,
    t_valid_end: Option<u64>,
    t_assert: u64,
    author_did: Option<u64>,
) -> Vec<NQuin> {
    let mut quins = Vec::with_capacity(4);
    quins.push(make_temporal(entity, P_GENERATED_AT, t_assert));
    quins.push(make_temporal(entity, P_STARTED_AT, t_valid_start));
    if let Some(end) = t_valid_end {
        quins.push(make_temporal(entity, P_ENDED_AT, end));
    }
    if let Some(did) = author_did {
        quins.push(make_temporal(entity, P_WAS_ATTRIBUTED_TO, did));
    }
    quins
}

/// Record that an entity was invalidated (tombstoned) at `t_invalidate` by `agent_did`.
pub fn invalidate_entity(entity: u64, agent_did: u64, t_invalidate: u64) -> [NQuin; 2] {
    [
        make_temporal(entity, P_WAS_INVALIDATED_BY, agent_did),
        make_temporal(entity, P_INVALIDATED_AT, t_invalidate),
    ]
}

// ── CogAI agent context quins ─────────────────────────────────────────────────

/// Write `cog:Agent` type quin for a DID hash to `AGENT_CONTEXT`.
pub fn register_agent(did_hash: u64, ts: u64) -> NQuin {
    NQuin {
        subject:   did_hash,
        predicate: P_RDF_TYPE,
        object:    C_COG_AGENT,
        context:   AGENT_CONTEXT,
        metadata:  ts & 0xFFFF_FFFF, // Lamport clock = lower 32 bits of ts
        parity:    0,
    }
}

/// Write `cog:Goal` for `(agent_did, goal_hash)` pair to `AGENT_CONTEXT`.
pub fn write_goal(agent_did: u64, goal_hash: u64, ts: u64) -> [NQuin; 2] {
    [
        NQuin { subject: goal_hash, predicate: P_RDF_TYPE,    object: C_COG_GOAL,  context: AGENT_CONTEXT, metadata: ts & 0xFFFF_FFFF, parity: 0 },
        NQuin { subject: agent_did, predicate: P_COG_HAS_GOAL, object: goal_hash,  context: AGENT_CONTEXT, metadata: ts & 0xFFFF_FFFF, parity: 0 },
    ]
}

/// Write `cog:Belief` with PROV-O provenance for `(agent_did, belief_hash)` pair.
///
/// Returns quins in `T_CONTEXT` (temporal) and `AGENT_CONTEXT` (belief link).
pub fn write_belief(
    agent_did: u64,
    belief_hash: u64,
    t_assert: u64,
    t_valid: u64,
) -> [NQuin; 4] {
    [
        NQuin { subject: belief_hash, predicate: P_RDF_TYPE,         object: C_COG_BELIEF,  context: AGENT_CONTEXT, metadata: t_assert & 0xFFFF_FFFF, parity: 0 },
        NQuin { subject: agent_did,  predicate: P_COG_HOLDS_BELIEF,  object: belief_hash,   context: AGENT_CONTEXT, metadata: t_assert & 0xFFFF_FFFF, parity: 0 },
        NQuin { subject: belief_hash, predicate: P_GENERATED_AT,      object: t_assert,      context: T_CONTEXT,     metadata: 0,                       parity: 0 },
        NQuin { subject: belief_hash, predicate: P_STARTED_AT,        object: t_valid,       context: T_CONTEXT,     metadata: 0,                       parity: 0 },
    ]
}

// ── Internal helpers ──────────────────────────────────────────────────────────

#[inline]
fn make_temporal(subject: u64, predicate: u64, object: u64) -> NQuin {
    NQuin { subject, predicate, object, context: T_CONTEXT, metadata: 0, parity: 0 }
}

// ── Fallback: no smallvec dependency ─────────────────────────────────────────
// If smallvec is not in the dependency tree, fall back to Vec.
// The module re-exports a `TempQuins` type alias so callers don't need to know.

/// Variable-length return type for `assert_temporal`.
pub type TempQuins = Vec<NQuin>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_temporal_all_fields() {
        let qs = assert_temporal(0x1111, 1_000, Some(2_000), 3_000, Some(0xDEAD));
        assert_eq!(qs.len(), 4);
        assert_eq!(qs[0].predicate, P_GENERATED_AT);
        assert_eq!(qs[0].object,    3_000);
        assert_eq!(qs[1].predicate, P_STARTED_AT);
        assert_eq!(qs[1].object,    1_000);
        assert_eq!(qs[2].predicate, P_ENDED_AT);
        assert_eq!(qs[2].object,    2_000);
        assert_eq!(qs[3].predicate, P_WAS_ATTRIBUTED_TO);
        assert_eq!(qs[3].object,    0xDEAD);
        for q in &qs { assert_eq!(q.context, T_CONTEXT); }
    }

    #[test]
    fn assert_temporal_no_end_no_author() {
        let qs = assert_temporal(0x1234, 500, None, 600, None);
        assert_eq!(qs.len(), 2);
    }

    #[test]
    fn cogai_belief_has_temporal_and_agent_context() {
        let qs = write_belief(0xAAAA, 0xBBBB, 9_000, 8_000);
        let t_ctx: Vec<_> = qs.iter().filter(|q| q.context == T_CONTEXT).collect();
        let a_ctx: Vec<_> = qs.iter().filter(|q| q.context == AGENT_CONTEXT).collect();
        assert_eq!(t_ctx.len(), 2);
        assert_eq!(a_ctx.len(), 2);
    }

    #[test]
    fn all_predicate_hashes_distinct() {
        let predicates = [
            P_GENERATED_AT, P_STARTED_AT, P_ENDED_AT, P_WAS_ATTRIBUTED_TO,
            P_WAS_GENERATED_BY, P_WAS_INVALIDATED_BY, P_INVALIDATED_AT,
            P_DC_VALID, P_DC_CREATOR,
        ];
        let mut seen = std::collections::HashSet::new();
        for p in &predicates {
            assert!(seen.insert(p), "duplicate predicate hash: {p}");
        }
    }
}
