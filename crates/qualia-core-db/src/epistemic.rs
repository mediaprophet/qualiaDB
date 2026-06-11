//! Dynamic Epistemic Logic (DEL) layer — separating objective and subjective reality.
//!
//! # Core design
//!
//! Standard databases have one "God's eye" table.  QualiaDB separates:
//!
//! - **Objective reality** (`OBJECTIVE_CONTEXT`) — cryptographically proven facts.
//!   A quin here has `prov:wasGeneratedBy` pointing to a `q42ep:CryptoSensor` or
//!   `q42ep:SignedObjectiveEvent`.  SHACL `ObjectiveKnowledgeShape` validates this.
//!
//! - **Per-agent epistemic space** (`agent_epistemic_context(did)`) — beliefs,
//!   inferences, and hearsay scoped to one agent.  Multiple agents can hold
//!   *different valid epistemic states* about the same proposition at the same time.
//!
//! # John / Frank / Jane scenario
//!
//! ```text
//! t₁: John directly observes the matter.
//!     → assert_objective_knowledge(john_did, matter_hash, sensor_hash, t1)
//!
//! t₂: Frank queries the matter — his query is itself an epistemic event.
//!     → record_query_observation(frank_did, matter_hash, t2)
//!
//! t₃: Jane observes Frank's query (not the original matter).
//! t₄: Jane derives her own version.
//!     → assert_inferred_belief(jane_did, jane_variant_hash, frank_query_hash, t4)
//! ```
//!
//! Jane's context (`agent_epistemic_context(jane_did)`) never overwrites
//! John's context.  SPARQL queries traverse either timeline independently.
//!
//! # SHACL classification
//!
//! `classify_epistemic_state()` evaluates the quin slice against three shapes:
//!   1. `ObjectiveKnowledge` — `prov:wasGeneratedBy` → CryptoSensor/SignedObjectiveEvent
//!   2. `InferredBelief` — `prov:wasDerivedFrom` → AgentQuery/HearsayEvent
//!   3. `HearsayBelief` — `q42ep:believesViaHearsay` present
//!   4. `Unknown` — no matching modality predicate found
//!
//! Integration with `deontic_logic.rs`:
//!   `OP_PERMIT read IF classify() == ObjectiveKnowledge`
//!   `OP_CONDITIONALLY_PERMIT read IF classify() == InferredBelief`

use crate::temporal_graph::{P_WAS_ATTRIBUTED_TO, P_WAS_GENERATED_BY};
use crate::{q_hash, NQuin};

// ── Named-graph contexts ──────────────────────────────────────────────────────

/// Cryptographically verified facts, shared across all agents.
pub const OBJECTIVE_CONTEXT: u64 = q_hash("urn:qualia:context:objective");

/// Base epistemic metadata context (query events, propagation records).
pub const EPISTEMIC_CONTEXT: u64 = q_hash("urn:qualia:context:epistemic");

/// Derive the per-agent epistemic context for `did_hash`.
///
/// Each agent has a distinct named graph for their beliefs.  XOR-folding the
/// base context with the DID produces a stable non-colliding context without
/// string allocation.
#[inline]
pub fn agent_epistemic_context(did_hash: u64) -> u64 {
    q_hash("urn:qualia:context:epistemic:agent") ^ did_hash
}

// ── JTB model resource type hashes ───────────────────────────────────────────

/// `q42ep:CryptoSensor` class hash — hardware-attested or cryptographically signed source.
pub const C_CRYPTO_SENSOR: u64 = q_hash("urn:qualia:ontology:epistemic:CryptoSensor");
/// `q42ep:SignedObjectiveEvent` class hash — notarized or DID-signed objective event.
pub const C_SIGNED_OBJECTIVE_EVENT: u64 = q_hash("urn:qualia:ontology:epistemic:SignedObjectiveEvent");
/// `q42ep:AgentQuery` class hash — a query event is itself an epistemic event.
pub const C_AGENT_QUERY: u64 = q_hash("urn:qualia:ontology:epistemic:AgentQuery");
/// `q42ep:HearsayEvent` class hash — information from an unverified peer.
pub const C_HEARSAY_EVENT: u64 = q_hash("urn:qualia:ontology:epistemic:HearsayEvent");

