//! Deontic compositions (Phase 4, DEONTIC_LOGIC_PLAN §4) — cluster A (zero-heap).
//!
//! Phase 0 proved the modality engines real but **uncomposed**. This module wires the
//! deontic verdict together with the temporal, epistemic, linear and DL/spatial engines —
//! the joins that turn standalone logics into legal reasoning:
//!
//! * **deontic × temporal** — `O(Gφ)` "must hold throughout", `O(φ U ψ)` "must hold until",
//!   via `temporal_ltl::evaluate_ltl_trace`.
//! * **deontic × epistemic** — *mens rea*: classify a violation as knowing vs ignorant
//!   (and ignorance-is-no-excuse when there was a duty to know), via the epistemic encoding.
//! * **deontic × linear** — an obligation discharged by fulfilment *consumes* the duty
//!   (`linear::consume_quin`): a resource spent once, not reusable.
//! * **deontic × spatial** — an obligation in force in a jurisdiction applies in every
//!   sub-jurisdiction `jur:within` it, via `dl::check_subsumption_quin`.
//!
//! All zero-heap (slice in / scalar or slice out). The heavier reasoning joins
//! (argumentation, probabilistic/fuzzy, ASP/abductive) land in cluster B.

use crate::modalities::dl::check_subsumption_quin;
use crate::modalities::epistemic::OP_KNOWS;
use crate::modalities::linear::consume_quin;
use crate::modalities::logic::deontic::{
    extract_deontic_opcode, DeonticStatus, OP_FORBID, OP_OBLIGATE,
};
use crate::modalities::stit::brought_about;
use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
use crate::NQuin;

// ─── deontic × temporal ─────────────────────────────────────────────────────────

/// `O(Gφ)` — the obligation that property `prop` holds **globally** across a state trace.
/// Discharged iff `prop` holds in every state; otherwise Violated. (Continuous protections:
/// "no one shall be subjected to torture" must hold in every state, not just eventually.)
pub fn obligation_globally(prop: u64, trace: &[NQuin]) -> DeonticStatus {
    if evaluate_ltl_trace(trace, &LtlFormula::Globally(prop)) {
        DeonticStatus::Discharged
    } else {
        DeonticStatus::Violated
    }
}

/// `O(φ U ψ)` — the obligation that `ante` holds **until** `consequent` becomes true
/// (provisional measures: "detention standards apply until release"). Discharged iff the
/// until-formula holds over the trace; otherwise Violated.
pub fn obligation_until(ante: u64, consequent: u64, trace: &[NQuin]) -> DeonticStatus {
    if evaluate_ltl_trace(trace, &LtlFormula::Until { ante, consequent }) {
        DeonticStatus::Discharged
    } else {
        DeonticStatus::Violated
    }
}

// ─── deontic × epistemic (mens rea) ───────────────────────────────────────────────

/// The mental state accompanying a deontic violation — the *mens rea* axis legal
/// instruments use to grade culpability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MensRea {
    /// No violation occurred.
    NoViolation,
    /// The agent violated the norm **knowing** it applied — full culpability.
    Knowing,
    /// The agent violated without knowing the norm existed, and had **no duty to know**.
    Ignorant,
    /// Violated in ignorance, but a duty to know was in force — *ignorantia juris non
    /// excusat*: ignorance is no excuse.
    InexcusableIgnorance,
}

/// Did `agent` (per the epistemic frame) KNOW `claim`? An epistemic quin with
/// `predicate[0..7] == OP_KNOWS`, `subject == agent`, `object == claim`.
pub fn agent_knows(epistemic: &[NQuin], agent: u64, claim: u64) -> bool {
    epistemic.iter().any(|q| {
        q.subject == agent && (q.predicate & 0xFF) as u8 == OP_KNOWS && q.object == claim
    })
}

/// Classify the *mens rea* of a possible violation of `norm` by its bearer:
/// `F[α stit φ]` is violated when α brought φ about; `O[α stit φ]` when α did not (omission).
/// A violation is `Knowing` if α knew the forbidden/obligatory content, else `Ignorant` —
/// upgraded to `InexcusableIgnorance` when `had_duty_to_know`.
pub fn classify_mens_rea(
    norm: &NQuin,
    facts: &[NQuin],
    epistemic: &[NQuin],
    had_duty_to_know: bool,
) -> MensRea {
    let agent = norm.subject;
    let content = norm.object;
    let violated = match extract_deontic_opcode(norm.predicate) {
        OP_FORBID => brought_about(facts, agent, content),
        OP_OBLIGATE => !brought_about(facts, agent, content),
        _ => false,
    };
    if !violated {
        return MensRea::NoViolation;
    }
    if agent_knows(epistemic, agent, content) {
        MensRea::Knowing
    } else if had_duty_to_know {
        MensRea::InexcusableIgnorance
    } else {
        MensRea::Ignorant
    }
}

// ─── deontic × linear (discharge consumes the duty) ───────────────────────────────

/// Discharge an obligation by fulfilment: if the bearer brought the obligation's content
/// about, the duty is `Discharged` **and consumed** (`linear::consume_quin` — a duty paid
/// is spent once, not reusable). Returns the status; mutates `norm` to mark consumption on
/// discharge. A non-obligation, or an unfulfilled one, is left unconsumed.
pub fn discharge_obligation(norm: &mut NQuin, facts: &[NQuin]) -> DeonticStatus {
    if extract_deontic_opcode(norm.predicate) != OP_OBLIGATE {
        return DeonticStatus::Active;
    }
    if brought_about(facts, norm.subject, norm.object) {
        consume_quin(norm);
        DeonticStatus::Discharged
    } else {
        DeonticStatus::Active
    }
}

