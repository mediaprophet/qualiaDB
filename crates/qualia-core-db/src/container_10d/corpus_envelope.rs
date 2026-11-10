//! Species-trait envelopes read from a species corpus.
//!
//! The hard-coded envelopes in [`super::metric_trait_gate`] are reference examples. A bound
//! with an evidenced assertion uses that assertion and cites its id. A bound with only a model
//! assumption cites that assumption. A bound with no assertion falls back to the reference
//! example and is labelled [`BoundBasis::ModelAssumption`].

use qualia_cooperative_core::contextual_evidence::{ClaimBasis, EvidenceValue};

use super::metric_trait_gate::MetricTraitEnvelope;
use crate::q42::species_corpus::SpeciesCorpus;

pub const P_BODY_LENGTH: &str = "https://ns.webizen.org/eco/traits#bodyLength";
pub const P_BODY_WIDTH: &str = "https://ns.webizen.org/eco/traits#bodyWidth";
pub const P_BODY_HEIGHT: &str = "https://ns.webizen.org/eco/traits#bodyHeight";
pub const P_BODY_MASS: &str = "https://ns.webizen.org/eco/traits#bodyMass";

pub const UNIT_METRE: &str = "http://qudt.org/vocab/unit/M";
pub const UNIT_KILOGRAM: &str = "http://qudt.org/vocab/unit/KiloGM";

/// Why a bound is held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundBasis {
    /// Measured, described or derived assertions agree.
    Evidence,
    /// A recorded model assumption, or the reference example when the corpus is silent.
    ModelAssumption,
}

/// One envelope axis and the assertions it cites.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundCitation {
    pub min: f32,
    pub max: f32,
    pub basis: BoundBasis,
    pub assertion_ids: Vec<String>,
}

/// A dimensional admission envelope whose axes cite corpus assertions.
#[derive(Clone, Debug, PartialEq)]
pub struct CitedMetricEnvelope {
    pub envelope: MetricTraitEnvelope,
    pub length: BoundCitation,
    pub width: BoundCitation,
    pub height: BoundCitation,
    pub mass: BoundCitation,
}

