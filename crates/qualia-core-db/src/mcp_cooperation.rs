//! MCP agent cooperation (Track M — task #17/#16/#18).
//!
//! The cooperation gate that has long been missing: every MCP call should carry a
//! **verified, typed calling-agent identity + standpoint** (who is asking, in what role),
//! and the request should be evaluated against the rights ontology *before* execution.
//!
//! This is load-bearing for the whole thesis — agents verify each other's conduct; **trust
//! is behaviourally derived, not self-asserted** ([[feedback-trust-is-behaviourally-derived]]);
//! and there is no platform-provider deciding for everyone. The gate composes three existing
//! pieces rather than inventing a fourth:
//!   1. **Verified, not asserted** — the caller's identity must be cryptographically verified
//!      (a signed VC via [`crate::verifiable_credential`]), not merely claimed.
//!   2. **Grounded** — an artificial agent with no human Principal is refused
//!      ([`crate::agent::is_ungrounded_agency`], agency.n3 G1').
//!   3. **Governed** — the request is run through the deontic policy gate
//!      ([`crate::modalities::interaction_governance::map_policy`], Phase 6).
//!
//! Mandatory per-call enforcement in the dispatch is a deliberate MCP-contract change (it
//! fails closed on unverified callers) and is gated on Timothy's sign-off — see
//! DEONTIC_LOGIC_PLAN Track M. This module is the mechanism + an opt-in tool; it does not
//! silently change every existing caller's behaviour.

use crate::indexing::QuinIndex;
use crate::modalities::interaction_governance::{map_policy, permits_execution, Governance, PolicyMode};
use crate::modalities::logic::deontic::DeonticStatus;

/// Who is calling, in what typed role, and whether their identity was *verified* (vs merely
/// asserted). `agent` and `role` are identifier hashes (one identity space, #14).
#[derive(Debug, Clone, Copy)]
pub struct CallerStandpoint {
    /// The calling agent's identifier.
    pub agent: u64,
    /// Its typed role / standpoint for this call (a values class or capability).
    pub role: u64,
    /// True iff the identity was cryptographically verified (a signed VC), not just claimed.
    pub verified: bool,
}

/// The outcome of the cooperation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooperationVerdict {
    /// The call may proceed, under this runtime policy mode.
    Authorized(PolicyMode),
    /// Refused: the caller's identity was asserted, not verified.
    DeniedUnverified,
    /// Refused: the caller is an artificial agent with no human Principal (agency.n3 G1').
    DeniedUngrounded,
    /// Refused by the deontic gate (e.g. a non-derogable violation → PreventiveBlock, or an
    /// ambiguous mapping → Interactive). Carries the mode so the caller knows why.
    DeniedByPolicy(PolicyMode),
}

/// Is a caller grounded — i.e. NOT an ungrounded artificial agent? (A human, a legal person,
/// or an AI with a declared `values:operatedBy` Principal all pass.)
#[inline]
pub fn caller_grounded(index: &QuinIndex, agent: u64) -> bool {
    !crate::agent::is_ungrounded_agency(index, agent)
}

/// The cooperation gate, with grounding supplied explicitly (index-free; used by the tool and
/// by callers that already know the caller's grounding). Order: verified → grounded → governed.
pub fn authorize(
    standpoint: &CallerStandpoint,
    grounded: bool,
    request_status: DeonticStatus,
    governance: Governance,
) -> CooperationVerdict {
    if !standpoint.verified {
        return CooperationVerdict::DeniedUnverified;
    }
    if !grounded {
        return CooperationVerdict::DeniedUngrounded;
    }
    let mode = map_policy(request_status, governance);
    if permits_execution(mode) {
        CooperationVerdict::Authorized(mode)
    } else {
        CooperationVerdict::DeniedByPolicy(mode)
    }
}

/// The cooperation gate over the live graph: resolves the caller's grounding from `index`
/// (agency.n3 G1'), then applies [`authorize`].
pub fn authorize_call(
    index: &QuinIndex,
    standpoint: &CallerStandpoint,
    request_status: DeonticStatus,
    governance: Governance,
) -> CooperationVerdict {
    authorize(standpoint, caller_grounded(index, standpoint.agent), request_status, governance)
}

/// Stable label for logs / MCP responses.
pub const fn cooperation_label(v: CooperationVerdict) -> &'static str {
    match v {
        CooperationVerdict::Authorized(_) => "Authorized",
        CooperationVerdict::DeniedUnverified => "DeniedUnverified",
        CooperationVerdict::DeniedUngrounded => "DeniedUngrounded",
        CooperationVerdict::DeniedByPolicy(_) => "DeniedByPolicy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{A_ARTIFICIAL_AGENT, A_NATURAL_PERSON, P_OPERATED_BY, P_RDF_TYPE};
    use crate::q_hash;
    use crate::NQuin;

    fn sp(agent: u64, verified: bool) -> CallerStandpoint {
        CallerStandpoint { agent, role: q_hash("role:requester"), verified }
    }
    fn t(s: u64, p: u64, o: u64) -> NQuin {
        let mut q = NQuin { subject: s, predicate: p, object: o, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn unverified_caller_is_denied() {
        let v = authorize(&sp(q_hash("did:x"), false), true, DeonticStatus::Active, Governance::default());
        assert_eq!(v, CooperationVerdict::DeniedUnverified);
    }

    #[test]
    fn ungrounded_caller_is_denied() {
        let v = authorize(&sp(q_hash("did:bot"), true), false, DeonticStatus::Active, Governance::default());
        assert_eq!(v, CooperationVerdict::DeniedUngrounded);
    }

    #[test]
    fn verified_grounded_ordinary_call_is_authorized() {
        let v = authorize(&sp(q_hash("did:alice"), true), true, DeonticStatus::Active, Governance::default());
        assert_eq!(v, CooperationVerdict::Authorized(PolicyMode::Allow));
    }

    #[test]
    fn non_derogable_violation_request_is_blocked_by_policy() {
        let g = Governance { non_derogable: true, humanitarian: false, ambiguous: false };
        let v = authorize(&sp(q_hash("did:alice"), true), true, DeonticStatus::Violated, g);
        assert_eq!(v, CooperationVerdict::DeniedByPolicy(PolicyMode::PreventiveBlock));
    }

    #[test]
    fn authorize_call_resolves_grounding_from_the_graph() {
        // An artificial agent with NO operatedBy Principal → ungrounded → denied.
        let bot = q_hash("did:bot");
        let idx = QuinIndex::from_slice(&[t(bot, P_RDF_TYPE, A_ARTIFICIAL_AGENT)]);
        assert_eq!(
            authorize_call(&idx, &sp(bot, true), DeonticStatus::Active, Governance::default()),
            CooperationVerdict::DeniedUngrounded
        );
        // The same agent WITH a human Principal → grounded → authorized.
        let human = q_hash("did:alice");
        let idx2 = QuinIndex::from_slice(&[
            t(bot, P_RDF_TYPE, A_ARTIFICIAL_AGENT),
            t(bot, P_OPERATED_BY, human),
        ]);
        assert_eq!(
            authorize_call(&idx2, &sp(bot, true), DeonticStatus::Active, Governance::default()),
            CooperationVerdict::Authorized(PolicyMode::Allow)
        );
        // A natural person is never ungrounded.
        let alice = q_hash("did:alice");
        let idx3 = QuinIndex::from_slice(&[t(alice, P_RDF_TYPE, A_NATURAL_PERSON)]);
        assert!(matches!(
            authorize_call(&idx3, &sp(alice, true), DeonticStatus::Active, Governance::default()),
            CooperationVerdict::Authorized(_)
        ));
    }
}
