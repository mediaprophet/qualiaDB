//! 3D Anatomy Qapp — **core factor + body-system accumulation model** (slice 1).
//!
//! The stable substance both audience lenses (clinician OSCE-Prac aid; person wellbeing gist) build on.
//! A [`Factor`] — any of {pathology finding, condition, medication, food, herb, tea, nutrient,
//! supplement, lifestyle, environmental} — maps onto one or more **body systems** with an
//! [`Effect`] (adverse / supportive / modulating), an [`EvidenceTier`], and a magnitude. Given a
//! person's active factors, [`accumulate`] rolls them into **per-system burden**, [`interactions`]
//! finds compounding / opposing pairs (herb–drug, food–condition), and [`systemic_implications`]
//! emits **proposals** — never diagnoses.
//!
//! **Honesty boundaries baked in:** every emitted [`SystemicImplication`] carries
//! [`EpistemicStatus::Hypothesis`] and the dominant evidence tier of its contributors; community /
//! anecdotal claims sit at the lowest tier. No temporal projection here (slice 2); no advice.
//!
//! The 17 systems mirror `bundled/qapps/Anatomy/Knowledge/system-map.json` so the native 3D view and
//! this engine agree on identity.

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

/// A body system (mirrors the Anatomy qapp's `system-map.json` ids/labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodySystem {
    pub id: &'static str,
    pub label: &'static str,
}

/// The 17 seeded body systems (extensible — jurisdiction/ontology packs can add more later).
pub static BODY_SYSTEMS: &[BodySystem] = &[
    BodySystem { id: "circulatory", label: "Circulatory (Cardiovascular) System" },
    BodySystem { id: "respiratory", label: "Respiratory System" },
    BodySystem { id: "digestive", label: "Digestive System" },
    BodySystem { id: "nervous", label: "Nervous System" },
    BodySystem { id: "muscular", label: "Muscular System" },
    BodySystem { id: "skeletal", label: "Skeletal System" },
    BodySystem { id: "endocrine", label: "Endocrine System" },
    BodySystem { id: "immune_lymphatic", label: "Immune / Lymphatic System" },
    BodySystem { id: "integumentary", label: "Integumentary System" },
    BodySystem { id: "urinary", label: "Urinary (Excretory) System" },
    BodySystem { id: "reproductive", label: "Reproductive System" },
    BodySystem { id: "sensory", label: "Sensory System" },
    BodySystem { id: "vestibular", label: "Vestibular System" },
    BodySystem { id: "exocrine", label: "Exocrine System" },
    BodySystem { id: "ecs", label: "Endocannabinoid System (ECS)" },
    BodySystem { id: "ens", label: "Enteric Nervous System (ENS)" },
    BodySystem { id: "glymphatic", label: "Glymphatic System" },
];

/// Look up a body system by id.
pub fn body_system(id: &str) -> Option<&'static BodySystem> {
    BODY_SYSTEMS.iter().find(|s| s.id == id)
}

/// The kind of factor mapping onto the body. Open-ended via `Other`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorKind {
    PathologyFinding,
    Condition,
    Medication,
    Food,
    Herb,
    Tea,
    WholeFood,
    Nutrient,
    Supplement,
    /// sleep / exercise / social, etc.
    Lifestyle,
    /// heat, season, activity exposure, etc.
    Environmental,
    Other(String),
}

/// The direction of a factor's effect on a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Adds load / strain.
    Adverse,
    /// Relieves load / supports recovery.
    Supportive,
    /// Changes behaviour without a clear +/- (interaction-relevant).
    Modulating,
}

/// Evidence backing a factor→system mapping, highest → lowest. Traditional-medicine and community
/// knowledge are **preserved at their own tier, never collapsed into "medical fact"**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTier {
    /// Lowest — internet / anecdotal "hot takes"; always a Hypothesis.
    CommunityAnecdotal,
    /// Documented traditional / folk use.
    TraditionalUse,
    /// Nutritional-database composition/association.
    NutritionalData,
    /// Plausible biological mechanism.
    Mechanistic,
    /// Clinical-trial / guideline evidence (highest).
    ClinicalEvidence,
}

impl EvidenceTier {
    /// Whether this tier is strong enough that a *source record* could be `Asserted` rather than a
    /// `Hypothesis`. (Computed systemic implications are always `Hypothesis` regardless.)
    pub fn is_clinical(self) -> bool {
        self == EvidenceTier::ClinicalEvidence
    }
}

