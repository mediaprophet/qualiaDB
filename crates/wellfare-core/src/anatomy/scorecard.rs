//! The **score-card** — an accumulative, fully-traceable interpretation across health-relevant *aspects*.
//!
//! The per-system [`SystemBurden`](super::accumulate::SystemBurden) layer answers "where is load
//! converging". This layer answers the question a person actually asks: *"so, overall — how are things,
//! and why?"* — but it answers it the **opposite way to a health-credit score.** It is:
//!
//! - **Multi-aspect, never one reductive number.** A card of [`Aspect`]s (systemic load, stress,
//!   resilience, convergence, interaction load, physiological demand) — a person is not a single grade.
//! - **Fully traceable — the antithesis of a black box.** Every [`AspectScore`] carries the
//!   [`Contribution`]s that produced it (which systems / factors / interactions / state, each with its
//!   weight and evidence tier). The score is never asserted; it is *shown its work*.
//! - **A transparent, editable weights model.** [`WeightModel`] is a plain, inspectable table of
//!   `(system, aspect) → weight` — a "human weights model" you can read and change, not a learned net.
//! - **The person's own tool, not an assessment weapon.** It is a discussion aid for the data-subject
//!   and *their chosen* clinician (see the care-relationship stance: a counter-record, **not** a public
//!   rating). It scores load / stress / resilience / convergence — the discussable *precursors* — and
//!   **explicitly does not score "illness" or "disease"**: that would be a diagnosis, which this never is.
//! - **Control *enables* knowing — protection and enablement are one right, not opposed.** As much as
//!   information can be weaponised, the *lack* of it is equally a harm: the right to know oneself with real
//!   science, and to move hypothesis → test → specialist input → action, is itself a human right. The
//!   forum-internum classification (below) is not a lock that keeps a person ignorant — it is *their
//!   control*, which is exactly what makes it safe for them to know, and to *deliberately* share on their
//!   own terms with a specialist they choose. Each aspect is a `Hypothesis` — a **pathway start** toward
//!   testing / specialist input / support (the investigative-pathway layer), never a dead-end verdict.
//!
//! **Honesty (unchanged).** Integer-only, coarse bands, deterministic; every aspect is an
//! [`EpistemicStatus::Hypothesis`]; traditional/community evidence is preserved at its own tier. The
//! **weights themselves are an illustrative seed** ([`seed_weight_model`]) of well-established directions —
//! the authoritative weighting + framing is curation-grade (Timothy's / an expert's to supply), exactly as
//! for the physiology seed.

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

use super::accumulate::{
    accumulate, interactions, systemic_implications, Interaction, InteractionKind, SystemBurden,
    SystemicImplication,
};
use super::factor::{EvidenceTier, Factor};
use super::physiology::{state_modulator, whole_body_profile, PhysiologicalState};
use super::system_key;
use super::systems::body_system;

/// A health-relevant **aspect** the card interprets — a discussion *lens*, not a diagnosis axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aspect {
    /// Overall accumulated adverse load across all systems.
    SystemicLoad,
    /// Load concentrated on the stress-responsive systems (nervous / endocrine / ECS …).
    Stress,
    /// The supportive / recovery side — what is *helping*. First-class so the card is never only-negatives.
    Resilience,
    /// How much load is *converging* — multiple distinct factors pointing at the same system(s).
    Convergence,
    /// Load from factors *combining* — herb–drug pairs and compounding.
    InteractionLoad,
    /// The current physiological state's whole-body engagement (the reproductive-continuum layer).
    PhysiologicalDemand,
}

impl Aspect {
    /// All aspects, in card order.
    pub fn all() -> [Aspect; 6] {
        [
            Aspect::SystemicLoad,
            Aspect::Stress,
            Aspect::Resilience,
            Aspect::Convergence,
            Aspect::InteractionLoad,
            Aspect::PhysiologicalDemand,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Aspect::SystemicLoad => "Systemic load",
            Aspect::Stress => "Stress load",
            Aspect::Resilience => "Resilience & support",
            Aspect::Convergence => "Converging factors",
            Aspect::InteractionLoad => "Combined-effect load",
            Aspect::PhysiologicalDemand => "Physiological demand",
        }
    }

