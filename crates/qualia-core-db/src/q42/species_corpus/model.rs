//! Species knowledge corpus: sources, evidenced assertions and recorded gaps for one taxon
//! concept at a stated revision. Stored natively as a unified `.q42` volume (see [`super::codec`]);
//! CBOR-LD is the interchange projection.

use std::collections::{BTreeMap, BTreeSet};

use qualia_cooperative_core::contextual_evidence::{ClaimBasis, EvidenceValue, EvidencedAssertion};
use serde::{Deserialize, Serialize};

use crate::q42_volume::IngestProvenanceRecord;

pub const SPECIES_CORPUS_CONTEXT: &str = "https://webizen.org/ns/qualia/species-corpus/v1";
pub const SPECIES_CORPUS_TYPE: &str = "QualiaSpeciesCorpus";
pub const SPECIES_CORPUS_FORMAT: &str = "qualia.q42.species-corpus.v1";
pub const MAX_CORPUS_ASSERTIONS: usize = 1_000_000;

/// One acquired source. Integrity fields mirror [`IngestProvenanceRecord`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusSource {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
    pub access_policy: String,
    pub source_sha256: String,
    pub mapping_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence_iri: Option<String>,
}

impl CorpusSource {
    /// Builds a source entry from an existing ingest provenance record.
    pub fn from_ingest(id: &str, title: &str, record: &IngestProvenanceRecord) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            citation: None,
            retrieved_at: None,
            access_policy: record.access_policy.clone(),
            source_sha256: record.source_sha256.clone(),
            mapping_version: record.mapping_version.clone(),
            licence_iri: None,
        }
    }
}

/// Knowledge group from the species embodiment specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeGroup {
    IdentifiersTaxonomy,
    MorphologyAnatomy,
    DevelopmentReproduction,
    PhysiologyBehaviour,
    Ecology,
    DistributionObservations,
    MolecularCellular,
    ConservationHumanRelations,
    RepresentationsMedia,
    ProvenanceOperation,
}

/// A known absence: the corpus does not hold evidence for this property/scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub id: String,
    pub group: KnowledgeGroup,
    pub property_iri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_iri: Option<String>,
    pub reason: String,
}

/// One active or inactive fact, flattened so callers do not depend on the evidence enum.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusFact<'a> {
    pub assertion_id: &'a str,
    pub property_iri: &'a str,
    pub basis: &'a str,
    pub active: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit_iri: Option<&'a str>,
    pub text: Option<&'a str>,
}

/// Assertions for one property and scope whose values disagree across independent sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusConflict {
    pub property_iri: String,
    pub scope_key: String,
    pub assertion_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeciesCorpus {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    pub type_: String,
    pub format: String,
    pub corpus_iri: String,
    pub taxon_concept_iri: String,
    pub species_id: String,
    pub revision: String,
    #[serde(default)]
    pub sources: Vec<CorpusSource>,
    #[serde(default)]
    pub assertions: Vec<EvidencedAssertion>,
    #[serde(default)]
    pub gaps: Vec<KnowledgeGap>,
}

fn require(v: &str, what: &str) -> Result<(), String> {
    if v.trim().is_empty() {
        Err(format!("species corpus {what} is empty"))
    } else {
        Ok(())
    }
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, what: &str) -> Result<BTreeSet<&'a str>, String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        require(id, &format!("{what} id"))?;
        if !seen.insert(id) {
            return Err(format!("duplicate {what} id '{id}'"));
        }
    }
    Ok(seen)
}

impl SpeciesCorpus {
    pub fn new(corpus_iri: &str, taxon_concept_iri: &str, species_id: &str, revision: &str) -> Self {
        Self {
            context: SPECIES_CORPUS_CONTEXT.to_string(),
            type_: SPECIES_CORPUS_TYPE.to_string(),
            format: SPECIES_CORPUS_FORMAT.to_string(),
            corpus_iri: corpus_iri.to_string(),
            taxon_concept_iri: taxon_concept_iri.to_string(),
            species_id: species_id.to_string(),
            revision: revision.to_string(),
            sources: Vec::new(),
            assertions: Vec::new(),
            gaps: Vec::new(),
        }
    }

    /// Decodes a JSON-LD document of this shape and validates it.
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, String> {
        let corpus: Self =
            serde_json::from_value(value).map_err(|e| format!("species corpus json: {e}"))?;
        corpus.validate()?;
        Ok(corpus)
    }

