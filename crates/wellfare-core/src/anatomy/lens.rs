//! S4 (domain half) — the **two-lens view model**.
//!
//! One engine, two audiences (plan §1). Given a person's factors, [`build_view`] produces an
//! [`AnatomyView`] shaped for a [`Lens`]:
//!
//! - **[`Lens::Person`]** — a plain, simple "how am I doing" picture. Every system with any load is
//!   shown with a plain-language headline and a coarse [`WellbeingLevel`]; detail sits behind
//!   progressive disclosure (accessibility is core). The hard boundary: **not advice** — "worth
//!   discussing with a clinician".
//! - **[`Lens::Clinician`]** — an OSCE-Prac *aid*. Only the convergence-flagged systems are surfaced,
//!   with the converging factors, herb–drug / compounding / opposing interactions, and the dominant
//!   evidence tier, framed as **considerations** for the professional's own evaluation. The hard
//!   boundary: **not a diagnosis or an order**.
//!
//! Both lenses are honest by construction: computed systemic implications carry
//! [`EpistemicStatus::Hypothesis`], evidence tiers are shown (never collapsed into "fact"), and every
//! view carries an explicit uncertainty note. **The clinician lens deliberately surfaces only
//! *structural* considerations** (what converges, what interacts, at what evidence tier) — it does not
//! invent clinical specifics (which tests to order, which drug to start); those are Timothy's / a
//! clinician's sign-off (⚑ S4 curation datum), added later as data, not fabricated here.
//!
//! The raw per-system burden that colours the 3D body (S5) is [`accumulate`](super::accumulate) —
//! lens-independent; this module produces the *narrative*, not the colour.

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

use super::accumulate::{accumulate, interactions, systemic_implications, Interaction, InteractionKind};
use super::factor::{EvidenceTier, Factor};
use super::systems::body_system;

/// Which audience the view is shaped for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lens {
    Person,
    Clinician,
}

/// A coarse, plain wellbeing level for a system — never a diagnosis, never a number to the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WellbeingLevel {
    /// Little or nothing accumulating.
    Settled,
    /// A little going on — worth being aware of.
    WorthWatching,
    /// Several things adding up.
    UnderStrain,
}

impl WellbeingLevel {
    fn from_net(net_milli: u32) -> Self {
        match net_milli {
            0..=99 => WellbeingLevel::Settled,
            100..=299 => WellbeingLevel::WorthWatching,
            _ => WellbeingLevel::UnderStrain,
        }
    }
}

/// One system's entry in a view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemView {
    pub system_id: String,
    /// Clinical label (behind disclosure for the person lens).
    pub system_label: String,
    /// Plain-language label (the default the person sees).
    pub plain_label: String,
    pub level: WellbeingLevel,
    pub net_milli: u32,
    /// The one-line, lens-appropriate headline (plain for the person; structural for the clinician).
    pub headline: String,
    /// Progressive-disclosure detail — advanced items, hidden by default.
    pub detail: Vec<String>,
    /// Factor ids converging adverse load here (empty if below the convergence threshold).
    pub converging_factors: Vec<String>,
    pub dominant_evidence: EvidenceTier,
    pub interactions: Vec<Interaction>,
    /// `Hypothesis` for anything computed — never asserted as fact.
    pub epistemic_status: EpistemicStatus,
}

/// The full view for one lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnatomyView {
    pub lens: Lens,
    /// Systems with something to say, worst-first.
    pub systems: Vec<SystemView>,
    /// One plain overall sentence.
    pub summary: String,
    /// The hard framing boundary for this lens.
    pub boundary: String,
    /// Explicit uncertainty — illustrative, not a measurement/diagnosis.
    pub uncertainty_note: String,
}

const PERSON_BOUNDARY: &str =
    "This is a general picture to help you get a sense of things — it is not medical advice or a diagnosis. If anything here concerns you, it's worth discussing with a clinician.";
