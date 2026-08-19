//! Sheaves, stalks, simplex, and topological tears (T47–T50).
//!
//! These are the geometric structures for agent context isolation and
//! consistency. They map topological concepts to the VibeScript runtime:
//!
//! - **Stalk (T47):** An isolated snapshot of agent context — a pointer,
//!   not a copied transcript. Contains a PoetSnapshot reference, a
//!   capability lease, and a pulse topic prefix.
//! - **Sheaf condition (T48):** A Pure predicate checked at commit of
//!   staged deltas. Failure is a diagnostic, not an exception unwind.
//! - **Simplex (T49):** A named record of jointly-required cells/graph
//!   shapes. Missing a member ⇒ load or commit reject.
//! - **Topological tear (T50):** A diagnostic + evidential (μ, λ) on a
//!   sealed receipt. Quarantine context is a host routing decision.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.10 T47–T50.

use crate::span::Span;
use crate::value::Value;
use std::collections::BTreeMap;

// ── T47: Stalk ──────────────────────────────────────────────────────────────

/// A stalk — an isolated agent context (T47).
///
/// The stalk is a pointer to an isolated `PoetSnapshot`, not a copied
/// transcript. It carries:
/// - A snapshot ID (opaque handle to the host's snapshot store).
/// - A capability lease ID (what capabilities this stalk is allowed).
/// - A pulse topic prefix (what events this stalk subscribes to).
///
/// The stalk is the agent's "local patch" — the region of the graph
/// it can see and act on. Outside the stalk, the agent is blind.
#[derive(Debug, Clone)]
pub struct Stalk {
    /// Opaque handle to the host's snapshot store.
    pub snapshot_id: u64,
    /// Capability lease ID — what capabilities this stalk is allowed.
    pub capability_lease_id: u64,
    /// Pulse topic prefix — what events this stalk subscribes to.
    pub topic_prefix: String,
    /// The agent DID that owns this stalk.
    pub agent_did: String,
}

impl Stalk {
    pub fn new(snapshot_id: u64, capability_lease_id: u64, topic_prefix: &str, agent_did: &str) -> Self {
        Self {
            snapshot_id,
            capability_lease_id,
            topic_prefix: topic_prefix.to_string(),
            agent_did: agent_did.to_string(),
        }
    }

    /// Convert to a VibeScript Record value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("snapshot_id".into(), Value::U64(self.snapshot_id));
        rec.insert("capability_lease_id".into(), Value::U64(self.capability_lease_id));
        rec.insert("topic_prefix".into(), Value::String(self.topic_prefix.clone()));
        rec.insert("agent_did".into(), Value::String(self.agent_did.clone()));
        Value::Record(rec)
    }

    /// Extract from a VibeScript Record value.
    pub fn from_value(val: &Value) -> Option<Self> {
        let rec = match val {
            Value::Record(r) => r,
            _ => return None,
        };
        let snapshot_id = match rec.get("snapshot_id")? {
            Value::U64(n) => *n,
            _ => return None,
        };
        let capability_lease_id = match rec.get("capability_lease_id")? {
            Value::U64(n) => *n,
            _ => return None,
        };
        let topic_prefix = match rec.get("topic_prefix")? {
            Value::String(s) => s.clone(),
            _ => return None,
        };
        let agent_did = match rec.get("agent_did")? {
            Value::String(s) => s.clone(),
            _ => return None,
        };
        Some(Self {
            snapshot_id,
            capability_lease_id,
            topic_prefix,
            agent_did,
        })
    }
}

// ── T48: Sheaf condition ────────────────────────────────────────────────────

/// A sheaf condition — a Pure predicate checked at commit (T48).
///
/// The sheaf condition verifies that staged deltas are consistent
/// before they are committed. If the condition fails, the result is
/// a diagnostic (not an exception unwind). The host decides whether
/// to retry, quarantine, or reject.
#[derive(Debug, Clone)]
pub struct SheafCondition {
    /// Human-readable name of the condition.
    pub name: String,
    /// The predicate expression (as VibeScript source or a host-side callback).
    /// In v0, this is a name the host resolves; VibeScript doesn't evaluate
    /// the predicate itself — the host does.
    pub predicate_name: String,
    /// Whether this condition is required (true) or advisory (false).
    pub required: bool,
}

