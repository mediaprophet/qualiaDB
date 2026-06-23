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

// ════════════════════════════════════════════════════════════════════════════════
// Cluster B — reasoning-engine joins (all zero-heap; bounded fixed-array backends)
// ════════════════════════════════════════════════════════════════════════════════

// ─── deontic × argumentation (conflict → grounded extension → verdict) ────────────

/// Resolve a normative conflict by Dung's grounded semantics: given the competing norm IDs
/// and the `attacks` pairs `(attacker, target)`, does `goal` SURVIVE (belong to the grounded
/// extension)? The survivor is the objectively defensible verdict after all attacks and
/// defences resolve — e.g. a general duty reinstated when an emergency override defeats its
/// exception. Composes `argumentation::grounded_contains` (bounded, zero-heap).
pub fn norm_survives_conflict(norm_ids: &[u64], attacks: &[(u64, u64)], goal: u64) -> bool {
    crate::modalities::argumentation::grounded_contains(norm_ids, attacks, goal)
}

// ─── deontic × fuzzy / probabilistic (partial fulfilment, trust) ──────────────────

/// Degree to which a *progressively-realised* obligation is fulfilled: the Gödel t-norm
/// (min) of its sub-requirements' truth degrees — the weakest link gates the whole (the
/// ICESCR "progressive realization" reading). Each requirement carries its degree in
/// `metadata`. Composes `fuzzy::conjunction`.
pub fn fulfilment_degree(requirements: &[NQuin]) -> f32 {
    crate::modalities::fuzzy::conjunction(requirements)
}

/// Is a progressively-realised obligation met to at least `threshold` ∈ [0,1]?
pub fn obligation_fuzzily_met(requirements: &[NQuin], threshold: f32) -> bool {
    fulfilment_degree(requirements) >= threshold
}

/// Behavioural-trust gate: a permission/capability activates only when the holder's derived
/// trust `weight` exceeds `threshold` τ. Composes `probabilistic::evaluate_threshold`.
pub fn trust_gate(weight: f32, threshold: f32) -> bool {
    crate::modalities::probabilistic::evaluate_threshold(weight, threshold)
}

// ─── deontic × ASP / abductive (multi-remedy scenarios, breach diagnosis) ─────────

/// Enumerate the valid compliance scenarios when an instrument under-determines the remedy
/// ("the State shall provide remedy X, Y, or Z"): the stable models (answer sets) of the
/// remedy `rules` over `atoms`, written to `out` as bitmasks over atom indices. Composes
/// `asp::compute_answer_sets`.
pub fn remedy_scenarios(
    atoms: &[u64],
    rules: &[crate::modalities::asp::AspRule],
    out: &mut [u64],
) -> usize {
    crate::modalities::asp::compute_answer_sets(atoms, rules, out)
}

/// Abductive breach diagnosis: walk backward from an observed `violation` along the
/// explanatory `rules` (predicate == `explains`) to the root cause — the missing duty or bad
/// act that accounts for it. Composes `abductive::abductive_explanation`.
pub fn diagnose_breach(rules: &[NQuin], violation: u64, explains: u64) -> Option<u64> {
    crate::modalities::abductive::abductive_explanation(rules, violation, explains)
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
        let within = q_hash("https://ns.webcivics.net/jurisdiction/within");
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

    // ─── Cluster B ──────────────────────────────────────────────────────────────

    #[test]
    fn argumentation_resolves_norm_conflict() {
        // General duty A is attacked by exception E; E is attacked by override O (unattacked).
        // Grounded: O survives → defeats E → A is reinstated.
        let a = q_hash("norm:dutyToStop");
        let e = q_hash("norm:exceptionPolice");
        let o = q_hash("norm:overrideEmergency");
        let ids = [a, e, o];
        let attacks = [(e, a), (o, e)];
        assert!(norm_survives_conflict(&ids, &attacks, a), "A reinstated by O defeating E");
        assert!(norm_survives_conflict(&ids, &attacks, o), "O is unattacked → survives");
        assert!(!norm_survives_conflict(&ids, &attacks, e), "E is defeated by O");
    }

    fn deg(d: f32) -> NQuin {
        let mut q = NQuin::default();
        q.metadata = d.to_bits() as u64; // truth degree in metadata (fuzzy::degree)
        q
    }

    #[test]
    fn fuzzy_partial_fulfilment_and_trust() {
        // Progressive realization: the weakest sub-requirement gates the whole.
        let reqs = [deg(0.9), deg(0.6), deg(0.8)];
        assert!((fulfilment_degree(&reqs) - 0.6).abs() < 1e-6);
        assert!(obligation_fuzzily_met(&reqs, 0.5));
        assert!(!obligation_fuzzily_met(&reqs, 0.7));
        // Behavioural-trust gate.
        assert!(trust_gate(0.85, 0.7));
        assert!(!trust_gate(0.5, 0.7));
    }

    #[test]
    fn asp_multi_remedy_scenarios() {
        use crate::modalities::asp::AspRule;
        // Under-determined remedy: x :- not y ; y :- not x → two stable models {x}, {y}.
        let x = q_hash("remedy:compensation");
        let y = q_hash("remedy:restitution");
        let atoms = [x, y];
        let rules = [AspRule::new(x, &[], &[y]), AspRule::new(y, &[], &[x])];
        let mut out = [0u64; 8];
        let n = remedy_scenarios(&atoms, &rules, &mut out);
        assert_eq!(n, 2, "two valid remedy scenarios");
    }

    #[test]
    fn abductive_breach_diagnosis() {
        // missing-funding → no-staff → service-failure (the observed breach). Root = missing-funding.
        let explains = q_hash("q42:explains");
        let edge = |h: u64, eff: u64| {
            let mut q = NQuin { subject: h, predicate: explains, object: eff, context: 0, metadata: 0, parity: 0 };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };
        let breach = q_hash("breach:serviceFailure");
        let nostaff = q_hash("cause:noStaff");
        let nofunding = q_hash("cause:missingFunding");
        let rules = [edge(nofunding, nostaff), edge(nostaff, breach)];
        assert_eq!(diagnose_breach(&rules, breach, explains), Some(nofunding), "root cause surfaced");
    }
}