/// One (system, effect, evidence, magnitude) mapping carried by a factor. `weight_milli` is a
/// bounded 0..=1000 contribution magnitude (integer, no float health arithmetic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorTarget {
    pub system_id: String,
    pub effect: Effect,
    pub evidence: EvidenceTier,
    pub weight_milli: u32,
}

/// A factor mapping onto the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Factor {
    pub id: String,
    pub kind: FactorKind,
    pub label: String,
    pub targets: Vec<FactorTarget>,
    /// Source / provenance reference (a record id, knowledge-source id, or citation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Factor {
    pub fn new(id: impl Into<String>, kind: FactorKind, label: impl Into<String>) -> Self {
        Self { id: id.into(), kind, label: label.into(), targets: Vec::new(), source: None }
    }

    pub fn targeting(
        mut self,
        system_id: impl Into<String>,
        effect: Effect,
        evidence: EvidenceTier,
        weight_milli: u32,
    ) -> Self {
        self.targets.push(FactorTarget {
            system_id: system_id.into(),
            effect,
            evidence,
            weight_milli: weight_milli.min(1000),
        });
        self
    }
}

/// Aggregated load on one body system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SystemBurden {
    pub system_id: String,
    pub adverse_milli: u32,
    pub supportive_milli: u32,
    /// `adverse - supportive`, floored at 0 (support cannot make a system "better than baseline" here).
    pub net_milli: u32,
    /// Factor ids contributing an adverse effect (the "convergence" set).
    pub adverse_contributors: Vec<String>,
    /// Factor ids contributing a supportive effect.
    pub supportive_contributors: Vec<String>,
}

/// Roll a set of factors into per-system burden. Deterministic; systems appear in `BODY_SYSTEMS`
/// order; only systems with any contribution are returned.
pub fn accumulate(factors: &[Factor]) -> Vec<SystemBurden> {
    use std::collections::BTreeMap;
    let mut by_system: BTreeMap<&str, SystemBurden> = BTreeMap::new();
    for factor in factors {
        for t in &factor.targets {
            let entry = by_system.entry(system_key(&t.system_id)).or_insert_with(|| SystemBurden {
                system_id: t.system_id.clone(),
                ..Default::default()
            });
            match t.effect {
                Effect::Adverse => {
                    entry.adverse_milli = entry.adverse_milli.saturating_add(t.weight_milli);
                    push_unique(&mut entry.adverse_contributors, &factor.id);
                }
                Effect::Supportive => {
                    entry.supportive_milli = entry.supportive_milli.saturating_add(t.weight_milli);
                    push_unique(&mut entry.supportive_contributors, &factor.id);
                }
                Effect::Modulating => {}
            }
        }
    }
    for b in by_system.values_mut() {
        b.net_milli = b.adverse_milli.saturating_sub(b.supportive_milli);
    }
    // Emit in canonical system order, then any unknown system ids alphabetically (BTreeMap order).
    let mut out: Vec<SystemBurden> = Vec::new();
    for sys in BODY_SYSTEMS {
        if let Some(b) = by_system.remove(sys.id) {
            out.push(b);
        }
    }
    out.extend(by_system.into_values());
    out
}

/// A compounding or opposing pair of factors on the same system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interaction {
    pub system_id: String,
    pub factor_a: String,
    pub factor_b: String,
    pub kind: InteractionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    /// A medication and a herb both loading the same system — a herb–drug interaction to flag.
    HerbDrug,
    /// Two adverse factors compounding on one system.
    Compounding,
    /// An adverse and a supportive factor on one system (mitigation).
    Opposing,
}

/// Find pairwise interactions among factors sharing a system. O(system × factors²) but bounded by a
/// person's factor count; deterministic ordering.
pub fn interactions(factors: &[Factor]) -> Vec<Interaction> {
    let mut out = Vec::new();
    for i in 0..factors.len() {
        for j in (i + 1)..factors.len() {
            let (a, b) = (&factors[i], &factors[j]);
            for ta in &a.targets {
                for tb in &b.targets {
                    if system_key(&ta.system_id) != system_key(&tb.system_id) {
                        continue;
                    }
                    let kind = classify_interaction(a, ta, b, tb);
                    if let Some(kind) = kind {
                        out.push(Interaction {
                            system_id: ta.system_id.clone(),
                            factor_a: a.id.clone(),
                            factor_b: b.id.clone(),
                            kind,
                        });
                    }
                }
            }
        }
    }
    out
}

