//! ChEBI record → Quin mapping (AST-04).
//!
//! Maps accepted [`ChebiRecord`](crate::q42::chebi_parse::ChebiRecord) values
//! from AST-03 into evidence-preserving, parity-valid [`NQuin`](crate::NQuin)
//! slices plus a cold validation / lexicon report.
//!
//! # Mapping (per accepted record)
//!
//! | # | Predicate IRI | Object |
//! |---|---------------|--------|
//! | 1 | `rdf:type` | `chebi:Compound` |
//! | 2 | `chebi:accession` | 60-bit hash of accession (same as subject) |
//! | 3 | `chebi:hasName` | 60-bit hash of name (surface in lexicon) |
//! | 4 | `chebi:hasParent` | 60-bit hash of `CHEBI:{parent_id}` (optional) |
//! | 5 | `chebi:fromRelease` | 60-bit hash of `release_label`; `metadata` low 32 = `source_line` |
//!
//! Subject for every Quin is `q_hash(accession)`. Context is
//! `q_hash(release_label)`. No network; no new Host/Vibe invoke IDs.
//!
//! Hot API: [`map_records_into`] — caller-buffered. Report types may allocate.

mod map;
mod report;

pub use map::{
    map_records_into, quin_parity_valid, quins_for_record, CLASS_CHEBI_COMPOUND, PRED_ACCESSION,
    PRED_FROM_RELEASE, PRED_HAS_NAME, PRED_HAS_PARENT, PRED_RDF_TYPE, QUINS_PER_RECORD_BASE,
};
pub use report::{LexiconEntry, MapBudgets, MapConflict, MapError, MapReport};