    /// Accessibility-first plain wording.
    pub fn plain_label(self) -> &'static str {
        match self {
            Aspect::SystemicLoad => "how much is adding up across your body",
            Aspect::Stress => "load on the stress-response systems",
            Aspect::Resilience => "what's helping and supporting recovery",
            Aspect::Convergence => "several things pointing at the same place",
            Aspect::InteractionLoad => "things that may combine",
            Aspect::PhysiologicalDemand => "the extra work of your current life stage",
        }
    }

    /// `Resilience` reads "higher is better"; every other aspect reads "higher = more to discuss". Lets a
    /// UI colour the bands correctly (a high resilience band is reassuring, not a warning).
    pub fn higher_is_supportive(self) -> bool {
        matches!(self, Aspect::Resilience)
    }
}

/// A coarse, dignity-framed band. **Never** a clinical grade or a pass/fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreBand {
    /// Little to nothing accumulating here.
    Settled,
    /// Some load building.
    Building,
    /// Notably heightened — worth a look / a conversation.
    Heightened,
    /// Markedly heightened — worth discussing with a clinician you trust.
    Marked,
}

impl ScoreBand {
    fn from_score(score_milli: u32) -> ScoreBand {
        match score_milli {
            0..=99 => ScoreBand::Settled,
            100..=349 => ScoreBand::Building,
            350..=649 => ScoreBand::Heightened,
            _ => ScoreBand::Marked,
        }
    }
}

/// What kind of underlying consideration a [`Contribution`] links to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    /// A body system's accumulated load.
    System,
    /// A specific interaction between factors.
    Interaction,
    /// A systemic-implication convergence.
    Convergence,
    /// The current physiological state's engagement of a system.
    State,
}

/// A single **linkage to an underlying consideration** — the traceability that makes the score-card the
/// opposite of a black box. Says *what* contributed, *how much* (weighted), and on *what evidence*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub kind: ContributionKind,
    /// The underlying id (system id, factor id, …) — the click-through target.
    pub source_id: String,
    /// Human-readable label.
    pub label: String,
    /// This source's weighted contribution to the aspect score (integer milli).
    pub weighted_milli: u32,
    /// The evidence backing this contribution (never collapsed across tiers).
    pub evidence: EvidenceTier,
}

/// One aspect's accumulated score, its band, and — crucially — the linkages that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectScore {
    pub aspect: Aspect,
    /// Accumulated 0..=1000.
    pub score_milli: u32,
    pub band: ScoreBand,
    /// The underlying considerations, strongest first. Empty ⇒ nothing accumulating for this aspect.
    pub contributions: Vec<Contribution>,
    /// Strongest evidence tier among the contributions (drives how prominently it's shown).
    pub dominant_evidence: EvidenceTier,
    /// Always `Hypothesis` — a computed interpretation, never a fact/diagnosis.
    pub epistemic_status: EpistemicStatus,
}

/// Where a piece of content sits in the selfhood / personhood taxonomy.
///
/// - **`Internum`** — *forum internum*: the inward domain of the person (the mind, the self, one's own
///   assessment of one's own body). Near-absolute and **non-derogable** — the person's alone.
/// - **`Externum`** — *forum externum*: outward conduct/agency, proportionately regulable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForumClass {
    Internum,
    Externum,
}