const CLINICIAN_BOUNDARY: &str =
    "Decision-support considerations only — not a diagnosis, a test order, or a prescription. Surfaced from the person's own records for the clinician's independent evaluation (OSCE-Prac aid).";
const UNCERTAINTY_NOTE: &str =
    "Illustrative model, not a measurement. Bodies vary; computed implications are hypotheses to explore, not facts.";

/// Build the view for a lens from a person's factors. `convergence_threshold` is the number of distinct
/// adverse factors on a system before it is "flagged" (see [`systemic_implications`]).
pub fn build_view(factors: &[Factor], lens: Lens, convergence_threshold: usize) -> AnatomyView {
    let burdens = accumulate(factors);
    let implications = systemic_implications(factors, convergence_threshold);
    let all_interactions = interactions(factors);

    let systems = match lens {
        Lens::Person => person_systems(&burdens, &implications),
        Lens::Clinician => clinician_systems(&implications, &all_interactions),
    };

    let summary = match lens {
        Lens::Person => person_summary(&systems),
        Lens::Clinician => clinician_summary(&systems, &all_interactions),
    };

    AnatomyView {
        lens,
        systems,
        summary,
        boundary: lens_boundary(lens).to_string(),
        uncertainty_note: UNCERTAINTY_NOTE.to_string(),
    }
}

fn lens_boundary(lens: Lens) -> &'static str {
    match lens {
        Lens::Person => PERSON_BOUNDARY,
        Lens::Clinician => CLINICIAN_BOUNDARY,
    }
}

/// Person lens: every system carrying net load, plain and simple, worst-first.
fn person_systems(
    burdens: &[super::accumulate::SystemBurden],
    implications: &[super::accumulate::SystemicImplication],
) -> Vec<SystemView> {
    let mut views: Vec<SystemView> = burdens
        .iter()
        .filter(|b| b.net_milli > 0)
        .map(|b| {
            let (label, plain) = labels_for(&b.system_id);
            let level = WellbeingLevel::from_net(b.net_milli);
            let imp = implications.iter().find(|i| i.system_id == b.system_id);
            let converging = imp.map(|i| i.converging_factors.clone()).unwrap_or_default();
            SystemView {
                headline: person_headline(&plain, level, converging.len()),
                detail: person_detail(&label, b),
                system_id: b.system_id.clone(),
                system_label: label,
                plain_label: plain,
                level,
                net_milli: b.net_milli,
                converging_factors: converging,
                dominant_evidence: imp
                    .map(|i| i.dominant_evidence)
                    .unwrap_or(EvidenceTier::CommunityAnecdotal),
                interactions: imp.map(|i| i.interactions.clone()).unwrap_or_default(),
                epistemic_status: EpistemicStatus::Hypothesis,
            }
        })
        .collect();
    views.sort_by(|a, b| b.net_milli.cmp(&a.net_milli).then(a.system_id.cmp(&b.system_id)));
    views
}

/// Clinician lens: the convergence-flagged systems, with structural considerations.
fn clinician_systems(
    implications: &[super::accumulate::SystemicImplication],
    _all_interactions: &[Interaction],
) -> Vec<SystemView> {
    let mut views: Vec<SystemView> = implications
        .iter()
        .map(|i| {
            let (label, plain) = labels_for(&i.system_id);
            SystemView {
                headline: clinician_headline(&label, i),
                detail: clinician_detail(i),
                system_id: i.system_id.clone(),
                system_label: label,
                plain_label: plain,
                level: WellbeingLevel::from_net(i.net_milli),
                net_milli: i.net_milli,
                converging_factors: i.converging_factors.clone(),
                dominant_evidence: i.dominant_evidence,
                interactions: i.interactions.clone(),
                epistemic_status: i.epistemic_status,
            }
        })
        .collect();
    views.sort_by(|a, b| b.net_milli.cmp(&a.net_milli).then(a.system_id.cmp(&b.system_id)));
    views
}

fn labels_for(system_id: &str) -> (String, String) {
    match body_system(system_id) {
        Some(s) => (s.label.to_string(), s.plain_label.to_string()),
        None => (system_id.to_string(), system_id.to_string()),
    }
}

