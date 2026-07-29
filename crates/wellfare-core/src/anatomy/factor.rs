//! The [`Factor`] model — the general thing that maps onto the body, with its effect, evidence tier,
//! and bounded integer magnitude. Non-temporal; the temporal layer wraps a factor in a `FactorEvent`.

use serde::{Deserialize, Serialize};

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
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            targets: Vec::new(),
            source: None,
        }
    }

    /// Add a (system, effect, evidence, weight) mapping. `weight_milli` is clamped to 0..=1000.
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

    /// Attach a source / provenance reference.
    pub fn from_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tiers_order_clinical_highest_community_lowest() {
        assert!(EvidenceTier::ClinicalEvidence > EvidenceTier::CommunityAnecdotal);
        assert!(EvidenceTier::ClinicalEvidence > EvidenceTier::TraditionalUse);
        assert!(EvidenceTier::TraditionalUse > EvidenceTier::CommunityAnecdotal);
        assert!(EvidenceTier::ClinicalEvidence.is_clinical());
        assert!(!EvidenceTier::TraditionalUse.is_clinical());
    }

    #[test]
    fn weight_is_clamped_to_the_bounded_domain() {
        let f = Factor::new("x", FactorKind::Medication, "x").targeting(
            "digestive",
            Effect::Adverse,
            EvidenceTier::ClinicalEvidence,
            5000,
        );
        assert_eq!(f.targets[0].weight_milli, 1000);
    }

    #[test]
    fn model_serde_round_trips() {
        let f = Factor::new("med:x", FactorKind::Medication, "X")
            .targeting(
                "digestive",
                Effect::Adverse,
                EvidenceTier::ClinicalEvidence,
                100,
            )
            .from_source("record:123");
        let json = serde_json::to_string(&f).unwrap();
        let back: Factor = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }
}
