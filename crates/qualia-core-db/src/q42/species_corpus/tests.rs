use qualia_cooperative_core::contextual_assertion::{
    AssertionStatus, ContextualAssertion, StatementType, VerificationStatus,
};
use qualia_cooperative_core::contextual_evidence::{
    BiologicalScope, ClaimBasis, EvidenceDetail, EvidenceValue, EvidencedAssertion,
};
use qualia_cooperative_core::record::SensitivityClass;

use super::model::{KnowledgeGroup, SPECIES_CORPUS_CONTEXT, SPECIES_CORPUS_FORMAT};
use super::SpeciesCorpusBuilder;
use crate::q42_volume::{IngestAccessPolicy, IngestProvenanceRecord};

const TAXON: &str = "https://biodiversity.org.au/afd/taxa/Wallabia_bicolor";
const LENGTH: &str = "https://ns.webizen.org/eco/traits#bodyLength";
const UNIT_M: &str = "http://qudt.org/vocab/unit/M";

fn sha() -> String {
    "ab".repeat(32)
}

fn assertion(id: &str, root: &str, statement: StatementType) -> ContextualAssertion {
    ContextualAssertion {
        id: id.into(),
        subject_uri: TAXON.into(),
        statement_type: statement,
        assertion_status: AssertionStatus::Active,
        verification_status: VerificationStatus::Proposed,
        claim_summary: id.into(),
        author_did: "did:example:curator".into(),
        root_source_id: root.into(),
        derived_from_assertion_ids: Vec::new(),
        scope_domain: "species:wallabia-bicolor".into(),
        sensitivity: SensitivityClass::Public,
        asserted_at_unix: 1_700_000_000,
        expires_at_unix: None,
        superseded_by_id: None,
        adjudicated_by_human_did: None,
    }
}

fn measured(id: &str, root: &str, property: &str, min: f64, max: f64) -> EvidencedAssertion {
    EvidencedAssertion {
        assertion: assertion(id, root, StatementType::Observation),
        evidence: EvidenceDetail {
            property_iri: property.into(),
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Range {
                min,
                max,
                unit_iri: UNIT_M.into(),
            }),
            uncertainty: None,
            method_iri: Some("urn:example:method:caliper".into()),
            observed_at: None,
            valid_during: None,
            scope: BiologicalScope {
                taxon_concept_iri: Some(TAXON.into()),
                ..Default::default()
            },
            source_record_id: None,
            media_region: None,
        },
    }
}

fn builder() -> SpeciesCorpusBuilder {
    let ingest = IngestProvenanceRecord::new(
        IngestAccessPolicy::PublicNotForRedistribution,
        sha(),
        "map-1",
        "c".repeat(64),
        "b".repeat(64),
    );
    SpeciesCorpusBuilder::new("urn:corpus:wallabia-bicolor", TAXON, "wallabia-bicolor", "r1")
        .source_from_ingest("src-1", "Field guide", &ingest)
}

#[test]
fn builder_records_a_gap_and_keeps_a_cross_source_conflict() {
    let other = IngestProvenanceRecord::new(
        IngestAccessPolicy::PublicNotForRedistribution,
        "cd".repeat(32),
        "map-1",
        "c".repeat(64),
        "b".repeat(64),
    );
    let corpus = builder()
        .source_from_ingest("src-2", "Second treatment", &other)
        .assertion(measured("ea-len-1", "src-1", LENGTH, 0.60, 0.85))
        .assertion(measured("ea-len-2", "src-2", LENGTH, 1.20, 1.40))
        .gap_if_missing(
            "gap-mass",
            KnowledgeGroup::MorphologyAnatomy,
            "https://ns.webizen.org/eco/traits#bodyMass",
            None,
            "no mass assertion in this revision",
        )
        .gap_if_missing("gap-len", KnowledgeGroup::MorphologyAnatomy, LENGTH, None, "missing")
        .build()
        .unwrap();

    assert_eq!(corpus.gaps.len(), 1);
    assert_eq!(corpus.gaps[0].id, "gap-mass");
    let conflicts = corpus.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].property_iri, LENGTH);
    assert_eq!(conflicts[0].assertion_ids, ["ea-len-1", "ea-len-2"]);
}

#[test]
fn overlapping_ranges_from_two_sources_are_not_a_conflict() {
    let other = IngestProvenanceRecord::new(
        IngestAccessPolicy::ProjectInternal,
        "cd".repeat(32),
        "map-1",
        "c".repeat(64),
        "b".repeat(64),
    );
    let corpus = builder()
        .source_from_ingest("src-2", "Second treatment", &other)
        .assertion(measured("ea-1", "src-1", LENGTH, 0.60, 0.90))
        .assertion(measured("ea-2", "src-2", LENGTH, 0.70, 1.00))
        .build()
        .unwrap();
    assert!(corpus.conflicts().is_empty());
}

#[test]
fn model_assumption_does_not_count_as_disagreeing_evidence() {
    let mut assumption = measured("ea-assume", "", LENGTH, 9.0, 9.5);
    assumption.assertion.statement_type = StatementType::Attestation;
    assumption.assertion.root_source_id.clear();
    assumption.evidence.basis = ClaimBasis::ModelAssumption;
    assumption.evidence.method_iri = None;
    assumption.evidence.scope.taxon_concept_iri = None;
    let corpus = builder()
        .assertion(measured("ea-1", "src-1", LENGTH, 0.60, 0.85))
        .assertion(assumption)
        .build()
        .unwrap();
    assert!(corpus.conflicts().is_empty());
    assert_eq!(corpus.assertions_for(LENGTH, None).count(), 2);
}

#[test]
fn q42_and_cbor_roundtrip_including_a_negative_range() {
    let mut ankle = measured("ea-ankle", "src-1", "q42:rom:left_ankle:flexion:healthy", -0.20, 0.95);
    ankle.evidence.value = Some(EvidenceValue::Range {
        min: -0.20,
        max: 0.95,
        unit_iri: "http://qudt.org/vocab/unit/RAD".into(),
    });
    let corpus = builder().assertion(ankle).build().unwrap();
    let sealed = corpus.seal().unwrap();
    assert_eq!(sealed.q42_sha256.len(), 64);
    assert!(sealed.conflicts.is_empty());

    let from_q42 = super::model::SpeciesCorpus::from_q42_bytes(&sealed.q42).unwrap();
    assert_eq!(from_q42, corpus);
    let from_cbor = super::model::SpeciesCorpus::from_cbor_ld(&sealed.cbor_ld).unwrap();
    assert_eq!(from_cbor.context, SPECIES_CORPUS_CONTEXT);
    assert_eq!(from_cbor.format, SPECIES_CORPUS_FORMAT);
    assert_eq!(from_cbor, corpus);

    let value = serde_json::to_value(&corpus).unwrap();
    assert_eq!(super::model::SpeciesCorpus::from_json_value(value).unwrap(), corpus);
}

#[test]
fn unknown_source_and_foreign_taxon_are_rejected() {
    let bad_source = builder()
        .assertion(measured("ea-1", "missing", LENGTH, 0.6, 0.8))
        .build();
    assert!(bad_source.unwrap_err().contains("unknown source"));

    let mut foreign = measured("ea-1", "src-1", LENGTH, 0.6, 0.8);
    foreign.evidence.scope.taxon_concept_iri = Some("urn:taxon:other".into());
    let bad_taxon = builder().assertion(foreign).build();
    assert!(bad_taxon.unwrap_err().contains("scoped to taxon"));
}
