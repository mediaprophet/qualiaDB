//! The **investigative pathway** — the engine of the thesis: *know thyself → hypothesis → what would help
//! you know → specialist input → action*.
//!
//! This is the point of Qualia made operational. The rest of the engine surfaces **hypotheses** (a
//! score-card aspect, a converging [`SystemicImplication`]) — honest, coarse, non-diagnostic. On its own a
//! hypothesis can be a dead-end or, worse, a locked-away worry. This layer turns each hypothesis into a
//! **path toward knowing and acting**: the questions worth considering, what could be observed or tested,
//! the levers the person actually controls, and *when* bringing in a specialist they trust would help — and
//! it **ranks** those by **value of information** (the one new primitive): which step would most help
//! discriminate among the live hypotheses and move the person from uncertainty toward understanding.
//!
//! **It enables; it never directs or diagnoses.** Every step is a *proposal*
//! ([`EpistemicStatus::Hypothesis`]) — "here is something that might help you find out / discuss / act on",
//! never "you have X" or "you must do Y". The person stays the agent; a specialist step points *to their
//! chosen* clinician (the care-relationship stance), not to a gatekeeper. This is the corrective to
//! information-asymmetry — the right to know oneself with real science — with the person in control.
//!
//! **Honesty (unchanged).** Integer-only, coarse, deterministic; evidence tiers preserved. The **step
//! library is curation-grade** — *which* questions/tests/levers attach to a hypothesis is Timothy's / an
//! expert's / a clinician's to supply (§9); this module builds the machinery (the pathway + the VOI
//! primitive + the ranking + the framing), not the authoritative content.

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

use super::accumulate::SystemicImplication;
use super::factor::EvidenceTier;

/// A hypothesis under investigation — e.g. a converging [`SystemicImplication`] or a score-card aspect.
/// Carries a **coarse prior** (0..=1000: how much is currently pointing at it) and its evidence tier.
/// Never a diagnosis; a thing worth *finding out more about*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub label: String,
    /// Coarse prior weight (how much is currently converging on this), 0..=1000.
    pub prior_milli: u32,
    pub evidence: EvidenceTier,
}

/// The kind of investigative step — the *modes* of moving from a hypothesis toward knowing/acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// A question worth considering (of oneself, or to raise with a clinician).
    Question,
    /// Something to notice / track over time (self-observation).
    Observation,
    /// An investigation / test that could help discriminate.
    Test,
    /// A lever the person themselves controls (sleep, movement, nutrition, …).
    LifestyleLever,
    /// When/why bringing in a specialist the person *chooses* would help.
    SpecialistInput,
}

/// One investigative step, and **which hypotheses it bears on** — the linkage that makes the pathway
/// traceable rather than a black box. A signed bearing per hypothesis: `> 0` a result would *support* it,
/// `< 0` a result would *tell against* it (that combination is what lets a step *discriminate*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigativeStep {
    pub id: String,
    pub kind: StepKind,
    pub label: String,
    /// `hypothesis_id → signed bearing` (magnitude ~0..=1000). Empty ⇒ bears on nothing (VOI 0).
    pub bears_on: Vec<(String, i32)>,
    pub evidence: EvidenceTier,
}

/// A step ranked by its value of information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedStep {
    pub step: InvestigativeStep,
    /// Value of information, 0..=1000.
    pub voi_milli: u32,
}

/// A ranked investigative pathway — the steps that would most help, most-valuable first. A **proposal**,
/// never a plan imposed on the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigativePathway {
    pub hypotheses: Vec<Hypothesis>,
    pub steps: Vec<RankedStep>,
    /// Always `Hypothesis` — a set of things that *might* help, never directives/diagnoses.
    pub epistemic_status: EpistemicStatus,
}

fn clamp1000(v: u64) -> u32 {
    v.min(1000) as u32
}

/// **Value of information** (0..=1000) of a step given the current hypotheses — the new primitive.
///
/// A transparent, monotone proxy (not a decision-theoretic optimum): it rewards a step that bears
/// **strongly on high-prior hypotheses** (*relevance*) and that would **separate** them — supporting some
/// while telling against others among the high-prior set (*discrimination*). A test that only bears on an
/// already-unlikely hypothesis is low-value; a test that would split two live, similarly-weighted
/// hypotheses is high-value — which is exactly what "what should I find out next" wants.
pub fn value_of_information(step: &InvestigativeStep, hypotheses: &[Hypothesis]) -> u32 {
    let mut relevance = 0u64;
    let mut support = 0u64; // relevance mass of the "would support" bearings
    let mut against = 0u64; // relevance mass of the "would tell against" bearings
    for (hid, bearing) in &step.bears_on {
        let Some(h) = hypotheses.iter().find(|h| &h.id == hid) else {
            continue;
        };
        // relevance contribution = prior × |bearing|, coarse-scaled.
        let contrib = h.prior_milli as u64 * bearing.unsigned_abs() as u64 / 1000;
        relevance += contrib;
        if *bearing > 0 {
            support += contrib;
        } else if *bearing < 0 {
            against += contrib;
        }
    }
    // Discrimination bonus: a step that both supports and tells-against (among high-prior hypotheses)
    // separates the field — reward the smaller of the two masses (you only "separate" as much as the
    // weaker side).
    let discrimination = support.min(against);
    clamp1000(relevance + discrimination)
}

