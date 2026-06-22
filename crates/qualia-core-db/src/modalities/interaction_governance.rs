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
}
