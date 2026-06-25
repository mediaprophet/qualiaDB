//! Interaction governance (Phase 6, DEONTIC_LOGIC_PLAN §15) — the final stage that maps an
//! abstract [`DeonticVerdict`] to a concrete runtime action in the Webizen VM.
//!
//! Once the deontic / spatial / argumentation logics yield a verdict, *something must
//! happen*. This module is the **pure decision layer**: verdict (+ a little classification)
//! → [`PolicyMode`]. The side effects each mode implies are performed by the caller:
//!   * [`PolicyMode::PreventiveBlock`] → the VM injects a `DenyRollback` and halts the
//!     transaction *before* harm (non-derogable violations: child safety, the ICCPR core).
//!   * [`PolicyMode::PermissiveAudit`] → the transaction proceeds but a `BreachRecord` is
//!     written to the WAL ([`super::meta_deontic::record_breach_to_wal`]) for the evidentiary
//!     trail (system utility preserved, conduct still recorded).
//!   * [`PolicyMode::Prioritize`] → QoS / routing preference for `hict:HumanitarianICT`
//!     (peace infrastructure, medical access).
//!   * [`PolicyMode::Interactive`] → halt and ask the human for a `sense:HumanCorrection`
//!     (ambiguous or uninterpretable mappings — agency over meaning stays human).
//!   * [`PolicyMode::Allow`] → nothing special (in force / no longer binding).
//!
//! Keeping the decision pure and separate from the effect is what makes the gate auditable
//! and the same logic reusable by both the VM and the MCP cooperation interface (Track M).

use crate::modalities::logic::deontic::{DeonticStatus, DeonticVerdict};

/// What the runtime should DO about a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    /// Halt before harm — `DenyRollback`. For non-derogable violations.
    PreventiveBlock,
    /// Allow, but log an immutable `BreachRecord` to the WAL. For non-critical violations.
    PermissiveAudit,
    /// Grant QoS / routing / UI priority. For in-force humanitarian norms.
    Prioritize,
    /// Halt and request a human correction. For ambiguous / uninterpretable mappings.
    Interactive,
    /// No special action — proceed.
    Allow,
}

/// The classification a verdict needs beyond its status to be governed: is the norm
/// non-derogable (a Hohfeldian Immunity / MandatoryBaseline), humanitarian, and is its
/// mapping ambiguous? These come from the graph (the `values:nonDerogable` overlay,
/// `hict:HumanitarianICT`, and the resolver), not invented here.
#[derive(Debug, Clone, Copy, Default)]
pub struct Governance {
    pub non_derogable: bool,
    pub humanitarian: bool,
    pub ambiguous: bool,
}

/// Map a deontic status + classification to the runtime [`PolicyMode`].
///
/// Order of precedence: ambiguity (ask the human) → violation (block if non-derogable, else
/// audit) → in-force humanitarian (prioritize) → otherwise allow.
pub fn map_policy(status: DeonticStatus, g: Governance) -> PolicyMode {
    if g.ambiguous {
        return PolicyMode::Interactive;
    }
    match status {
        DeonticStatus::Violated => {
            if g.non_derogable {
                PolicyMode::PreventiveBlock
            } else {
                PolicyMode::PermissiveAudit
            }
        }
        DeonticStatus::Active | DeonticStatus::Discharged => {
            if g.humanitarian {
                PolicyMode::Prioritize
            } else {
                PolicyMode::Allow
            }
        }
        // Not yet binding, no longer binding, or uninterpretable.
        DeonticStatus::Pending | DeonticStatus::Defeated | DeonticStatus::Expired => {
            PolicyMode::Allow
        }
        DeonticStatus::Malformed => PolicyMode::Interactive, // cannot interpret → ask a human
    }
}

/// Govern a full verdict (convenience over [`map_policy`]).
#[inline]
pub fn govern_verdict(verdict: &DeonticVerdict, g: Governance) -> PolicyMode {
    map_policy(verdict.status, g)
}

/// Whether this mode lets the transaction proceed (audit/prioritize/allow) vs halts it
/// (block/interactive). The VM uses this as the go/no-go bit.
#[inline]
pub fn permits_execution(mode: PolicyMode) -> bool {
    matches!(
        mode,
        PolicyMode::PermissiveAudit | PolicyMode::Prioritize | PolicyMode::Allow
    )
}

/// A short, stable label for logs / MCP responses.
pub const fn policy_action(mode: PolicyMode) -> &'static str {
    match mode {
        PolicyMode::PreventiveBlock => "DenyRollback",
        PolicyMode::PermissiveAudit => "AllowAndAuditToWAL",
        PolicyMode::Prioritize => "GrantPriority",
        PolicyMode::Interactive => "RequestHumanCorrection",
        PolicyMode::Allow => "Allow",
    }
}

// ─── Dynamic overriding rules (humanitarian emergency) ────────────────────────────

/// A humanitarian emergency may downgrade a [`PolicyMode::PreventiveBlock`] to
/// [`PolicyMode::PermissiveAudit`] (proceed but record) — EXCEPT for the non-overridable
/// **hard core** (torture, child safety, the non-derogable absolute prohibitions), which never
/// bypasses. All other modes pass through unchanged. This is the structured emergency exception,
/// not an open backdoor.
pub fn apply_emergency_override(base: PolicyMode, emergency: bool, hard_core: bool) -> PolicyMode {
    if emergency && base == PolicyMode::PreventiveBlock && !hard_core {
        PolicyMode::PermissiveAudit
    } else {
        base
    }
}

