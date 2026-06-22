//! STIT agency — "α Sees To It That φ" (Phase 3, DEONTIC_LOGIC_PLAN §5).
//!
//! Standard deontic logic makes *states of affairs* obligatory (`O(φ)`). Legal instruments
//! make *specific agents* obligated to **act** ("the State shall ensure…"). STIT binds the
//! deontic force to the agent who is the causal force, which lets the engine:
//!   * distinguish a **duty-bearer** from a **bystander** ([`is_duty_bearer`]);
//!   * detect **omission** — an in-force obligation the bearer did not bring about
//!     ([`agentive_status`] → `Violated`); and
//!   * model **joint action / shared liability** — `O[{α,β} stit φ]` discharged iff *any*
//!     member brings φ about, else *all* members share liability
//!     ([`joint_discharged`], [`joint_liable_members`]).
//!
//! A STIT-bound norm reuses the deontic norm Quin: `subject` is the agent α (the causal
//! force), `object` is the brought-about content φ. The causal fact convention is
//! `(α, q42:broughtAbout, φ)`. This module is a *post-hoc accountability* reading over a
//! deontic norm (evaluated when the duty is due/closed), complementing the live-status
//! `deontic::norm_lifecycle_status`. Zero-heap throughout.

use crate::modalities::logic::deontic::{
    extract_deontic_opcode, DeonticStatus, OP_FORBID, OP_OBLIGATE,
};
use crate::{q_hash, NQuin};

/// Did `agent` see to it that `content` — i.e. is the causal fact `(agent, q42:broughtAbout,
/// content)` present?
pub fn brought_about(facts: &[NQuin], agent: u64, content: u64) -> bool {
    let p = q_hash("q42:broughtAbout");
    facts
        .iter()
        .any(|q| q.subject == agent && q.predicate == p && q.object == content)
}

/// True iff `agent` is the bearer (causal subject) of the agentive norm — a duty-bearer
/// rather than a bystander. (Accountability: the obligation attaches to the actor, not to
/// everyone who could have acted.)
#[inline]
pub fn is_duty_bearer(norm: &NQuin, agent: u64) -> bool {
    norm.subject == agent
}

/// Post-hoc accountability status of an agentive norm `O[α stit φ]` / `F[α stit φ]`,
/// evaluated when the duty is due/closed:
/// * `OP_OBLIGATE`: the bearer brought φ about → [`Discharged`](DeonticStatus::Discharged);
///   otherwise it is an **omission** → [`Violated`](DeonticStatus::Violated).
/// * `OP_FORBID`: the bearer brought the forbidden φ about → `Violated`; else `Active`.
/// * anything else (e.g. a permission): `Active` — a liberty cannot be omitted.
pub fn agentive_status(norm: &NQuin, facts: &[NQuin]) -> DeonticStatus {
    let agent = norm.subject;
    let content = norm.object;
    match extract_deontic_opcode(norm.predicate) {
        OP_OBLIGATE => {
            if brought_about(facts, agent, content) {
                DeonticStatus::Discharged
            } else {
                DeonticStatus::Violated // omission: O[α stit φ] ∧ ¬[α stit φ]
            }
        }
        OP_FORBID => {
            if brought_about(facts, agent, content) {
                DeonticStatus::Violated
            } else {
                DeonticStatus::Active
            }
        }
        _ => DeonticStatus::Active,
    }
}

/// Joint obligation `O[{members} stit φ]`: discharged iff **any** member saw to it that φ
/// (joint sufficiency). Zero-heap.
pub fn joint_discharged(members: &[u64], content: u64, facts: &[NQuin]) -> bool {
    members.iter().any(|&m| brought_about(facts, m, content))
}

/// Shared liability: if the joint obligation is NOT discharged, **every** member shares
/// liability — write them into `out` and return the count. Returns `0` when discharged
/// (no one is liable). Zero-heap (caller-supplied `out`).
pub fn joint_liable_members(
    members: &[u64],
    content: u64,
    facts: &[NQuin],
    out: &mut [u64],
) -> usize {
    if joint_discharged(members, content, facts) {
        return 0;
    }
    let mut n = 0usize;
    for &m in members {
        if n >= out.len() {
            break;
        }
        out[n] = m;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::compile_norm_quin;

    fn fact(s: u64, p: u64, o: u64) -> NQuin {
        let mut q = NQuin { subject: s, predicate: p, object: o, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn duty_bearer_vs_bystander() {
        let (state, citizen) = (q_hash("did:state"), q_hash("did:citizen"));
        let ensure = q_hash("q42:ensureRemedy");
        let norm = compile_norm_quin(state, OP_OBLIGATE, ensure, q_hash("q42:victim"), q_hash("frame"), 0, false);
        assert!(is_duty_bearer(&norm, state), "the State bears the duty");
        assert!(!is_duty_bearer(&norm, citizen), "a citizen is a bystander to this duty");
    }

    #[test]
    fn obligation_brought_about_is_discharged_else_omission() {
        let state = q_hash("did:state");
        let outcome = q_hash("q42:provideRemedy");
        let norm = compile_norm_quin(state, OP_OBLIGATE, q_hash("q42:remedyDuty"), outcome, q_hash("frame"), 0, false);
        // Brought about → Discharged.
        let done = [fact(state, q_hash("q42:broughtAbout"), outcome)];
        assert_eq!(agentive_status(&norm, &done), DeonticStatus::Discharged);
        // Not brought about → omission → Violated.
        assert_eq!(agentive_status(&norm, &[]), DeonticStatus::Violated);
    }

    #[test]
    fn forbidden_act_brought_about_is_violation() {
        let platform = q_hash("did:platformAgent");
        let manipulate = q_hash("q42:manipulateUser");
        let norm = compile_norm_quin(platform, OP_FORBID, q_hash("q42:noManip"), manipulate, q_hash("frame"), 0, false);
        // Performed the forbidden act → Violated.
        let did = [fact(platform, q_hash("q42:broughtAbout"), manipulate)];
        assert_eq!(agentive_status(&norm, &did), DeonticStatus::Violated);
        // Did not → Active.
        assert_eq!(agentive_status(&norm, &[]), DeonticStatus::Active);
    }

    #[test]
    fn joint_action_shared_liability() {
        let principal = q_hash("did:principal");
        let platform = q_hash("did:platformAgent");
        let members = [principal, platform];
        let content = q_hash("q42:protectUserData");

        // Neither brought it about → joint obligation undischarged, BOTH share liability.
        let mut out = [0u64; 4];
        assert!(!joint_discharged(&members, content, &[]));
        let n = joint_liable_members(&members, content, &[], &mut out);
        assert_eq!(n, 2, "both members share liability");
        assert!(out[..n].contains(&principal) && out[..n].contains(&platform));

        // One member brings it about → discharged, no one liable (joint sufficiency).
        let done = [fact(platform, q_hash("q42:broughtAbout"), content)];
        assert!(joint_discharged(&members, content, &done));
        assert_eq!(joint_liable_members(&members, content, &done, &mut out), 0);
    }
}