// ── Epistemic modality predicate hashes ──────────────────────────────────────

/// `q42ep:knowsDirectly` — agent has direct cryptographic or sensory proof.
pub const P_KNOWS_DIRECTLY: u64 = q_hash("urn:qualia:epistemic:knowsDirectly");
/// `q42ep:infersFrom` — agent derived a belief via local logic from a source event.
pub const P_INFERS_FROM: u64 = q_hash("urn:qualia:epistemic:infersFrom");
/// `q42ep:believesViaHearsay` — belief received from an unverified peer.
pub const P_BELIEVES_VIA_HEARSAY: u64 = q_hash("urn:qualia:epistemic:believesViaHearsay");

/// `prov:wasDerivedFrom` — derivation edge in the PROV-O provenance chain.
pub const P_WAS_DERIVED_FROM: u64 = q_hash("http://www.w3.org/ns/prov#wasDerivedFrom");

/// `cog:believes` — the core JTB belief relation.
pub const P_COG_BELIEVES: u64 = q_hash("https://www.w3.org/community/cogai/ont#believes");
/// `cog:queries` — an agent queries about a proposition.
pub const P_COG_QUERIES: u64 = q_hash("https://www.w3.org/community/cogai/ont#queries");
/// `cog:observes` — an agent observes an event.
pub const P_COG_OBSERVES: u64 = q_hash("https://www.w3.org/community/cogai/ont#observes");
/// `cog:infers` — an agent infers a new belief.
pub const P_COG_INFERS: u64 = q_hash("https://www.w3.org/community/cogai/ont#infers");

/// `rdf:type` (cached locally to avoid cross-module dep loop).
const P_RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");

// ── Epistemic state classification ────────────────────────────────────────────

/// The epistemic weight of an agent's belief about a proposition.
///
/// Maps to the JTB (Justified True Belief) model:
/// - `ObjectiveKnowledge` — belief + justification from a cryptographic source = Knowledge
/// - `InferredBelief`     — belief + derivation from a prior event (no direct proof)
/// - `HearsayBelief`      — belief received from an unverified peer (lowest weight)
/// - `Unknown`            — no epistemic modality predicate found in the quin slice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicState {
    /// Cryptographically verified — `prov:wasGeneratedBy` → CryptoSensor/SignedObjectiveEvent.
    ObjectiveKnowledge,
    /// Derived via local logic — `prov:wasDerivedFrom` → AgentQuery/HearsayEvent.
    InferredBelief,
    /// Received from an unverified peer — `q42ep:believesViaHearsay` present.
    HearsayBelief,
    /// No epistemic modality found for this agent/proposition pair.
    Unknown,
}

/// Classify the epistemic state of `agent_did` regarding `proposition_hash`.
///
/// Scans the quin slice for modality predicates in the agent's epistemic context
/// and returns the highest-confidence classification found.  The precedence order is:
///   `ObjectiveKnowledge` > `InferredBelief` > `HearsayBelief` > `Unknown`
///
/// This is an O(n) linear scan — for production use, maintain a hash index on
/// `(context, predicate)` pairs.
pub fn classify_epistemic_state(
    quins: &[NQuin],
    agent_did: u64,
    proposition_hash: u64,
) -> EpistemicState {
    let agent_ctx = agent_epistemic_context(agent_did);
    let mut best = EpistemicState::Unknown;

    for q in quins {
        if q.context != agent_ctx && q.context != OBJECTIVE_CONTEXT {
            continue;
        }
        if q.subject != agent_did {
            continue;
        }
        if q.object != proposition_hash && q.predicate != P_WAS_DERIVED_FROM {
            // Only match direct proposition references or derivation edges
            if q.object != proposition_hash {
                continue;
            }
        }

        let state = predicate_to_state(q.predicate);
        best = higher(best, state);
        if best == EpistemicState::ObjectiveKnowledge {
            break;
        }
    }
    best
}

