//! ChEBI chemical knowledge queries over an imported Quin slice (AST-05).
//!
//! Bounded, in-memory APIs over caller-supplied `&[NQuin]` (and optional
//! [`ChebiRecord`](crate::q42::chebi_parse::ChebiRecord) index). Encoding must
//! match [`crate::q42::chebi_map`] (subjects = `q_hash(accession)`, predicates
//! `chebi:accession` / `hasName` / `hasParent` / `fromRelease`).
//!
//! No network. No Host/Vibe invoke IDs. Empty / ambiguous / limit paths fail
//! closed with [`QueryError`].

mod access;
mod error;
mod export;
mod query;
mod scan;
mod types;

#[cfg(test)]
mod tests;

pub use access::{format_chebi_accession, normalize_accession_query};
pub use error::QueryError;
pub use export::export_subgraph_into;
pub use query::{
    describe_release, lookup_children_into, lookup_evidence_into, lookup_parents_into,
    resolve_chemical_into,
};
pub use types::{
    ChemicalHit, EvidenceHit, QueryLimits, RelationHit, ReleaseDescription, Uncertainty,
};