/// The whole card — one [`AspectScore`] per [`Aspect`], in [`Aspect::all`] order.
///
/// **A score-card is selfhood content.** It is a person's *inward self-assessment* of their own body —
/// [`ForumClass::Internum`], the near-absolute, non-derogable inner domain — **never** a third party's
/// assessment *of* them. As a derivation from health records it inherits the **most-restrictive**
/// sensitivity class (`Sanctuary`) under the high-water-mark rule, and defaults to non-disclosure. The
/// storage/consent layer must classify and gate it accordingly (see [`ScoreCard::forum_class`] /
/// [`ScoreCard::sensitivity_class`]); this typing is the structural backing for "the person's own tool,
/// not a rating weapon."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreCard {
    pub aspects: Vec<AspectScore>,
}

impl ScoreCard {
    /// The score for a given aspect, if present.
    pub fn aspect(&self, aspect: Aspect) -> Option<&AspectScore> {
        self.aspects.iter().find(|a| a.aspect == aspect)
    }

    /// A score-card is always **forum internum** — inward selfhood content, the person's alone.
    pub const fn forum_class(&self) -> ForumClass {
        ForumClass::Internum
    }

    /// The sensitivity class to store/handle a score-card under: always **`Sanctuary`** (the
    /// most-restrictive rung of the `Public → Restricted → Classified → Sanctuary` ladder). Returned as
    /// the ladder's canonical name so the storage layer (which owns the sensitivity enum) maps it without
    /// this pure domain crate depending on it. Absent classification already means "most-restrictive"; this
    /// is the explicit belt-and-braces.
    pub const fn sensitivity_class(&self) -> &'static str {
        "Sanctuary"
    }
}

// ===========================================================================
// The transparent weights model
// ===========================================================================

/// One weight in the model: how much a body system contributes to an aspect (percent; `0` = not at all,
/// `100` = fully). Plain and inspectable — this *is* the "human weights model", written out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemAspectWeight {
    pub system_id: String,
    pub aspect: Aspect,
    pub weight_pct: u32,
}

/// A readable, editable table of `(system, aspect) → weight`. The interpretive core of the card, exposed
/// rather than hidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WeightModel {
    pub system_weights: Vec<SystemAspectWeight>,
}

impl WeightModel {
    /// The weight for `(system_id, aspect)` (default `0` if unlisted).
    pub fn weight_for(&self, system_id: &str, aspect: Aspect) -> u32 {
        self.system_weights
            .iter()
            .find(|w| w.aspect == aspect && system_key(&w.system_id) == system_key(system_id))
            .map(|w| w.weight_pct)
            .unwrap_or(0)
    }
}

/// The **illustrative seed** weights (§: curation-grade content deferred to Timothy/an expert). Encodes
/// only well-established directions — which systems carry the *stress* axis, which carry *resilience-
/// relevant* recovery — coarse, not authoritative magnitudes. Extend/replace with a curated model.
pub fn seed_weight_model() -> WeightModel {
    // (system_id, stress_weight_pct, resilience_weight_pct)
    let rows: &[(&str, u32, u32)] = &[
        ("nervous", 100, 60),
        ("endocrine", 90, 50),
        ("ecs", 80, 70), // the endocannabinoid "internal balance" system
        ("circulatory", 50, 40),
        ("digestive", 40, 50),
        ("immune_lymphatic", 40, 60),
        ("glymphatic", 60, 80), // the brain's overnight clearance — sleep/recovery relevant
        ("muscular", 30, 50),
        ("respiratory", 40, 30),
    ];
    let mut system_weights = Vec::new();
    for (id, stress, resilience) in rows {
        if *stress > 0 {
            system_weights.push(SystemAspectWeight {
                system_id: id.to_string(),
                aspect: Aspect::Stress,
                weight_pct: *stress,
            });
        }
        if *resilience > 0 {
            system_weights.push(SystemAspectWeight {
                system_id: id.to_string(),
                aspect: Aspect::Resilience,
                weight_pct: *resilience,
            });
        }
    }
    WeightModel { system_weights }
}

// ===========================================================================
// Scoring
// ===========================================================================

fn clamp1000(v: u64) -> u32 {
    v.min(1000) as u32
}

