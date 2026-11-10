//! Species knowledge corpus (Ecopunk G72).
//!
//! A species `.q42` is a unified Q42 volume: one [`IngestProvenanceRecord`]-backed source entry
//! per acquired source, and one [`EvidencedAssertion`] per fact. CBOR-LD is the interchange
//! projection. Conflicts stay as separate assertions. Gaps are explicit absences.
//!
//! VibeScript calls this through `qualia.species.assemble_corpus`. Hypermedia production reads
//! the sealed corpus when it turns evidence into sizes, joint ranges and rules.

mod builder;
mod codec;
mod model;

#[cfg(test)]
mod tests;

pub use builder::SpeciesCorpusBuilder;
pub use model::{
    CorpusConflict, CorpusFact, CorpusSource, KnowledgeGap, KnowledgeGroup, SpeciesCorpus,
    MAX_CORPUS_ASSERTIONS, SPECIES_CORPUS_CONTEXT, SPECIES_CORPUS_FORMAT, SPECIES_CORPUS_TYPE,
};

#[cfg(not(target_arch = "wasm32"))]
pub use codec::SealedSpeciesCorpus;
pub use codec::volume_sha256;