fn person_headline(plain: &str, level: WellbeingLevel, converging: usize) -> String {
    match level {
        WellbeingLevel::Settled => format!("Your {plain} looks settled."),
        WellbeingLevel::WorthWatching => {
            format!("Your {plain} has a little going on — worth being aware of.")
        }
        WellbeingLevel::UnderStrain if converging >= 2 => {
            format!("A few things you've logged seem to be adding up for your {plain}.")
        }
        WellbeingLevel::UnderStrain => {
            format!("Something you've logged is weighing on your {plain}.")
        }
    }
}

fn person_detail(label: &str, b: &super::accumulate::SystemBurden) -> Vec<String> {
    let mut d = Vec::new();
    d.push(format!("System: {label}."));
    if !b.adverse_contributors.is_empty() {
        d.push(format!("Contributing: {}.", b.adverse_contributors.join(", ")));
    }
    if !b.supportive_contributors.is_empty() {
        d.push(format!("Helping: {}.", b.supportive_contributors.join(", ")));
    }
    d
}

fn clinician_headline(label: &str, i: &super::accumulate::SystemicImplication) -> String {
    let n = i.converging_factors.len();
    let ix = i.interactions.len();
    let mut h = format!(
        "{label}: {n} converging factor{} (net load {}, dominant evidence {}).",
        if n == 1 { "" } else { "s" },
        i.net_milli,
        evidence_word(i.dominant_evidence),
    );
    if ix > 0 {
        h.push_str(&format!(" {ix} interaction{} to review.", if ix == 1 { "" } else { "s" }));
    }
    h.push_str(" Consider whether this pattern warrants review.");
    h
}

fn clinician_detail(i: &super::accumulate::SystemicImplication) -> Vec<String> {
    let mut d = Vec::new();
    d.push(format!("Converging factors: {}.", i.converging_factors.join(", ")));
    for it in &i.interactions {
        d.push(format!(
            "{}: {} + {} on this system — {}.",
            interaction_word(it.kind),
            it.factor_a,
            it.factor_b,
            interaction_note(it.kind),
        ));
    }
    d.push(format!("Dominant evidence tier: {}.", evidence_word(i.dominant_evidence)));
    d.push("Structural considerations only — clinical specifics (tests, medications, referrals) are the clinician's own judgement.".to_string());
    d
}

fn person_summary(systems: &[SystemView]) -> String {
    let strained = systems.iter().filter(|s| s.level == WellbeingLevel::UnderStrain).count();
    match (systems.len(), strained) {
        (0, _) => "Nothing much is adding up right now from what you've logged.".to_string(),
        (_, 0) => "A few small things to be aware of, but nothing is standing out strongly.".to_string(),
        (_, n) => format!(
            "A few things seem to be adding up across {n} area{}. It may be worth a chat with a clinician.",
            if n == 1 { "" } else { "s" }
        ),
    }
}

fn clinician_summary(systems: &[SystemView], all_interactions: &[Interaction]) -> String {
    let herb_drug =
        all_interactions.iter().filter(|i| i.kind == InteractionKind::HerbDrug).count();
    let mut s = format!(
        "{} system{} flagged by convergence.",
        systems.len(),
        if systems.len() == 1 { "" } else { "s" }
    );
    if herb_drug > 0 {
        s.push_str(&format!(
            " {herb_drug} herb–drug interaction{} present.",
            if herb_drug == 1 { "" } else { "s" }
        ));
    }
    s
}

fn evidence_word(t: EvidenceTier) -> &'static str {
    match t {
        EvidenceTier::CommunityAnecdotal => "community/anecdotal",
        EvidenceTier::TraditionalUse => "traditional-use",
        EvidenceTier::NutritionalData => "nutritional-data",
        EvidenceTier::Mechanistic => "mechanistic",
        EvidenceTier::ClinicalEvidence => "clinical",
    }
}