impl SheafCondition {
    pub fn new(name: &str, predicate_name: &str, required: bool) -> Self {
        Self {
            name: name.to_string(),
            predicate_name: predicate_name.to_string(),
            required,
        }
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("predicate_name".into(), Value::String(self.predicate_name.clone()));
        rec.insert("required".into(), Value::Bool(self.required));
        Value::Record(rec)
    }
}

/// Result of checking a sheaf condition at commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheafResult {
    /// Condition passed — deltas are consistent.
    Passed,
    /// Condition failed — deltas are inconsistent. The diagnostic
    /// describes what went wrong. The host decides what to do.
    Failed(String),
    /// Condition was advisory and skipped.
    Skipped,
}

// ── T49: Simplex ────────────────────────────────────────────────────────────

/// A simplex — a named record of jointly-required cells/graph shapes (T49).
///
/// A simplex declares that a set of cells (or graph shapes) must all
/// be present together. If any member is missing, the load or commit
/// is rejected. This is the "jointly-required" constraint: you can't
/// have a face without its edges, or an edge without its vertices.
#[derive(Debug, Clone)]
pub struct Simplex {
    /// Human-readable name (e.g. "triangle", "tetrahedron").
    pub name: String,
    /// The IDs of the jointly-required members.
    pub members: Vec<u64>,
    /// The dimension of the simplex (0=vertex, 1=edge, 2=face, 3=cell, ...).
    pub dimension: u8,
}

impl Simplex {
    pub fn new(name: &str, dimension: u8, members: Vec<u64>) -> Self {
        Self {
            name: name.to_string(),
            members,
            dimension,
        }
    }

    /// Check that all members are present in the given set.
    /// Returns the first missing member, or None if all present.
    pub fn check_members(&self, available: &[u64]) -> Option<u64> {
        for &m in &self.members {
            if !available.contains(&m) {
                return Some(m);
            }
        }
        None
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("dimension".into(), Value::I64(self.dimension as i64));
        rec.insert(
            "members".into(),
            Value::List(self.members.iter().map(|&m| Value::U64(m)).collect()),
        );
        Value::Record(rec)
    }
}

// ── T50: Topological tear ───────────────────────────────────────────────────

/// A topological tear — a diagnostic + evidential (μ, λ) on a sealed receipt (T50).
///
/// When a stalk's context is torn (quarantined), the tear is recorded
/// as a sealed receipt with:
/// - The diagnostic (what went wrong).
/// - The evidential μ (provenance weight) and λ (wavelength/penalty).
/// - The stalk ID that was torn.
///
/// Quarantine context is a host routing decision — the tear receipt
/// is the evidence the host uses to decide what to do.
#[derive(Debug, Clone)]
pub struct TopologicalTear {
    /// The diagnostic message.
    pub diagnostic: String,
    /// Evidential μ (provenance weight) — how much provenance was at stake.
    pub mu: f64,
    /// Evidential λ (wavelength/penalty) — the severity of the tear.
    pub lambda: f64,
    /// The stalk ID that was torn.
    pub stalk_id: u64,
    /// The span where the tear was detected.
    pub span: Span,
}

impl TopologicalTear {
    pub fn new(diagnostic: &str, mu: f64, lambda: f64, stalk_id: u64, span: Span) -> Self {
        Self {
            diagnostic: diagnostic.to_string(),
            mu,
            lambda,
            stalk_id,
            span,
        }
    }

