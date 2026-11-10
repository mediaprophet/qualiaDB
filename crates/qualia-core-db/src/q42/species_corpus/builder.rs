//! Assembles one species `.q42` from ingest provenance and evidenced assertions.
//!
//! Conflicts are not resolved here. Every assertion is kept, and [`SpeciesCorpus::conflicts`]
//! lists properties whose evidence disagrees across independent sources. A caller that names
//! an expected property with no assertion records that absence as a gap.

use qualia_cooperative_core::contextual_evidence::EvidencedAssertion;

use super::model::{
    CorpusSource, KnowledgeGap, KnowledgeGroup, SpeciesCorpus, MAX_CORPUS_ASSERTIONS,
};
use crate::q42_volume::IngestProvenanceRecord;

/// Collects sources, assertions and gaps, then validates them as one corpus.
#[derive(Debug, Clone)]
pub struct SpeciesCorpusBuilder {
    corpus: SpeciesCorpus,
}

impl SpeciesCorpusBuilder {
    pub fn new(corpus_iri: &str, taxon_concept_iri: &str, species_id: &str, revision: &str) -> Self {
        Self {
            corpus: SpeciesCorpus::new(corpus_iri, taxon_concept_iri, species_id, revision),
        }
    }

    pub fn source(mut self, source: CorpusSource) -> Self {
        self.corpus.sources.push(source);
        self
    }

    /// Adds a source whose integrity fields come from an ingest provenance record.
    pub fn source_from_ingest(
        self,
        id: &str,
        title: &str,
        record: &IngestProvenanceRecord,
    ) -> Self {
        self.source(CorpusSource::from_ingest(id, title, record))
    }

    pub fn assertion(mut self, assertion: EvidencedAssertion) -> Self {
        self.corpus.assertions.push(assertion);
        self
    }

    pub fn gap(mut self, gap: KnowledgeGap) -> Self {
        self.corpus.gaps.push(gap);
        self
    }

    /// Records a gap when no assertion covers `property_iri` (and `part_iri`, when set).
    pub fn gap_if_missing(
        self,
        id: &str,
        group: KnowledgeGroup,
        property_iri: &str,
        part_iri: Option<&str>,
        reason: &str,
    ) -> Self {
        let covered = self.corpus.assertions.iter().any(|a| {
            a.evidence.property_iri == property_iri
                && part_iri.map_or(true, |p| a.evidence.scope.part_iri.as_deref() == Some(p))
        });
        if covered {
            self
        } else {
            self.gap(KnowledgeGap {
                id: id.to_string(),
                group,
                property_iri: property_iri.to_string(),
                part_iri: part_iri.map(str::to_string),
                reason: reason.to_string(),
            })
        }
    }

    /// Validates the collected corpus. Disagreement between sources is kept, not rejected.
    pub fn build(self) -> Result<SpeciesCorpus, String> {
        if self.corpus.assertions.len() > MAX_CORPUS_ASSERTIONS {
            return Err(format!("species corpus exceeds {MAX_CORPUS_ASSERTIONS} assertions"));
        }
        self.corpus.validate()?;
        Ok(self.corpus)
    }
}