fn interaction_word(k: InteractionKind) -> &'static str {
    match k {
        InteractionKind::HerbDrug => "Herb–drug",
        InteractionKind::Compounding => "Compounding",
        InteractionKind::Opposing => "Opposing",
    }
}

fn interaction_note(k: InteractionKind) -> &'static str {
    match k {
        InteractionKind::HerbDrug => "a medication and a botanical acting on the same system",
        InteractionKind::Compounding => "two adverse factors on the same system",
        InteractionKind::Opposing => "an adverse and a supportive factor on the same system",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{Effect, FactorKind};

    fn med(id: &str, system: &str, w: u32) -> Factor {
        Factor::new(id, FactorKind::Medication, id).targeting(
            system,
            Effect::Adverse,
            EvidenceTier::ClinicalEvidence,
            w,
        )
    }
    fn herb(id: &str, system: &str, effect: Effect, w: u32) -> Factor {
        Factor::new(id, FactorKind::Herb, id).targeting(system, effect, EvidenceTier::TraditionalUse, w)
    }

    #[test]
    fn person_lens_is_plain_non_diagnostic_and_worst_first() {
        let factors = vec![
            med("cond:nafld", "digestive", 500),
            med("med:hepatotoxic", "digestive", 300),
            med("med:mild", "urinary", 120),
        ];
        let view = build_view(&factors, Lens::Person, 2);
        assert_eq!(view.lens, Lens::Person);
        // Digestive (net 800) before urinary (net 120).
        assert_eq!(view.systems[0].system_id, "digestive");
        assert!(view.systems[0].net_milli > view.systems[1].net_milli);
        // Plain label surfaced; headline uses it and is non-diagnostic.
        assert_eq!(view.systems[0].plain_label, "digestion");
        assert!(view.systems[0].headline.contains("digestion"));
        assert!(!view.systems[0].headline.to_lowercase().contains("diagnos"));
        // Everything computed is a hypothesis; the boundary says "not medical advice".
        assert!(view.systems.iter().all(|s| s.epistemic_status == EpistemicStatus::Hypothesis));
        assert!(view.boundary.contains("not medical advice"));
    }

    #[test]
    fn clinician_lens_surfaces_structural_considerations_and_the_herb_drug_interaction() {
        let factors = vec![
            med("med:warfarin", "circulatory", 400),
            herb("herb:ginkgo", "circulatory", Effect::Adverse, 300),
        ];
        let view = build_view(&factors, Lens::Clinician, 2);
        assert_eq!(view.systems.len(), 1);
        let c = &view.systems[0];
        assert_eq!(c.system_id, "circulatory");
        assert_eq!(c.interactions.len(), 1);
        assert_eq!(c.interactions[0].kind, InteractionKind::HerbDrug);
        // Headline is structural, includes the review prompt, and stays a hypothesis.
        assert!(c.headline.contains("converging factor"));
        assert!(c.headline.contains("Consider whether this pattern warrants review"));
        assert_eq!(c.epistemic_status, EpistemicStatus::Hypothesis);
        // Boundary forbids diagnosis/order; summary counts the herb–drug interaction.
        assert!(view.boundary.contains("not a diagnosis"));
        assert!(view.summary.contains("herb–drug"));
        // Detail explicitly leaves clinical specifics to the clinician (no invented tests/meds).
        assert!(c.detail.iter().any(|d| d.contains("clinician's own judgement")));
    }

    #[test]
    fn person_summary_is_gentle_when_nothing_converges() {
        let view = build_view(&[], Lens::Person, 2);
        assert!(view.systems.is_empty());
        assert!(view.summary.contains("Nothing much is adding up"));
    }

    #[test]
    fn view_serde_round_trips() {
        let view = build_view(&[med("m", "digestive", 400)], Lens::Person, 1);
        let json = serde_json::to_string(&view).unwrap();
        let back: AnatomyView = serde_json::from_str(&json).unwrap();
        assert_eq!(view, back);
    }
}
