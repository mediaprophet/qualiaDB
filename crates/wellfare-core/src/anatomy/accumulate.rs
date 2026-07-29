//! Non-temporal accumulation: roll a set of [`Factor`]s into per-system burden, find pairwise
//! interactions, and emit systemic-implication **proposals** (never diagnoses).

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

use super::factor::{Effect, EvidenceTier, Factor, FactorKind};
use super::systems::{BODY_SYSTEMS, body_system};
use super::{push_unique, system_key};

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
/// order (unknown ids after, alphabetically); only systems with any contribution are returned.
pub fn accumulate(factors: &[Factor]) -> Vec<SystemBurden> {
    use std::collections::BTreeMap;
    let mut by_system: BTreeMap<&str, SystemBurden> = BTreeMap::new();
    for factor in factors {
        for t in &factor.targets {
            let entry = by_system
                .entry(system_key(&t.system_id))
                .or_insert_with(|| SystemBurden {
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
    /// A medication and a botanical (herb / tea / supplement) both loading the same system.
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
                    if let Some(kind) = classify_interaction(a, ta.effect, b, tb.effect) {
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

fn classify_interaction(a: &Factor, ea: Effect, b: &Factor, eb: Effect) -> Option<InteractionKind> {
    let is_med = |f: &Factor| f.kind == FactorKind::Medication;
    let is_botanical = |f: &Factor| {
        matches!(
            f.kind,
            FactorKind::Herb | FactorKind::Tea | FactorKind::Supplement
        )
    };
    // Herb–drug: one medication + one botanical touching the same system (either direction).
    if (is_med(a) && is_botanical(b)) || (is_botanical(a) && is_med(b)) {
        return Some(InteractionKind::HerbDrug);
    }
    match (ea, eb) {
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
        .filter(|t| {
            t.effect == Effect::Adverse && system_key(&t.system_id) == system_key(system_id)
        })
        .map(|t| t.evidence)
        .max()
        .unwrap_or(EvidenceTier::CommunityAnecdotal)
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
        Factor::new(id, FactorKind::Herb, id).targeting(
            system,
            effect,
            EvidenceTier::TraditionalUse,
            w,
        )
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
        assert_eq!(
            b.iter()
                .find(|b| b.system_id == "urinary")
                .unwrap()
                .net_milli,
            0
        );
    }

    #[test]
    fn convergence_only_flags_systems_meeting_the_threshold() {
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
        assert_eq!(
            impl2.len(),
            1,
            "only 'digestive' has 2 converging adverse factors"
        );
        assert_eq!(impl2[0].system_id, "digestive");
        assert_eq!(impl2[0].converging_factors.len(), 2);
        assert_eq!(impl2[0].epistemic_status, EpistemicStatus::Hypothesis);
        assert_eq!(impl2[0].dominant_evidence, EvidenceTier::ClinicalEvidence);

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

        let compounding = vec![
            Factor::new("a", FactorKind::Condition, "a").targeting(
                "respiratory",
                Effect::Adverse,
                EvidenceTier::ClinicalEvidence,
                100,
            ),
            Factor::new("b", FactorKind::Condition, "b").targeting(
                "respiratory",
                Effect::Adverse,
                EvidenceTier::ClinicalEvidence,
                100,
            ),
        ];
        assert_eq!(
            interactions(&compounding)[0].kind,
            InteractionKind::Compounding
        );
    }

    #[test]
    fn community_hot_take_stays_lowest_tier_and_only_a_hypothesis() {
        let factors = vec![
            Factor::new(
                "post:reddit-123",
                FactorKind::Herb,
                "someone said this tea 'detoxes' the liver",
            )
            .targeting(
                "digestive",
                Effect::Adverse,
                EvidenceTier::CommunityAnecdotal,
                50,
            ),
        ];
        let imps = systemic_implications(&factors, 1);
        assert_eq!(imps.len(), 1);
        assert_eq!(imps[0].dominant_evidence, EvidenceTier::CommunityAnecdotal);
        assert_eq!(imps[0].epistemic_status, EpistemicStatus::Hypothesis);
    }
}
