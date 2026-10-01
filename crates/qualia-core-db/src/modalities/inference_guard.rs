//! Graph-backed inference guard.
//!
//! This is the small, deterministic boundary between a generative backend and
//! the semantic graph. It deliberately returns an action rather than emitting
//! text: the caller owns rollback, UI copy, and any corrective prompt.

use crate::modalities::epistemic::agent_knows;
use crate::modalities::interaction_governance::{map_policy, Governance, PolicyMode};
use crate::modalities::logic::deontic::{
    harvest_defeater_fingerprints, norm_lifecycle_status, DeonticStatus, MAX_DEFEATER_SLOTS,
};
use crate::modalities::paraconsistent::route_paraconsistent;
use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
use crate::NQuin;

/// Maximum graph facts that the browser guard admits in one pass. This keeps
/// its two caller-owned scratch buffers below 25 KiB.
pub const MAX_GUARD_GRAPH_QUINS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceGuardAction {
    Allow,
    /// Do not use the unqualified result; the caller should retry with the
    /// stated graph constraint in its prompt.
    Steer,
    /// Stop generation or roll back the current token stream.
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceGuardReason {
    None,
    DeonticViolation,
    TemporalViolation,
    MissingKnowledge,
    ContradictoryGraph,
    GraphCapacityExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InferenceGuardVerdict {
    pub action: InferenceGuardAction,
    pub reason: InferenceGuardReason,
    pub isolated_count: usize,
}

impl InferenceGuardVerdict {
    const fn allow() -> Self {
        Self {
            action: InferenceGuardAction::Allow,
            reason: InferenceGuardReason::None,
            isolated_count: 0,
        }
    }
}

/// Evaluate graph logic before (or between tokens of) inference.
///
/// `non_derogable` is graph policy classification for the supplied norms. A
/// violated non-derogable norm maps to `DenyRollback`; an ordinary violation
/// maps to `Steer`, preserving the existing audit-first policy semantics.
/// `required_claim == 0` and `temporal_invariant == 0` disable their respective
/// checks, which lets callers compose only the logics relevant to a request.
pub fn evaluate_inference_guard(
    norms: &[NQuin],
    facts: &[NQuin],
    epistemic: &[NQuin],
    trace: &[NQuin],
    actor: u64,
    required_claim: u64,
    temporal_invariant: u64,
    now_unix: u32,
    non_derogable: bool,
) -> InferenceGuardVerdict {
    if facts.len() > MAX_GUARD_GRAPH_QUINS {
        return InferenceGuardVerdict {
            action: InferenceGuardAction::Steer,
            reason: InferenceGuardReason::GraphCapacityExceeded,
            isolated_count: 0,
        };
    }

    // Temporal safety is a hard guard: a continuous invariant that has already
    // failed cannot be repaired by an LLM completion.
    if temporal_invariant != 0
        && !evaluate_ltl_trace(trace, &LtlFormula::Globally(temporal_invariant))
    {
        return InferenceGuardVerdict {
            action: InferenceGuardAction::Deny,
            reason: InferenceGuardReason::TemporalViolation,
            isolated_count: 0,
        };
    }

    let mut defeaters = [0u64; MAX_DEFEATER_SLOTS];
    let defeater_count = harvest_defeater_fingerprints(norms, &mut defeaters);
    for norm in norms {
        if norm_lifecycle_status(norm, now_unix, 0, &defeaters[..defeater_count], facts)
            == DeonticStatus::Violated
        {
            let policy = map_policy(
                DeonticStatus::Violated,
                Governance {
                    non_derogable,
                    humanitarian: false,
                    ambiguous: false,
                },
            );
            return InferenceGuardVerdict {
                action: if policy == PolicyMode::PreventiveBlock {
                    InferenceGuardAction::Deny
                } else {
                    InferenceGuardAction::Steer
                },
                reason: InferenceGuardReason::DeonticViolation,
                isolated_count: 0,
            };
        }
    }

    if required_claim != 0 && !agent_knows(epistemic, actor, required_claim) {
        return InferenceGuardVerdict {
            action: InferenceGuardAction::Steer,
            reason: InferenceGuardReason::MissingKnowledge,
            isolated_count: 0,
        };
    }

    let mut consistent = [NQuin::default(); MAX_GUARD_GRAPH_QUINS];
    let mut isolated = [NQuin::default(); MAX_GUARD_GRAPH_QUINS];
    match route_paraconsistent(facts, &mut consistent, &mut isolated) {
        Ok((_, isolated_count)) if isolated_count != 0 => InferenceGuardVerdict {
            action: InferenceGuardAction::Steer,
            reason: InferenceGuardReason::ContradictoryGraph,
            isolated_count,
        },
        Ok(_) => InferenceGuardVerdict::allow(),
        // Capacity was checked above, so an error is a conservative deferral.
        Err(_) => InferenceGuardVerdict {
            action: InferenceGuardAction::Steer,
            reason: InferenceGuardReason::GraphCapacityExceeded,
            isolated_count: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::epistemic::{CERTAINTY_BIT_SHIFT, OP_KNOWS};
    use crate::modalities::logic::deontic::OP_FORBID;
    use crate::q_hash;

    fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity: subject ^ predicate ^ object ^ context,
        }
    }

    #[test]
    fn non_derogable_prohibition_denies_inference() {
        let actor = q_hash("did:demo:agent");
        let action = q_hash("q42:publishSensitiveRecord");
        let norm = quin(actor, OP_FORBID as u64, action, q_hash("q42:policy"));
        let fact = quin(actor, q_hash("q42:performed"), action, q_hash("q42:policy"));
        let verdict = evaluate_inference_guard(&[norm], &[fact], &[], &[], actor, 0, 0, 1, true);
        assert_eq!(verdict.action, InferenceGuardAction::Deny);
        assert_eq!(verdict.reason, InferenceGuardReason::DeonticViolation);
    }

    #[test]
    fn missing_knowledge_steers_without_blocking() {
        let actor = q_hash("did:demo:agent");
        let claim = q_hash("q42:verifiedSource");
        let verdict = evaluate_inference_guard(&[], &[], &[], &[], actor, claim, 0, 1, false);
        assert_eq!(verdict.action, InferenceGuardAction::Steer);
        assert_eq!(verdict.reason, InferenceGuardReason::MissingKnowledge);

        let knowledge = quin(
            actor,
            OP_KNOWS as u64 | ((255u64) << CERTAINTY_BIT_SHIFT),
            claim,
            q_hash("q42:world"),
        );
        assert_eq!(
            evaluate_inference_guard(&[], &[], &[knowledge], &[], actor, claim, 0, 1, false).action,
            InferenceGuardAction::Allow
        );
    }

    #[test]
    fn failed_temporal_invariant_denies() {
        let safe = q_hash("q42:safetyChecked");
        let trace = [quin(1, safe, 0, 1), quin(1, q_hash("q42:unsafe"), 0, 1)];
        let verdict = evaluate_inference_guard(&[], &[], &[], &trace, 0, 0, safe, 1, false);
        assert_eq!(verdict.reason, InferenceGuardReason::TemporalViolation);
        assert_eq!(verdict.action, InferenceGuardAction::Deny);
    }

    #[test]
    fn contradiction_isolated_then_steered() {
        let predicate = q_hash("q42:recordStatus");
        let facts = [quin(1, predicate, 10, 9), quin(1, predicate, 11, 9)];
        let verdict = evaluate_inference_guard(&[], &facts, &[], &[], 0, 0, 0, 1, false);
        assert_eq!(verdict.action, InferenceGuardAction::Steer);
        assert_eq!(verdict.reason, InferenceGuardReason::ContradictoryGraph);
        assert_eq!(verdict.isolated_count, 1);
    }
}