    /// Checks identity, unique ids, per-assertion evidence, source references and taxon binding.
    pub fn validate(&self) -> Result<(), String> {
        if self.context != SPECIES_CORPUS_CONTEXT || self.format != SPECIES_CORPUS_FORMAT {
            return Err("not a species corpus v1 document".into());
        }
        require(&self.corpus_iri, "corpus_iri")?;
        require(&self.taxon_concept_iri, "taxon_concept_iri")?;
        require(&self.species_id, "species_id")?;
        require(&self.revision, "revision")?;
        if self.assertions.len() > MAX_CORPUS_ASSERTIONS {
            return Err(format!("species corpus exceeds {MAX_CORPUS_ASSERTIONS} assertions"));
        }
        let source_ids = unique(self.sources.iter().map(|s| s.id.as_str()), "source")?;
        unique(self.assertions.iter().map(|a| a.assertion.id.as_str()), "assertion")?;
        unique(self.gaps.iter().map(|g| g.id.as_str()), "gap")?;
        for s in &self.sources {
            require(&s.title, &format!("source '{}' title", s.id))?;
            if s.source_sha256.len() != 64 || !s.source_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(format!("source '{}' source_sha256 must be 64 hex digits", s.id));
            }
        }
        for ea in &self.assertions {
            let id = &ea.assertion.id;
            ea.validate().map_err(|e| format!("assertion '{id}': {e}"))?;
            let root = ea.assertion.root_source_id.as_str();
            if !root.is_empty() && !source_ids.contains(root) {
                return Err(format!("assertion '{id}' cites unknown source '{root}'"));
            }
            if let Some(t) = &ea.evidence.scope.taxon_concept_iri {
                if t != &self.taxon_concept_iri {
                    return Err(format!(
                        "assertion '{id}' is scoped to taxon '{t}', not the corpus taxon"
                    ));
                }
            }
        }
        for g in &self.gaps {
            require(&g.property_iri, &format!("gap '{}' property_iri", g.id))?;
            require(&g.reason, &format!("gap '{}' reason", g.id))?;
        }
        Ok(())
    }

    /// Flattens every assertion into a fact a production reader can use without the evidence crate.
    pub fn facts(&self) -> Vec<CorpusFact<'_>> {
        use qualia_cooperative_core::contextual_assertion::AssertionStatus;
        use qualia_cooperative_core::contextual_evidence::EvidenceValue;
        self.assertions
            .iter()
            .map(|a| {
                let (min, max, unit_iri, text) = match &a.evidence.value {
                    Some(EvidenceValue::Quantity { value, unit_iri }) => {
                        (Some(*value), Some(*value), Some(unit_iri.as_str()), None)
                    }
                    Some(EvidenceValue::Range { min, max, unit_iri }) => {
                        (Some(*min), Some(*max), Some(unit_iri.as_str()), None)
                    }
                    Some(EvidenceValue::Text { text, .. }) => (None, None, None, Some(text.as_str())),
                    Some(EvidenceValue::Iri { iri }) => (None, None, None, Some(iri.as_str())),
                    None => (None, None, None, None),
                };
                CorpusFact {
                    assertion_id: a.assertion.id.as_str(),
                    property_iri: a.evidence.property_iri.as_str(),
                    basis: a.evidence.basis.as_str(),
                    active: a.assertion.assertion_status == AssertionStatus::Active,
                    min,
                    max,
                    unit_iri,
                    text,
                }
            })
            .collect()
    }

    /// Active assertions for `property_iri`, optionally restricted to a part.
    pub fn assertions_for<'a>(
        &'a self,
        property_iri: &'a str,
        part_iri: Option<&'a str>,
    ) -> impl Iterator<Item = &'a EvidencedAssertion> + 'a {
        use qualia_cooperative_core::contextual_assertion::AssertionStatus;
        self.assertions.iter().filter(move |a| {
            a.evidence.property_iri == property_iri
                && a.assertion.assertion_status == AssertionStatus::Active
                && part_iri.map_or(true, |p| a.evidence.scope.part_iri.as_deref() == Some(p))
        })
    }

    /// Finds properties whose evidence-basis values disagree across independent root sources.
    /// Model assumptions, artistic choices and unknowns are not evidence and are excluded.
    pub fn conflicts(&self) -> Vec<CorpusConflict> {
        let mut groups: BTreeMap<(String, String), Vec<&EvidencedAssertion>> = BTreeMap::new();
        for a in &self.assertions {
            if matches!(
                a.evidence.basis,
                ClaimBasis::ModelAssumption | ClaimBasis::ArtisticChoice | ClaimBasis::Unknown
            ) || a.evidence.value.is_none()
            {
                continue;
            }
            let s = &a.evidence.scope;
            let scope_key = format!(
                "{}|{}|{}|{}|{}",
                s.part_iri.as_deref().unwrap_or(""),
                s.life_stage_iri.as_deref().unwrap_or(""),
                s.sex_iri.as_deref().unwrap_or(""),
                s.population_iri.as_deref().unwrap_or(""),
                s.individual_iri.as_deref().unwrap_or("")
            );
            groups
                .entry((a.evidence.property_iri.clone(), scope_key))
                .or_default()
                .push(a);
        }
        let mut out = Vec::new();
        for ((property_iri, scope_key), items) in groups {
            let roots: BTreeSet<&str> =
                items.iter().map(|a| a.assertion.root_source_id.as_str()).collect();
            if roots.len() < 2 {
                continue;
            }
            let first = items[0].evidence.value.as_ref();
            if items.iter().any(|a| !values_compatible(first, a.evidence.value.as_ref())) {
                out.push(CorpusConflict {
                    property_iri,
                    scope_key,
                    assertion_ids: items.iter().map(|a| a.assertion.id.clone()).collect(),
                });
            }
        }
        out
    }
}

/// Values agree when equal, or for numeric values when their ranges overlap in the same unit.
fn values_compatible(a: Option<&EvidenceValue>, b: Option<&EvidenceValue>) -> bool {
    fn span(v: &EvidenceValue) -> Option<(f64, f64, &str)> {
        match v {
            EvidenceValue::Quantity { value, unit_iri } => Some((*value, *value, unit_iri)),
            EvidenceValue::Range { min, max, unit_iri } => Some((*min, *max, unit_iri)),
            _ => None,
        }
    }
    match (a, b) {
        (Some(x), Some(y)) => match (span(x), span(y)) {
            (Some((a0, a1, ua)), Some((b0, b1, ub))) => ua == ub && a0 <= b1 && b0 <= a1,
            _ => x == y,
        },
        _ => true,
    }
}