/// Build the VOI-ranked investigative pathway for a set of hypotheses + candidate steps. Deterministic:
/// steps sort by VOI descending, then by id. Steps with zero VOI (bear on nothing live) are dropped — the
/// pathway shows only what would actually help.
pub fn investigative_pathway(
    hypotheses: Vec<Hypothesis>,
    steps: Vec<InvestigativeStep>,
) -> InvestigativePathway {
    let mut ranked: Vec<RankedStep> = steps
        .into_iter()
        .map(|step| {
            let voi_milli = value_of_information(&step, &hypotheses);
            RankedStep { step, voi_milli }
        })
        .filter(|r| r.voi_milli > 0)
        .collect();
    ranked.sort_by(|a, b| b.voi_milli.cmp(&a.voi_milli).then(a.step.id.cmp(&b.step.id)));
    InvestigativePathway {
        hypotheses,
        steps: ranked,
        epistemic_status: EpistemicStatus::Hypothesis,
    }
}

/// Derive investigable [`Hypothesis`]es from the anatomy engine's [`SystemicImplication`] proposals — the
/// bridge from "where is load converging" to "what's worth finding out about". The implication's net load
/// becomes the coarse prior; its dominant evidence carries through.
pub fn hypotheses_from_implications(implications: &[SystemicImplication]) -> Vec<Hypothesis> {
    implications
        .iter()
        .map(|im| Hypothesis {
            id: im.system_id.clone(),
            label: im.system_label.clone(),
            prior_milli: im.net_milli.min(1000),
            evidence: im.dominant_evidence,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hyp(id: &str, prior: u32) -> Hypothesis {
        Hypothesis {
            id: id.into(),
            label: id.into(),
            prior_milli: prior,
            evidence: EvidenceTier::Mechanistic,
        }
    }

    fn step(id: &str, kind: StepKind, bears: &[(&str, i32)]) -> InvestigativeStep {
        InvestigativeStep {
            id: id.into(),
            kind,
            label: id.into(),
            bears_on: bears.iter().map(|(h, w)| (h.to_string(), *w)).collect(),
            evidence: EvidenceTier::Mechanistic,
        }
    }

    #[test]
    fn voi_rewards_bearing_on_high_prior_hypotheses() {
        let hyps = vec![hyp("live", 800), hyp("unlikely", 50)];
        let on_live = step("s1", StepKind::Test, &[("live", 1000)]);
        let on_unlikely = step("s2", StepKind::Test, &[("unlikely", 1000)]);
        assert!(
            value_of_information(&on_live, &hyps) > value_of_information(&on_unlikely, &hyps),
            "a test bearing on the live hypothesis is worth more"
        );
    }

    #[test]
    fn voi_rewards_discrimination_between_two_live_hypotheses() {
        // Two similarly-live hypotheses. A step that would SUPPORT one and TELL AGAINST the other separates
        // them → higher VOI than a step that only bears on one.
        let hyps = vec![hyp("a", 600), hyp("b", 600)];
        let discriminating = step("disc", StepKind::Test, &[("a", 1000), ("b", -1000)]);
        let one_sided = step("one", StepKind::Test, &[("a", 1000)]);
        assert!(
            value_of_information(&discriminating, &hyps) > value_of_information(&one_sided, &hyps),
            "a test that separates two live hypotheses is the most informative"
        );
    }

    #[test]
    fn pathway_ranks_by_voi_and_drops_useless_steps() {
        let hyps = vec![hyp("live", 900), hyp("dead", 0)];
        let steps = vec![
            step("useless", StepKind::Question, &[("dead", 1000)]), // bears only on a dead hypothesis
            step("weak", StepKind::Observation, &[("live", 200)]),
            step("strong", StepKind::Test, &[("live", 1000)]),
        ];
        let path = investigative_pathway(hyps, steps);
        assert_eq!(path.epistemic_status, EpistemicStatus::Hypothesis);
        // The zero-VOI step is dropped; the strong step ranks first.
        let ids: Vec<&str> = path.steps.iter().map(|r| r.step.id.as_str()).collect();
        assert_eq!(ids, vec!["strong", "weak"], "ranked by VOI, useless dropped");
        assert!(path.steps[0].voi_milli > path.steps[1].voi_milli);
    }

    #[test]
    fn a_pathway_covers_the_modes_of_knowing_and_acting() {
        // The step kinds together span the enablement modes: ask, observe, test, act, and bring-in-a-specialist.
        let hyps = vec![hyp("h", 700)];
        let steps = vec![
            step("q", StepKind::Question, &[("h", 500)]),
            step("o", StepKind::Observation, &[("h", 500)]),
            step("t", StepKind::Test, &[("h", 800)]),
            step("l", StepKind::LifestyleLever, &[("h", 400)]),
            step("s", StepKind::SpecialistInput, &[("h", 600)]),
        ];
        let path = investigative_pathway(hyps, steps);
        let kinds: std::collections::BTreeSet<StepKind> =
            path.steps.iter().map(|r| r.step.kind).collect();
        assert!(kinds.contains(&StepKind::Question));
        assert!(kinds.contains(&StepKind::LifestyleLever)); // a lever the person controls
        assert!(kinds.contains(&StepKind::SpecialistInput)); // toward a chosen specialist
        // Every step is offered as part of a Hypothesis-status pathway (a proposal, never a directive).
        assert_eq!(path.epistemic_status, EpistemicStatus::Hypothesis);
    }

    #[test]
    fn empty_inputs_give_an_empty_but_honest_pathway() {
        let path = investigative_pathway(vec![], vec![]);
        assert!(path.steps.is_empty());
        assert_eq!(path.epistemic_status, EpistemicStatus::Hypothesis);
    }

    #[test]
    fn serde_round_trips() {
        let hyps = vec![hyp("h", 500)];
        let steps = vec![step("s", StepKind::Test, &[("h", 700)])];
        let path = investigative_pathway(hyps, steps);
        let json = serde_json::to_string(&path).unwrap();
        let back: InvestigativePathway = serde_json::from_str(&json).unwrap();
        assert_eq!(path, back);
    }
}