/// Strongest evidence tier among a factor set's mappings (any effect) onto `system_id`.
fn system_evidence(factors: &[Factor], system_id: &str) -> EvidenceTier {
    factors
        .iter()
        .flat_map(|f| &f.targets)
        .filter(|t| system_key(&t.system_id) == system_key(system_id))
        .map(|t| t.evidence)
        .max()
        .unwrap_or(EvidenceTier::CommunityAnecdotal)
}

fn dominant(contributions: &[Contribution]) -> EvidenceTier {
    contributions
        .iter()
        .map(|c| c.evidence)
        .max()
        .unwrap_or(EvidenceTier::CommunityAnecdotal)
}

fn finish(aspect: Aspect, raw: u64, mut contributions: Vec<Contribution>) -> AspectScore {
    // Strongest contribution first (stable: then by label) — the "why", most-salient-first.
    contributions.sort_by(|a, b| {
        b.weighted_milli
            .cmp(&a.weighted_milli)
            .then(a.label.cmp(&b.label))
    });
    let score_milli = clamp1000(raw);
    AspectScore {
        aspect,
        score_milli,
        band: ScoreBand::from_score(score_milli),
        dominant_evidence: dominant(&contributions),
        contributions,
        epistemic_status: EpistemicStatus::Hypothesis,
    }
}

/// Severity (milli) an interaction contributes to the combined-effect aspect, by kind.
fn interaction_severity(kind: InteractionKind) -> u32 {
    match kind {
        InteractionKind::HerbDrug => 300, // a botanical + a medication on one system — worth checking
        InteractionKind::Compounding => 200, // two adverse factors compounding
        InteractionKind::Opposing => 60, // an adverse + a supportive (mitigation) — mild, informational
    }
}

/// Compute the full score-card from a person's factors, at a physiological state, under a weights model.
///
/// Load aspects are computed over the **state-modulated** burdens (the reproductive-continuum modulator
/// applied), so "the same factors, in this life stage" is what's scored. `convergence_threshold` is the
/// number of distinct factors converging on a system before it counts (as in
/// [`systemic_implications`](super::accumulate::systemic_implications)). Deterministic.
pub fn score_card(
    factors: &[Factor],
    convergence_threshold: usize,
    state: PhysiologicalState,
    weights: &WeightModel,
) -> ScoreCard {
    let raw_burdens = accumulate(factors);
    let burdens: Vec<SystemBurden> = state_modulator(state).apply_to_burdens(&raw_burdens);
    let implications: Vec<SystemicImplication> = systemic_implications(factors, convergence_threshold);
    let ix: Vec<Interaction> = interactions(factors);

    let mut aspects = Vec::new();
    aspects.push(score_systemic_load(&burdens, factors));
    aspects.push(score_weighted(Aspect::Stress, &burdens, factors, weights));
    aspects.push(score_resilience(&burdens, factors, weights));
    aspects.push(score_convergence(&implications));
    aspects.push(score_interactions(&ix));
    aspects.push(score_physiological_demand(state));
    ScoreCard { aspects }
}

/// Overall adverse load: every system's net counts fully (no per-system weighting — this is the sum).
fn score_systemic_load(burdens: &[SystemBurden], factors: &[Factor]) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for b in burdens {
        if b.net_milli == 0 {
            continue;
        }
        raw += b.net_milli as u64;
        contributions.push(Contribution {
            kind: ContributionKind::System,
            source_id: b.system_id.clone(),
            label: system_label(&b.system_id),
            weighted_milli: b.net_milli,
            evidence: system_evidence(factors, &b.system_id),
        });
    }
    finish(Aspect::SystemicLoad, raw, contributions)
}