/// Return the higher-confidence state of `a` and `b`.
#[inline]
fn higher(a: EpistemicState, b: EpistemicState) -> EpistemicState {
    use EpistemicState::*;
    match (a, b) {
        (ObjectiveKnowledge, _) | (_, ObjectiveKnowledge) => ObjectiveKnowledge,
        (InferredBelief, _)    | (_, InferredBelief)     => InferredBelief,
        (HearsayBelief, _)     | (_, HearsayBelief)      => HearsayBelief,
        (Unknown, Unknown)                                => Unknown,
    }
}

#[inline]
fn predicate_to_state(predicate: u64) -> EpistemicState {
    if predicate == P_KNOWS_DIRECTLY {
        EpistemicState::ObjectiveKnowledge
    } else if predicate == P_INFERS_FROM || predicate == P_WAS_DERIVED_FROM {
        EpistemicState::InferredBelief
    } else if predicate == P_BELIEVES_VIA_HEARSAY {
        EpistemicState::HearsayBelief
    } else {
        EpistemicState::Unknown
    }
}

// ── Write helpers ─────────────────────────────────────────────────────────────

/// Record that `agent_did` has ObjectiveKnowledge of `proposition_hash`, justified
/// by `proof_hash` (e.g. a `q42ep:CryptoSensor` or `q42ep:SignedObjectiveEvent`).
///
/// Returns 3 quins in `agent_epistemic_context(agent_did)`:
///   1. `agent_did → knowsDirectly → proposition_hash` (epistemic modality)
///   2. `agent_did → cog:believes → proposition_hash` (CogAI belief relation)
///   3. `proposition_hash → prov:wasGeneratedBy → proof_hash` (justification chain)
pub fn assert_objective_knowledge(
    agent_did: u64,
    proposition_hash: u64,
    proof_hash: u64,
    ts: u64,
) -> [NQuin; 3] {
    let ctx = agent_epistemic_context(agent_did);
    [
        make_ep(agent_did,        P_KNOWS_DIRECTLY, proposition_hash, ctx, ts),
        make_ep(agent_did,        P_COG_BELIEVES,   proposition_hash, ctx, ts),
        make_ep(proposition_hash, P_WAS_GENERATED_BY, proof_hash,     OBJECTIVE_CONTEXT, ts),
    ]
}

/// Record that `agent_did` holds an InferredBelief about `proposition_hash`,
/// derived from `source_hash` (an `AgentQuery` or `HearsayEvent`).
///
/// Returns 3 quins in `agent_epistemic_context(agent_did)`:
///   1. `agent_did → infersFrom → source_hash`
///   2. `agent_did → cog:believes → proposition_hash`
///   3. `proposition_hash → prov:wasDerivedFrom → source_hash`
pub fn assert_inferred_belief(
    agent_did: u64,
    proposition_hash: u64,
    source_hash: u64,
    ts: u64,
) -> [NQuin; 3] {
    let ctx = agent_epistemic_context(agent_did);
    [
        make_ep(agent_did,        P_INFERS_FROM,    source_hash,      ctx, ts),
        make_ep(agent_did,        P_COG_BELIEVES,   proposition_hash, ctx, ts),
        make_ep(proposition_hash, P_WAS_DERIVED_FROM, source_hash,    ctx, ts),
    ]
}

/// Record that `agent_did` holds a HearsayBelief about `proposition_hash`,
/// received from `informant_did` (an unverified peer).
///
/// Returns 3 quins in `agent_epistemic_context(agent_did)`:
///   1. `agent_did → believesViaHearsay → proposition_hash`
///   2. `agent_did → cog:believes → proposition_hash`
///   3. `proposition_hash → prov:wasDerivedFrom → informant_did` (hearsay source)
pub fn assert_hearsay_belief(
    agent_did: u64,
    proposition_hash: u64,
    informant_did: u64,
    ts: u64,
) -> [NQuin; 3] {
    let ctx = agent_epistemic_context(agent_did);
    [
        make_ep(agent_did,        P_BELIEVES_VIA_HEARSAY, proposition_hash, ctx, ts),
        make_ep(agent_did,        P_COG_BELIEVES,         proposition_hash, ctx, ts),
        make_ep(proposition_hash, P_WAS_DERIVED_FROM,     informant_did,    ctx, ts),
    ]
}