// ─── Multi-stakeholder M-of-N threshold ───────────────────────────────────────────

/// A multi-stakeholder governance decision is authorized iff at least `m` stakeholders approved
/// (an M-of-N threshold; `approvals` is the count of approving stakeholders). `m == 0` is never
/// authorized (a decision needs at least one approver).
pub fn threshold_authorized(approvals: usize, m: usize) -> bool {
    m > 0 && approvals >= m
}

// ─── Systemic circuit breaker (paraconsistent inconsistency spike) ────────────────

/// Trip the systemic circuit breaker when inconsistency `saturation` (from
/// `paraconsistent::local_saturation` / `global_saturation`) reaches `threshold`: the system
/// halts into [`PolicyMode::Interactive`] (ask a human) rather than act on a saturated,
/// self-contradictory graph. Returns the override mode if tripped, else `None`.
pub fn circuit_breaker(saturation: f32, threshold: f32) -> Option<PolicyMode> {
    if saturation >= threshold {
        Some(PolicyMode::Interactive)
    } else {
        None
    }
}

// ─── Proportionality binding (human-rights instruments) ───────────────────────────

/// A governance action that **restricts individual agency** is justified only if proportionate —
/// its `marginal_harm` to the person is strictly less than the `advantage` it secures. Binds
/// algorithmic governance to the proportionality test of the human-rights instruments. A
/// non-restricting action is always permitted.
pub fn restriction_proportionate(restricts_agency: bool, marginal_harm: f64, advantage: f64) -> bool {
    !restricts_agency || marginal_harm < advantage
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(non_derogable: bool, humanitarian: bool, ambiguous: bool) -> Governance {
        Governance { non_derogable, humanitarian, ambiguous }
    }

    #[test]
    fn non_derogable_violation_is_preventive_block() {
        let m = map_policy(DeonticStatus::Violated, g(true, false, false));
        assert_eq!(m, PolicyMode::PreventiveBlock);
        assert!(!permits_execution(m), "a non-derogable breach must NOT proceed");
        assert_eq!(policy_action(m), "DenyRollback");
    }

    #[test]
    fn ordinary_violation_is_permissive_audit() {
        let m = map_policy(DeonticStatus::Violated, g(false, false, false));
        assert_eq!(m, PolicyMode::PermissiveAudit);
        assert!(permits_execution(m), "a non-critical breach proceeds but is recorded");
    }

    #[test]
    fn humanitarian_in_force_is_prioritized() {
        assert_eq!(map_policy(DeonticStatus::Active, g(false, true, false)), PolicyMode::Prioritize);
        // Non-humanitarian in force → just allow.
        assert_eq!(map_policy(DeonticStatus::Active, g(false, false, false)), PolicyMode::Allow);
    }

    #[test]
    fn ambiguity_always_defers_to_a_human() {
        // Ambiguity wins even over a non-derogable violation — the human decides the mapping.
        assert_eq!(map_policy(DeonticStatus::Violated, g(true, false, true)), PolicyMode::Interactive);
        assert_eq!(map_policy(DeonticStatus::Active, g(false, true, true)), PolicyMode::Interactive);
        // Malformed verdicts also route to a human.
        assert_eq!(map_policy(DeonticStatus::Malformed, g(false, false, false)), PolicyMode::Interactive);
    }

    #[test]
    fn non_binding_statuses_allow() {
        for s in [DeonticStatus::Pending, DeonticStatus::Defeated, DeonticStatus::Expired] {
            assert_eq!(map_policy(s, g(false, false, false)), PolicyMode::Allow);
        }
        // Discharged duty, humanitarian context → still prioritized.
        assert_eq!(map_policy(DeonticStatus::Discharged, g(false, true, false)), PolicyMode::Prioritize);
    }

    #[test]
    fn humanitarian_emergency_overrides_non_core_blocks_only() {
        // An ordinary non-derogable block downgrades to audit under emergency…
        assert_eq!(
            apply_emergency_override(PolicyMode::PreventiveBlock, true, false),
            PolicyMode::PermissiveAudit
        );
        // …but the hard core (torture / child safety) NEVER bypasses.
        assert_eq!(
            apply_emergency_override(PolicyMode::PreventiveBlock, true, true),
            PolicyMode::PreventiveBlock
        );
        // No emergency → unchanged; non-block modes pass through.
        assert_eq!(apply_emergency_override(PolicyMode::PreventiveBlock, false, false), PolicyMode::PreventiveBlock);
        assert_eq!(apply_emergency_override(PolicyMode::Allow, true, false), PolicyMode::Allow);
    }

    #[test]
    fn m_of_n_threshold_governance() {
        assert!(threshold_authorized(3, 3));
        assert!(threshold_authorized(4, 3));
        assert!(!threshold_authorized(2, 3));
        assert!(!threshold_authorized(0, 0), "a decision needs at least one approver");
    }

    #[test]
    fn circuit_breaker_trips_on_inconsistency_spike() {
        assert_eq!(circuit_breaker(0.9, 0.8), Some(PolicyMode::Interactive));
        assert_eq!(circuit_breaker(0.5, 0.8), None);
    }

    #[test]
    fn agency_restriction_must_be_proportionate() {
        // A restriction whose harm < advantage is justified; harm ≥ advantage is not.
        assert!(restriction_proportionate(true, 1.0, 5.0));
        assert!(!restriction_proportionate(true, 5.0, 1.0));
        // A non-restricting action is always permitted.
        assert!(restriction_proportionate(false, 100.0, 0.0));
    }
}