/// A weighted-over-systems adverse aspect (Stress). Each system's net is scaled by its aspect weight.
fn score_weighted(
    aspect: Aspect,
    burdens: &[SystemBurden],
    factors: &[Factor],
    weights: &WeightModel,
) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for b in burdens {
        let w = weights.weight_for(&b.system_id, aspect);
        if w == 0 || b.net_milli == 0 {
            continue;
        }
        let weighted = (b.net_milli as u64 * w as u64) / 100;
        if weighted == 0 {
            continue;
        }
        raw += weighted;
        contributions.push(Contribution {
            kind: ContributionKind::System,
            source_id: b.system_id.clone(),
            label: system_label(&b.system_id),
            weighted_milli: weighted as u32,
            evidence: system_evidence(factors, &b.system_id),
        });
    }
    finish(aspect, raw, contributions)
}

/// Resilience: the *supportive* side, weighted. Higher = more support in play (reads as reassuring).
fn score_resilience(burdens: &[SystemBurden], factors: &[Factor], weights: &WeightModel) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for b in burdens {
        let w = weights.weight_for(&b.system_id, Aspect::Resilience);
        if w == 0 || b.supportive_milli == 0 {
            continue;
        }
        let weighted = (b.supportive_milli as u64 * w as u64) / 100;
        if weighted == 0 {
            continue;
        }
        raw += weighted;
        contributions.push(Contribution {
            kind: ContributionKind::System,
            source_id: b.system_id.clone(),
            label: system_label(&b.system_id),
            weighted_milli: weighted as u32,
            evidence: system_evidence(factors, &b.system_id),
        });
    }
    finish(Aspect::Resilience, raw, contributions)
}

/// Convergence: from the systemic-implication proposals. Each converging system contributes its net,
/// amplified by how many distinct factors converge (more convergence = more worth discussing).
fn score_convergence(implications: &[SystemicImplication]) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for im in implications {
        let n = im.converging_factors.len() as u64;
        // net × (converging count), so 3-factor convergence outweighs a 1-factor same-net system.
        let weighted = (im.net_milli as u64 * n) / 1;
        raw += weighted;
        contributions.push(Contribution {
            kind: ContributionKind::Convergence,
            source_id: im.system_id.clone(),
            label: format!("{} ({} converging)", im.system_label, im.converging_factors.len()),
            weighted_milli: clamp1000(weighted),
            evidence: im.dominant_evidence,
        });
    }
    finish(Aspect::Convergence, raw, contributions)
}

/// Combined-effect load: from detected interactions, by severity.
fn score_interactions(ix: &[Interaction]) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for it in ix {
        let sev = interaction_severity(it.kind);
        raw += sev as u64;
        contributions.push(Contribution {
            kind: ContributionKind::Interaction,
            source_id: format!("{}+{}", it.factor_a, it.factor_b),
            label: format!("{} & {} on {}", it.factor_a, it.factor_b, system_label(&it.system_id)),
            weighted_milli: sev,
            // Interactions are structural detections; tier them at Mechanistic (a plausible interaction),
            // not clinical certainty.
            evidence: EvidenceTier::Mechanistic,
        });
    }
    finish(Aspect::InteractionLoad, raw, contributions)
}

/// Physiological demand: from the current state's whole-body engagement profile.
fn score_physiological_demand(state: PhysiologicalState) -> AspectScore {
    let mut raw = 0u64;
    let mut contributions = Vec::new();
    for e in whole_body_profile(state) {
        // Engagement above baseline: (scale - 100) as the demand signal.
        let demand = (e.scale_pct.saturating_sub(100)) as u64;
        if demand == 0 {
            continue;
        }
        raw += demand;
        contributions.push(Contribution {
            kind: ContributionKind::State,
            source_id: e.system_id.clone(),
            label: e.system_label.clone(),
            weighted_milli: demand as u32,
            // The engagement is well-established baseline physiology (illustrative-seed magnitudes).
            evidence: EvidenceTier::Mechanistic,
        });
    }
    finish(Aspect::PhysiologicalDemand, raw, contributions)
}