// ─── deontic × spatial (jurisdictional subsumption) ───────────────────────────────

/// Locative obligation subsumption: an obligation in force in `norm_jurisdiction` applies
/// in `target_jurisdiction` iff the target is `jur:within` the norm's jurisdiction
/// (transitively). `within` holds `jur:within` Quins (`subject within object`); the check
/// reuses the DL transitive-closure search. (RCC-8 region *geometry* is not encodable in a
/// 48-byte NQuin — we use the jurisdiction hierarchy, per the plan §1.)
pub fn obligation_applies_in(
    norm_jurisdiction: u64,
    target_jurisdiction: u64,
    within: &[NQuin],
) -> bool {
    // target within norm_jurisdiction  ⟺  subsumption(target, norm_jurisdiction) over `within`.
    check_subsumption_quin(target_jurisdiction, norm_jurisdiction, within)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::linear::is_consumed;
    use crate::modalities::logic::deontic::compile_norm_quin;
    use crate::q_hash;

    fn state(pred: u64) -> NQuin {
        let mut q = NQuin { subject: 0, predicate: pred, object: 0, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }
    fn fact(s: u64, p: u64, o: u64) -> NQuin {
        let mut q = NQuin { subject: s, predicate: p, object: o, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn temporal_globally_obligation() {
        let no_torture = q_hash("q42:freeFromTorture");
        // A trace where the protection holds in every state → Discharged.
        let good = [state(no_torture), state(no_torture), state(no_torture)];
        assert_eq!(obligation_globally(no_torture, &good), DeonticStatus::Discharged);
        // A trace with one breaching state → Violated.
        let bad = [state(no_torture), state(q_hash("q42:torture")), state(no_torture)];
        assert_eq!(obligation_globally(no_torture, &bad), DeonticStatus::Violated);
    }

    #[test]
    fn temporal_until_obligation() {
        let standards = q_hash("q42:detentionStandards");
        let release = q_hash("q42:release");
        // standards hold until release → Discharged.
        let trace = [state(standards), state(standards), state(release)];
        assert_eq!(obligation_until(standards, release, &trace), DeonticStatus::Discharged);
        // standards lapse before release → Violated.
        let bad = [state(standards), state(q_hash("q42:neglect")), state(release)];
        assert_eq!(obligation_until(standards, release, &bad), DeonticStatus::Violated);
    }

    #[test]
    fn mens_rea_knowing_vs_ignorant() {
        let agent = q_hash("did:agent");
        let forbidden = q_hash("q42:launderMoney");
        let norm = compile_norm_quin(agent, OP_FORBID, q_hash("q42:noLaunder"), forbidden, q_hash("frame"), 0, false);
        let did_it = [fact(agent, q_hash("q42:broughtAbout"), forbidden)];

        // No violation if not done.
        assert_eq!(classify_mens_rea(&norm, &[], &[], false), MensRea::NoViolation);
        // Did it, knew it was forbidden → Knowing.
        let knows = [fact(agent, OP_KNOWS as u64, forbidden)];
        assert_eq!(classify_mens_rea(&norm, &did_it, &knows, false), MensRea::Knowing);
        // Did it, didn't know, no duty to know → Ignorant.
        assert_eq!(classify_mens_rea(&norm, &did_it, &[], false), MensRea::Ignorant);
        // Did it, didn't know, BUT had a duty to know → ignorance is no excuse.
        assert_eq!(classify_mens_rea(&norm, &did_it, &[], true), MensRea::InexcusableIgnorance);
    }

    #[test]
    fn discharge_consumes_the_duty() {
        let debtor = q_hash("did:debtor");
        let payment = q_hash("q42:payDebt");
        let mut norm = compile_norm_quin(debtor, OP_OBLIGATE, q_hash("q42:debtDuty"), payment, q_hash("loan"), 0, false);
        assert!(!is_consumed(&norm));
        // Unpaid → Active, not consumed.
        assert_eq!(discharge_obligation(&mut norm, &[]), DeonticStatus::Active);
        assert!(!is_consumed(&norm));
        // Paid → Discharged AND consumed (a duty paid is spent once).
        let paid = [fact(debtor, q_hash("q42:broughtAbout"), payment)];
        assert_eq!(discharge_obligation(&mut norm, &paid), DeonticStatus::Discharged);
        assert!(is_consumed(&norm));
    }

    #[test]
    fn jurisdictional_subsumption() {
        let au = q_hash("jur:Commonwealth-of-Australia");
        let vic = q_hash("jur:Victoria");
        let melbourne = q_hash("jur:Melbourne");
        let nz = q_hash("jur:New-Zealand");
        let within = q_hash("https://ns.webcivics.org/jurisdiction/within");
        let e = |s: u64, o: u64| {
            let mut q = NQuin { subject: s, predicate: within, object: o, context: 0, metadata: 0, parity: 0 };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };
        let graph = [e(vic, au), e(melbourne, vic)]; // Melbourne within VIC within AU
        // An ICCPR obligation in force for AU applies in VIC and (transitively) Melbourne.
        assert!(obligation_applies_in(au, vic, &graph));
        assert!(obligation_applies_in(au, melbourne, &graph));
        // It does not reach a different State.
        assert!(!obligation_applies_in(au, nz, &graph));
    }
}