/// Reference example for a known species id (`macropus-giganteus`, spaces and underscores accepted).
pub fn reference_envelope_for_species(species_id: &str) -> Option<&'static MetricTraitEnvelope> {
    use super::metric_trait_gate::*;
    let key: String = species_id
        .chars()
        .map(|c| {
            if c == ' ' || c == '_' {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    Some(match key.as_str() {
        "macropus-giganteus" | "eastern-grey-kangaroo" => &ENVELOPE_MACROPUS_GIGANTEUS,
        "vombatus-ursinus" | "common-wombat" => &ENVELOPE_VOMBATUS_URSINUS,
        "ornithorhynchus-anatinus" | "platypus" => &ENVELOPE_ORNITHORHYNCHUS_ANATINUS,
        "aquila-audax" | "wedge-tailed-eagle" => &ENVELOPE_AQUILA_AUDAX,
        "corymbia-aparrerinja" | "ghost-gum" => &ENVELOPE_CORYMBIA_APARRERINJA,
        "field-utility-4wd" => &ENVELOPE_FIELD_UTILITY_4WD,
        _ => return None,
    })
}

/// Builds an envelope from `corpus`, using `reference` wherever the corpus has no range.
///
/// Disagreeing evidence is an error. The reference values are not mixed into an evidenced span.
pub fn cited_envelope_from_corpus(
    corpus: &SpeciesCorpus,
    reference: &MetricTraitEnvelope,
) -> Result<CitedMetricEnvelope, String> {
    let length = cite_bound(corpus, P_BODY_LENGTH, UNIT_METRE, reference.min_length_m, reference.max_length_m)?;
    let width = cite_bound(corpus, P_BODY_WIDTH, UNIT_METRE, reference.min_width_m, reference.max_width_m)?;
    let height = cite_bound(corpus, P_BODY_HEIGHT, UNIT_METRE, reference.min_height_m, reference.max_height_m)?;
    let mass = cite_bound(corpus, P_BODY_MASS, UNIT_KILOGRAM, reference.min_mass_kg, reference.max_mass_kg)?;
    Ok(CitedMetricEnvelope {
        envelope: MetricTraitEnvelope {
            entity_class_id: reference.entity_class_id,
            min_length_m: length.min,
            max_length_m: length.max,
            min_width_m: width.min,
            max_width_m: width.max,
            min_height_m: height.min,
            max_height_m: height.max,
            min_mass_kg: mass.min,
            max_mass_kg: mass.max,
        },
        length,
        width,
        height,
        mass,
    })
}

fn cite_bound(
    corpus: &SpeciesCorpus,
    property: &str,
    unit: &str,
    ref_min: f32,
    ref_max: f32,
) -> Result<BoundCitation, String> {
    if corpus.conflicts().iter().any(|c| c.property_iri == property) {
        return Err(format!("species corpus conflicts on '{property}'"));
    }
    let mut evidence: Vec<(f64, f64, String)> = Vec::new();
    let mut assumptions: Vec<(f64, f64, String)> = Vec::new();
    for a in corpus.assertions_for(property, None) {
        let Some((min, max, u)) = range_of(&a.evidence.value) else {
            continue;
        };
        if u != unit {
            return Err(format!(
                "assertion '{}' uses unit '{u}', expected '{unit}'",
                a.assertion.id
            ));
        }
        let slot = match a.evidence.basis {
            ClaimBasis::Measured | ClaimBasis::Described | ClaimBasis::Derived => &mut evidence,
            ClaimBasis::ModelAssumption => &mut assumptions,
            ClaimBasis::ArtisticChoice | ClaimBasis::Unknown => continue,
        };
        slot.push((min, max, a.assertion.id.clone()));
    }
    let chosen = if !evidence.is_empty() {
        Some((BoundBasis::Evidence, evidence))
    } else if !assumptions.is_empty() {
        Some((BoundBasis::ModelAssumption, assumptions))
    } else {
        None
    };
    let Some((basis, spans)) = chosen else {
        return Ok(BoundCitation {
            min: ref_min,
            max: ref_max,
            basis: BoundBasis::ModelAssumption,
            assertion_ids: Vec::new(),
        });
    };
    let (min, max) = union_overlapping(&spans, property)?;
    Ok(BoundCitation {
        min: min as f32,
        max: max as f32,
        basis,
        assertion_ids: spans.into_iter().map(|(_, _, id)| id).collect(),
    })
}

fn range_of(value: &Option<EvidenceValue>) -> Option<(f64, f64, &str)> {
    match value {
        Some(EvidenceValue::Range { min, max, unit_iri }) => Some((*min, *max, unit_iri.as_str())),
        Some(EvidenceValue::Quantity { value, unit_iri }) => Some((*value, *value, unit_iri.as_str())),
        _ => None,
    }
}

fn union_overlapping(spans: &[(f64, f64, String)], property: &str) -> Result<(f64, f64), String> {
    let (mut lo, mut hi, _) = spans[0];
    for (a0, a1, _) in spans {
        if *a0 > hi || lo > *a1 {
            return Err(format!("inconsistent ranges for '{property}' within one source"));
        }
        lo = lo.min(*a0);
        hi = hi.max(*a1);
    }
    if !lo.is_finite() || !hi.is_finite() || lo > hi {
        return Err(format!("range for '{property}' is not finite"));
    }
    Ok((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::species_corpus::{KnowledgeGroup, SpeciesCorpusBuilder};
    use crate::q42_volume::{IngestAccessPolicy, IngestProvenanceRecord};
    use qualia_cooperative_core::contextual_assertion::{
        AssertionStatus, ContextualAssertion, StatementType, VerificationStatus,
    };
    use qualia_cooperative_core::contextual_evidence::{
        BiologicalScope, ClaimBasis, EvidenceDetail, EvidenceValue, EvidencedAssertion,
    };
    use qualia_cooperative_core::record::SensitivityClass;

    fn measured(id: &str, property: &str, min: f64, max: f64, unit: &str) -> EvidencedAssertion {
        EvidencedAssertion {
            assertion: ContextualAssertion {
                id: id.into(),
                subject_uri: "urn:taxon:macropus".into(),
                statement_type: StatementType::Observation,
                assertion_status: AssertionStatus::Active,
                verification_status: VerificationStatus::Proposed,
                claim_summary: id.into(),
                author_did: "did:example:curator".into(),
                root_source_id: "src-1".into(),
                derived_from_assertion_ids: Vec::new(),
                scope_domain: "species:macropus-giganteus".into(),
                sensitivity: SensitivityClass::Public,
                asserted_at_unix: 1,
                expires_at_unix: None,
                superseded_by_id: None,
                adjudicated_by_human_did: None,
            },
            evidence: EvidenceDetail {
                property_iri: property.into(),
                basis: ClaimBasis::Measured,
                value: Some(EvidenceValue::Range {
                    min,
                    max,
                    unit_iri: unit.into(),
                }),
                uncertainty: None,
                method_iri: Some("urn:example:method".into()),
                observed_at: None,
                valid_during: None,
                scope: BiologicalScope::default(),
                source_record_id: None,
                media_region: None,
            },
        }
    }

    #[test]
    fn evidenced_length_is_cited_and_missing_axes_stay_reference_assumptions() {
        let ingest = IngestProvenanceRecord::new(
            IngestAccessPolicy::PublicNotForRedistribution,
            "ab".repeat(32),
            "1",
            "c".repeat(64),
            "b".repeat(64),
        );
        let corpus = SpeciesCorpusBuilder::new(
            "urn:corpus:macropus",
            "urn:taxon:macropus",
            "macropus-giganteus",
            "r1",
        )
        .source_from_ingest("src-1", "guide", &ingest)
        .assertion(measured("ea-len", P_BODY_LENGTH, 1.0, 1.6, UNIT_METRE))
        .gap_if_missing(
            "gap-mass",
            KnowledgeGroup::MorphologyAnatomy,
            P_BODY_MASS,
            None,
            "no mass assertion",
        )
        .build()
        .unwrap();
        let reference = reference_envelope_for_species("Macropus giganteus").unwrap();
        let cited = cited_envelope_from_corpus(&corpus, reference).unwrap();
        assert_eq!(cited.length.basis, BoundBasis::Evidence);
        assert_eq!(cited.length.assertion_ids, ["ea-len"]);
        assert!((cited.envelope.min_length_m - 1.0).abs() < 1e-5);
        assert_eq!(cited.mass.basis, BoundBasis::ModelAssumption);
        assert!(cited.mass.assertion_ids.is_empty());
        assert_eq!(cited.envelope.min_mass_kg, reference.min_mass_kg);
    }
}