    /// Convert to a sealed receipt Value.
    pub fn to_receipt(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("diagnostic".into(), Value::String(self.diagnostic.clone()));
        rec.insert("mu".into(), Value::F64(self.mu));
        rec.insert("lambda".into(), Value::F64(self.lambda));
        rec.insert("stalk_id".into(), Value::U64(self.stalk_id));
        rec.insert("span_start".into(), Value::U64(self.span.start as u64));
        rec.insert("span_end".into(), Value::U64(self.span.end as u64));
        rec.insert("sealed".into(), Value::Bool(true));
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t47_stalk_construction() {
        let stalk = Stalk::new(42, 7, "agent/updates", "did:example:agent");
        assert_eq!(stalk.snapshot_id, 42);
        assert_eq!(stalk.capability_lease_id, 7);
        assert_eq!(stalk.topic_prefix, "agent/updates");
        assert_eq!(stalk.agent_did, "did:example:agent");
    }

    #[test]
    fn t47_stalk_value_round_trip() {
        let stalk = Stalk::new(100, 200, "pulse/topic", "did:example:a");
        let val = stalk.to_value();
        let restored = Stalk::from_value(&val).unwrap();
        assert_eq!(restored.snapshot_id, 100);
        assert_eq!(restored.capability_lease_id, 200);
        assert_eq!(restored.topic_prefix, "pulse/topic");
        assert_eq!(restored.agent_did, "did:example:a");
    }

    #[test]
    fn t47_stalk_from_value_none_on_non_record() {
        assert!(Stalk::from_value(&Value::Null).is_none());
        assert!(Stalk::from_value(&Value::I64(42)).is_none());
    }

    #[test]
    fn t48_sheaf_condition_construction() {
        let cond = SheafCondition::new("glue_check", "pred.glue_consistent", true);
        assert_eq!(cond.name, "glue_check");
        assert_eq!(cond.predicate_name, "pred.glue_consistent");
        assert!(cond.required);
    }

    #[test]
    fn t48_sheaf_condition_value() {
        let cond = SheafCondition::new("glue", "pred.glue", false);
        let val = cond.to_value();
        let rec = match &val {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("name").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "glue"
        );
        assert_eq!(
            match rec.get("required").unwrap() {
                Value::Bool(b) => *b,
                _ => panic!("expected Bool"),
            },
            false
        );
    }

    #[test]
    fn t48_sheaf_result_variants() {
        assert_eq!(SheafResult::Passed, SheafResult::Passed);
        assert_ne!(SheafResult::Passed, SheafResult::Skipped);
        assert_eq!(
            SheafResult::Failed("inconsistent".into()),
            SheafResult::Failed("inconsistent".into())
        );
    }

    #[test]
    fn t49_simplex_construction() {
        let simplex = Simplex::new("triangle", 2, vec![1, 2, 3]);
        assert_eq!(simplex.name, "triangle");
        assert_eq!(simplex.dimension, 2);
        assert_eq!(simplex.members, vec![1, 2, 3]);
    }

    #[test]
    fn t49_simplex_check_members_all_present() {
        let simplex = Simplex::new("edge", 1, vec![10, 20]);
        assert!(simplex.check_members(&[10, 20, 30]).is_none());
    }

    #[test]
    fn t49_simplex_check_members_missing() {
        let simplex = Simplex::new("triangle", 2, vec![1, 2, 3]);
        assert_eq!(simplex.check_members(&[1, 2]), Some(3));
    }

    #[test]
    fn t49_simplex_check_members_empty_available() {
        let simplex = Simplex::new("vertex", 0, vec![42]);
        assert_eq!(simplex.check_members(&[]), Some(42));
    }

    #[test]
    fn t49_simplex_value() {
        let simplex = Simplex::new("tet", 3, vec![1, 2, 3, 4]);
        let val = simplex.to_value();
        let rec = match &val {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("name").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "tet"
        );
        assert_eq!(
            match rec.get("dimension").unwrap() {
                Value::I64(n) => *n,
                _ => panic!("expected I64"),
            },
            3
        );
        let members = match rec.get("members").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(members.len(), 4);
    }

    #[test]
    fn t50_topological_tear_construction() {
        let tear = TopologicalTear::new("context inconsistency", 0.8, 0.3, 42, Span::point(100));
        assert_eq!(tear.diagnostic, "context inconsistency");
        assert_eq!(tear.mu, 0.8);
        assert_eq!(tear.lambda, 0.3);
        assert_eq!(tear.stalk_id, 42);
    }

    #[test]
    fn t50_topological_tear_receipt() {
        let tear = TopologicalTear::new("tear detected", 0.5, 0.9, 7, Span::new(10, 20));
        let receipt = tear.to_receipt();
        let rec = match &receipt {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("diagnostic").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "tear detected"
        );
        assert_eq!(rec.get("mu").unwrap().as_f64(), Some(0.5));
        assert_eq!(rec.get("lambda").unwrap().as_f64(), Some(0.9));
        assert_eq!(
            match rec.get("stalk_id").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            7
        );
        assert_eq!(
            match rec.get("sealed").unwrap() {
                Value::Bool(b) => *b,
                _ => panic!("expected Bool"),
            },
            true
        );
        assert_eq!(
            match rec.get("span_start").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            10
        );
        assert_eq!(
            match rec.get("span_end").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            20
        );
    }
}