fn system_label(system_id: &str) -> String {
    body_system(system_id)
        .map(|s| s.label.to_string())
        .unwrap_or_else(|| system_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::factor::{Effect, FactorKind, FactorTarget};
    use crate::anatomy::physiology::{ReproductiveState, Trimester};

    fn factor(id: &str, kind: FactorKind, targets: &[(&str, Effect, EvidenceTier, u32)]) -> Factor {
        let mut f = Factor::new(id, kind, id);
        f.targets = targets
            .iter()
            .map(|(s, e, ev, w)| FactorTarget {
                system_id: s.to_string(),
                effect: *e,
                evidence: *ev,
                weight_milli: *w,
            })
            .collect();
        f
    }

    fn baseline() -> PhysiologicalState {
        PhysiologicalState::Baseline
    }

    #[test]
    fn card_has_one_score_per_aspect_in_order() {
        let card = score_card(&[], 1, baseline(), &seed_weight_model());
        assert_eq!(card.aspects.len(), Aspect::all().len());
        for (got, want) in card.aspects.iter().zip(Aspect::all()) {
            assert_eq!(got.aspect, want);
        }
        // Empty factors → everything settled, no contributions, still Hypothesis.
        for a in &card.aspects {
            assert_eq!(a.band, ScoreBand::Settled);
            assert!(a.contributions.is_empty());
            assert_eq!(a.epistemic_status, EpistemicStatus::Hypothesis);
        }
    }

    #[test]
    fn every_score_traces_to_its_underlying_considerations() {
        // Two adverse factors converge on the nervous system (a stress-weighted system).
        let factors = vec![
            factor(
                "cond:chronic-stress",
                FactorKind::Condition,
                &[("nervous", Effect::Adverse, EvidenceTier::ClinicalEvidence, 400)],
            ),
            factor(
                "life:poor-sleep",
                FactorKind::Lifestyle,
                &[("nervous", Effect::Adverse, EvidenceTier::Mechanistic, 300)],
            ),
        ];
        let card = score_card(&factors, 2, baseline(), &seed_weight_model());

        // Systemic load is non-zero and links to the nervous system.
        let load = card.aspect(Aspect::SystemicLoad).unwrap();
        assert!(load.score_milli > 0);
        assert!(load.contributions.iter().any(|c| c.source_id == "nervous"));

        // Stress is weighted heavily on the nervous system → present, and traceable.
        let stress = card.aspect(Aspect::Stress).unwrap();
        assert!(stress.score_milli > 0);
        let nervous = stress.contributions.iter().find(|c| c.source_id == "nervous").unwrap();
        assert_eq!(nervous.kind, ContributionKind::System);
        assert!(!nervous.label.is_empty(), "human-readable linkage");
        // Dominant evidence is the strongest contributing tier (clinical here).
        assert_eq!(stress.dominant_evidence, EvidenceTier::ClinicalEvidence);

        // Convergence fires (2 factors converge on one system) and links back to it.
        let conv = card.aspect(Aspect::Convergence).unwrap();
        assert!(conv.score_milli > 0);
        assert!(conv.contributions.iter().any(|c| c.source_id == "nervous"));
    }

    #[test]
    fn resilience_reads_the_supportive_side_and_higher_is_good() {
        let factors = vec![factor(
            "act:meditation-and-sleep",
            FactorKind::Lifestyle,
            &[("nervous", Effect::Supportive, EvidenceTier::Mechanistic, 500)],
        )];
        let card = score_card(&factors, 1, baseline(), &seed_weight_model());
        let res = card.aspect(Aspect::Resilience).unwrap();
        assert!(res.score_milli > 0, "support shows up as resilience");
        assert!(res.contributions.iter().any(|c| c.source_id == "nervous"));
        assert!(Aspect::Resilience.higher_is_supportive());
        // Support alone leaves systemic load settled (net floors at 0).
        assert_eq!(card.aspect(Aspect::SystemicLoad).unwrap().band, ScoreBand::Settled);
    }

    #[test]
    fn interaction_load_scores_a_herb_drug_pair() {
        let factors = vec![
            factor(
                "med:warfarin",
                FactorKind::Medication,
                &[("circulatory", Effect::Adverse, EvidenceTier::ClinicalEvidence, 400)],
            ),
            factor(
                "herb:ginkgo",
                FactorKind::Herb,
                &[("circulatory", Effect::Adverse, EvidenceTier::TraditionalUse, 300)],
            ),
        ];
        let card = score_card(&factors, 1, baseline(), &seed_weight_model());
        let ix = card.aspect(Aspect::InteractionLoad).unwrap();
        assert_eq!(ix.score_milli, interaction_severity(InteractionKind::HerbDrug));
        assert_eq!(ix.contributions.len(), 1);
        assert_eq!(ix.contributions[0].kind, ContributionKind::Interaction);
    }

    #[test]
    fn physiological_demand_reflects_the_life_stage_and_state_modulates_load() {
        // A kidney-taxing med, scored in the third trimester: load is modulated up AND demand shows.
        let med = factor(
            "med:nephrotoxic",
            FactorKind::Medication,
            &[("urinary", Effect::Adverse, EvidenceTier::ClinicalEvidence, 400)],
        );
        let t3 = PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Third));
        let base_card = score_card(std::slice::from_ref(&med), 1, baseline(), &seed_weight_model());
        let preg_card = score_card(std::slice::from_ref(&med), 1, t3, &seed_weight_model());

        // Physiological demand is settled at baseline, present in pregnancy.
        assert_eq!(base_card.aspect(Aspect::PhysiologicalDemand).unwrap().band, ScoreBand::Settled);
        let demand = preg_card.aspect(Aspect::PhysiologicalDemand).unwrap();
        assert!(demand.score_milli > 0);
        assert!(demand.contributions.iter().any(|c| c.source_id == "urinary"));

        // The same med lands as more systemic load in the third trimester (renal engagement scales it).
        let base_load = base_card.aspect(Aspect::SystemicLoad).unwrap().score_milli;
        let preg_load = preg_card.aspect(Aspect::SystemicLoad).unwrap().score_milli;
        assert!(preg_load > base_load, "state modulates the load: {preg_load} > {base_load}");
    }

    #[test]
    fn bands_are_coarse_and_monotone() {
        assert_eq!(ScoreBand::from_score(0), ScoreBand::Settled);
        assert_eq!(ScoreBand::from_score(99), ScoreBand::Settled);
        assert_eq!(ScoreBand::from_score(100), ScoreBand::Building);
        assert_eq!(ScoreBand::from_score(349), ScoreBand::Building);
        assert_eq!(ScoreBand::from_score(350), ScoreBand::Heightened);
        assert_eq!(ScoreBand::from_score(649), ScoreBand::Heightened);
        assert_eq!(ScoreBand::from_score(650), ScoreBand::Marked);
        assert_eq!(ScoreBand::from_score(5000), ScoreBand::Marked);
    }

    #[test]
    fn a_score_card_is_forum_internum_selfhood_content_at_sanctuary_class() {
        // The classification is intrinsic to the type — every card, even an empty one, is the person's
        // inward, most-protected selfhood content. This is the structural backing of "not a rating weapon".
        let card = score_card(&[], 1, baseline(), &seed_weight_model());
        assert_eq!(card.forum_class(), ForumClass::Internum);
        assert_eq!(card.sensitivity_class(), "Sanctuary");
    }

    #[test]
    fn deterministic_and_serde_round_trips() {
        let factors = vec![factor(
            "x",
            FactorKind::Condition,
            &[("endocrine", Effect::Adverse, EvidenceTier::Mechanistic, 300)],
        )];
        let a = score_card(&factors, 1, baseline(), &seed_weight_model());
        let b = score_card(&factors, 1, baseline(), &seed_weight_model());
        assert_eq!(a, b, "deterministic");
        let json = serde_json::to_string(&a).unwrap();
        let back: ScoreCard = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