fn classify_interaction(
    a: &Factor,
    ta: &FactorTarget,
    b: &Factor,
    tb: &FactorTarget,
) -> Option<InteractionKind> {
    let is_med = |f: &Factor| f.kind == FactorKind::Medication;
    let is_botanical =
        |f: &Factor| matches!(f.kind, FactorKind::Herb | FactorKind::Tea | FactorKind::Supplement);
    // Herb–drug: one medication + one botanical touching the same system (either direction).
    if (is_med(a) && is_botanical(b)) || (is_botanical(a) && is_med(b)) {
        return Some(InteractionKind::HerbDrug);
    }
    match (ta.effect, tb.effect) {
        (Effect::Adverse, Effect::Adverse) => Some(InteractionKind::Compounding),
        (Effect::Adverse, Effect::Supportive) | (Effect::Supportive, Effect::Adverse) => {
            Some(InteractionKind::Opposing)
        }
        _ => None,
    }
}

/// A computed, contestable proposal about a system under accumulated load. **Never a diagnosis.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemicImplication {
    pub system_id: String,
    pub system_label: String,
    pub net_milli: u32,
    /// Factor ids converging adverse load on this system (≥ the convergence threshold).
    pub converging_factors: Vec<String>,
    /// The strongest evidence tier among the contributing adverse mappings (drives how it's shown).
    pub dominant_evidence: EvidenceTier,
    /// Interactions on this system worth surfacing.
    pub interactions: Vec<Interaction>,
    /// Always `Hypothesis` — a computed association, not a fact/diagnosis.
    pub epistemic_status: EpistemicStatus,
}

/// Emit systemic-implication proposals for systems where at least `convergence_threshold` distinct
/// factors converge adverse load (and net load is non-zero). Deterministic, canonical-system order.
pub fn systemic_implications(
    factors: &[Factor],
    convergence_threshold: usize,
) -> Vec<SystemicImplication> {
    let burdens = accumulate(factors);
    let all_interactions = interactions(factors);
    let mut out = Vec::new();
    for b in burdens {
        if b.adverse_contributors.len() < convergence_threshold.max(1) || b.net_milli == 0 {
            continue;
        }
        let dominant = dominant_evidence(factors, &b.system_id);
        let sys_interactions: Vec<Interaction> = all_interactions
            .iter()
            .filter(|it| system_key(&it.system_id) == system_key(&b.system_id))
            .cloned()
            .collect();
        out.push(SystemicImplication {
            system_id: b.system_id.clone(),
            system_label: body_system(&b.system_id)
                .map(|s| s.label.to_string())
                .unwrap_or_else(|| b.system_id.clone()),
            net_milli: b.net_milli,
            converging_factors: b.adverse_contributors,
            dominant_evidence: dominant,
            interactions: sys_interactions,
            epistemic_status: EpistemicStatus::Hypothesis,
        });
    }
    out
}

/// The strongest evidence tier among adverse mappings targeting `system_id` (default lowest).
fn dominant_evidence(factors: &[Factor], system_id: &str) -> EvidenceTier {
    factors
        .iter()
        .flat_map(|f| &f.targets)
        .filter(|t| t.effect == Effect::Adverse && system_key(&t.system_id) == system_key(system_id))
        .map(|t| t.evidence)
        .max()
        .unwrap_or(EvidenceTier::CommunityAnecdotal)
}

fn system_key(id: &str) -> &str {
    id.trim()
}