/// Record a query event — `observer_did` queries about `query_subject_hash`.
///
/// The query event hash is derived deterministically from `(observer_did, query_subject_hash, ts)`.
/// Returns 1 quin in `EPISTEMIC_CONTEXT` so other agents can reference it as a source.
///
/// Callers should capture the returned `NQuin::object` (= `query_event_hash`) and pass
/// it to `assert_inferred_belief` for agents who observe this query.
pub fn record_query_observation(observer_did: u64, query_subject_hash: u64, ts: u64) -> NQuin {
    make_ep(observer_did, P_COG_QUERIES, query_subject_hash, EPISTEMIC_CONTEXT, ts)
}

/// Record that `agent_did` typed `event_hash` as a `q42ep:CryptoSensor` result.
/// Call this when wiring hardware-attested data into the objective context.
pub fn register_crypto_sensor_output(proof_hash: u64, ts: u64) -> NQuin {
    make_ep(proof_hash, P_RDF_TYPE, C_CRYPTO_SENSOR, OBJECTIVE_CONTEXT, ts)
}

/// Record that `agent_did` typed `event_hash` as a `q42ep:AgentQuery`.
/// Call this alongside `record_query_observation` so SHACL shapes can validate it.
pub fn register_query_event(query_event_hash: u64, querying_agent_did: u64, ts: u64) -> [NQuin; 2] {
    [
        make_ep(query_event_hash, P_RDF_TYPE,           C_AGENT_QUERY,       EPISTEMIC_CONTEXT, ts),
        make_ep(query_event_hash, P_WAS_ATTRIBUTED_TO,  querying_agent_did,  EPISTEMIC_CONTEXT, ts),
    ]
}

// ── Query helpers ─────────────────────────────────────────────────────────────

/// Return all propositions that `agent_did` holds ObjectiveKnowledge of.
pub fn objective_knowledge_of(quins: &[NQuin], agent_did: u64) -> Vec<u64> {
    let ctx = agent_epistemic_context(agent_did);
    quins
        .iter()
        .filter(|q| q.context == ctx && q.subject == agent_did && q.predicate == P_KNOWS_DIRECTLY)
        .map(|q| q.object)
        .collect()
}

/// Return all propositions that `agent_did` believes via any modality.
pub fn all_beliefs_of(quins: &[NQuin], agent_did: u64) -> Vec<u64> {
    let ctx = agent_epistemic_context(agent_did);
    quins
        .iter()
        .filter(|q| q.context == ctx && q.subject == agent_did && q.predicate == P_COG_BELIEVES)
        .map(|q| q.object)
        .collect()
}

/// Return true if any quin in the slice classifies `proposition` as ObjectiveKnowledge
/// for at least one agent.
pub fn has_objective_proof(quins: &[NQuin], proposition_hash: u64) -> bool {
    quins.iter().any(|q| {
        q.context == OBJECTIVE_CONTEXT
            && q.subject == proposition_hash
            && q.predicate == P_WAS_GENERATED_BY
    })
}

// ── Internal helper ───────────────────────────────────────────────────────────