fn push_unique(v: &mut Vec<String>, id: &str) {
    if !v.iter().any(|x| x == id) {
        v.push(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn seventeen_systems_and_lookup() {
        assert_eq!(BODY_SYSTEMS.len(), 17);
        assert_eq!(body_system("digestive").unwrap().label, "Digestive System");
        assert!(body_system("nope").is_none());
    }

    #[test]
    fn evidence_tiers_order_clinical_highest_community_lowest() {
        assert!(EvidenceTier::ClinicalEvidence > EvidenceTier::CommunityAnecdotal);
        assert!(EvidenceTier::ClinicalEvidence > EvidenceTier::TraditionalUse);
        assert!(EvidenceTier::TraditionalUse > EvidenceTier::CommunityAnecdotal);
    }

    #[test]
    fn accumulate_nets_adverse_minus_supportive_floored_at_zero() {
        let factors = vec![
            Factor::new("cond:nafld", FactorKind::Condition, "NAFLD").targeting(
                "digestive",
                Effect::Adverse,
                EvidenceTier::ClinicalEvidence,
                600,
            ),
            herb("herb:milk-thistle", "digestive", Effect::Supportive, 200),
        ];
        let burdens = accumulate(&factors);
        let dig = burdens.iter().find(|b| b.system_id == "digestive").unwrap();
        assert_eq!(dig.adverse_milli, 600);
        assert_eq!(dig.supportive_milli, 200);
        assert_eq!(dig.net_milli, 400);
        assert_eq!(dig.adverse_contributors, vec!["cond:nafld".to_string()]);

        // Support exceeding adverse floors net at 0 (never "better than baseline").
        let more_support = vec![
            med("med:x", "urinary", 100),
            herb("herb:y", "urinary", Effect::Supportive, 500),
        ];
        let b = accumulate(&more_support);
        assert_eq!(b.iter().find(|b| b.system_id == "urinary").unwrap().net_milli, 0);
    }

    #[test]
    fn convergence_only_flags_systems_meeting_the_threshold() {
        // Two distinct adverse factors on 'digestive', one on 'urinary'.
        let factors = vec![
            Factor::new("cond:nafld", FactorKind::Condition, "NAFLD").targeting(
                "digestive",
                Effect::Adverse,
                EvidenceTier::ClinicalEvidence,
                500,
            ),
            med("med:hepatotoxic", "digestive", 300),
            med("med:nephro", "urinary", 400),
        ];
        let impl2 = systemic_implications(&factors, 2);
        assert_eq!(impl2.len(), 1, "only 'digestive' has 2 converging adverse factors");
        assert_eq!(impl2[0].system_id, "digestive");
        assert_eq!(impl2[0].converging_factors.len(), 2);
        // Always a hypothesis, never a diagnosis.
        assert_eq!(impl2[0].epistemic_status, EpistemicStatus::Hypothesis);
        // Dominant evidence is the strongest adverse mapping (clinical here).
        assert_eq!(impl2[0].dominant_evidence, EvidenceTier::ClinicalEvidence);

        // Threshold 1 flags both systems.
        assert_eq!(systemic_implications(&factors, 1).len(), 2);
    }

    #[test]
    fn herb_drug_interaction_is_detected_on_a_shared_system() {
        let factors = vec![
            med("med:warfarin", "circulatory", 400),
            herb("herb:ginkgo", "circulatory", Effect::Adverse, 300),
        ];
        let ix = interactions(&factors);
        assert_eq!(ix.len(), 1);
        assert_eq!(ix[0].kind, InteractionKind::HerbDrug);
        assert_eq!(ix[0].system_id, "circulatory");

        // Two conditions compounding (not a herb-drug) → Compounding.
        let compounding = vec![
            Factor::new("a", FactorKind::Condition, "a").targeting("respiratory", Effect::Adverse, EvidenceTier::ClinicalEvidence, 100),
            Factor::new("b", FactorKind::Condition, "b").targeting("respiratory", Effect::Adverse, EvidenceTier::ClinicalEvidence, 100),
        ];
        assert_eq!(interactions(&compounding)[0].kind, InteractionKind::Compounding);
    }

    #[test]
    fn community_hot_take_stays_lowest_tier_and_only_a_hypothesis() {
        // An anecdotal internet claim converging with nothing else — at threshold 1 it surfaces, but
        // as a Hypothesis at the CommunityAnecdotal tier (clearly "unverified" when shown).
        let factors = vec![Factor::new(
            "post:reddit-123",
            FactorKind::Herb,
            "someone said this tea 'detoxes' the liver",
        )
        .targeting("digestive", Effect::Adverse, EvidenceTier::CommunityAnecdotal, 50)];
        let imps = systemic_implications(&factors, 1);
        assert_eq!(imps.len(), 1);
        assert_eq!(imps[0].dominant_evidence, EvidenceTier::CommunityAnecdotal);
        assert_eq!(imps[0].epistemic_status, EpistemicStatus::Hypothesis);
    }

    #[test]
    fn model_serde_round_trips() {
        let f = med("med:x", "digestive", 100);
        let json = serde_json::to_string(&f).unwrap();
        let back: Factor = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }
}