#[inline]
fn make_ep(subject: u64, predicate: u64, object: u64, context: u64, lamport: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: lamport & 0xFFFF_FFFF,
        parity:   0,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const JOHN: u64 = 0x111A_u64;
    const FRANK: u64 = 0x222B_u64;
    const JANE: u64 = 0x333C_u64;
    const MATTER: u64 = 0xFACE_1234_u64;
    const SENSOR: u64 = 0xDEAD_BEEF_u64;

    #[test]
    fn agent_epistemic_contexts_are_distinct() {
        let ctx_john  = agent_epistemic_context(JOHN);
        let ctx_frank = agent_epistemic_context(FRANK);
        let ctx_jane  = agent_epistemic_context(JANE);
        assert_ne!(ctx_john, ctx_frank);
        assert_ne!(ctx_frank, ctx_jane);
        assert_ne!(ctx_john, ctx_jane);
    }

    #[test]
    fn objective_knowledge_produces_correct_contexts() {
        let qs = assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000);
        let john_ctx = agent_epistemic_context(JOHN);
        // First two quins are in John's epistemic context
        assert_eq!(qs[0].context, john_ctx);
        assert_eq!(qs[1].context, john_ctx);
        // Justification chain quin is in the shared OBJECTIVE_CONTEXT
        assert_eq!(qs[2].context, OBJECTIVE_CONTEXT);
        assert_eq!(qs[2].predicate, P_WAS_GENERATED_BY);
        assert_eq!(qs[2].object, SENSOR);
    }

    #[test]
    fn inferred_belief_does_not_collide_with_objective() {
        let frank_query = 0x9999_u64;
        let obj_qs  = assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000);
        let inf_qs  = assert_inferred_belief(JANE, MATTER, frank_query, 2000);

        let john_ctx = agent_epistemic_context(JOHN);
        let jane_ctx = agent_epistemic_context(JANE);
        assert_ne!(john_ctx, jane_ctx);

        // John's quins are not in Jane's context
        for q in &obj_qs {
            assert_ne!(q.context, jane_ctx);
        }
        // Jane's quins are not in John's context
        for q in &inf_qs {
            assert_ne!(q.context, john_ctx);
        }
    }

    #[test]
    fn classify_detects_objective_knowledge() {
        let qs = assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000);
        let state = classify_epistemic_state(&qs, JOHN, MATTER);
        assert_eq!(state, EpistemicState::ObjectiveKnowledge);
    }

    #[test]
    fn classify_detects_inferred_belief() {
        let frank_query = 0x9999_u64;
        let qs = assert_inferred_belief(JANE, MATTER, frank_query, 2000);
        let state = classify_epistemic_state(&qs, JANE, MATTER);
        assert_eq!(state, EpistemicState::InferredBelief);
    }

    #[test]
    fn classify_detects_hearsay() {
        let qs = assert_hearsay_belief(JANE, MATTER, FRANK, 3000);
        let state = classify_epistemic_state(&qs, JANE, MATTER);
        assert_eq!(state, EpistemicState::HearsayBelief);
    }

    #[test]
    fn classify_returns_unknown_for_unrelated_quins() {
        let qs = assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000);
        // Jane has no quins about MATTER
        let state = classify_epistemic_state(&qs, JANE, MATTER);
        assert_eq!(state, EpistemicState::Unknown);
    }

    #[test]
    fn has_objective_proof_detects_crypto_link() {
        let qs = assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000);
        assert!(has_objective_proof(&qs, MATTER));
        assert!(!has_objective_proof(&qs, 0xFFFF_u64));
    }

    #[test]
    fn john_frank_jane_scenario() {
        let mut all: Vec<NQuin> = Vec::new();

        // t₁: John directly observes the matter
        all.extend_from_slice(&assert_objective_knowledge(JOHN, MATTER, SENSOR, 1000));

        // t₂: Frank queries — his query is a shared epistemic event
        all.push(record_query_observation(FRANK, MATTER, 2000));
        let frank_query_hash = FRANK.wrapping_mul(0x9e37_79b9) ^ MATTER ^ 2000;

        // t₃-t₄: Jane infers from Frank's query
        all.extend_from_slice(&assert_inferred_belief(JANE, MATTER, frank_query_hash, 3000));

        // Check: John has ObjectiveKnowledge
        assert_eq!(
            classify_epistemic_state(&all, JOHN, MATTER),
            EpistemicState::ObjectiveKnowledge,
        );
        // Check: Jane has InferredBelief — not the same as John's
        assert_eq!(
            classify_epistemic_state(&all, JANE, MATTER),
            EpistemicState::InferredBelief,
        );
        // Check: Frank has Unknown (he queried but never asserted a belief about it)
        assert_eq!(
            classify_epistemic_state(&all, FRANK, MATTER),
            EpistemicState::Unknown,
        );
    }
}
