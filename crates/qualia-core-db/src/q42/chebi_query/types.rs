//! Result and limit types for ChEBI queries (AST-05).
//!
//! Hot APIs write compact hits into caller buffers. Cold report structs may
//! allocate (`String`) for release labels and licence obligation notes.

use super::error::QueryError;

/// Caller ceilings for multi-hit and subgraph operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLimits {
    /// Maximum hits a resolve / relation / evidence call may return.
    pub max_hits: usize,
    /// Maximum parent/child walk depth for subgraph export (seed = 0).
    pub max_depth: usize,
    /// Maximum Quins subgraph export may write (also limited by `out.len()`).
    pub max_export_quins: usize,
}

impl QueryLimits {
    /// Reject zero ceilings (fail closed before any work).
    pub fn validate(self) -> Result<Self, QueryError> {
        if self.max_hits == 0 || self.max_depth == 0 || self.max_export_quins == 0 {
            return Err(QueryError::InvalidLimits);
        }
        Ok(self)
    }
}

/// Evidence certainty attached to every hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Uncertainty {
    /// All expected fields present for this hit.
    Known = 0,
    /// Compound identity resolved but one or more evidence fields missing.
    Partial = 1,
    /// Identity or provenance could not be established from the slice.
    Unknown = 2,
}

/// One bounded chemical resolve / index hit (compact + cold licence note).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChemicalHit {
    /// `q_hash(accession)` — subject of mapped Quins.
    pub subject_hash: u64,
    /// `q_hash(name)` from `chebi:hasName`, or `0` when absent.
    pub name_hash: u64,
    /// Parent subject hash from `chebi:hasParent`, when present.
    pub parent_hash: Option<u64>,
    /// Release / asset context hash (`q_hash(release_label)`).
    pub release_hash: u64,
    /// Provenance line from `chebi:fromRelease` metadata (low 32 bits).
    pub source_line: u32,
    pub uncertainty: Uncertainty,
    /// Surface accession when known (`CHEBI:{id}`); empty if only hash known.
    pub accession: String,
    /// Licence obligation note (caller parameter or catalogue stub).
    pub licence_note: String,
}

/// Parent / child edge discovered from `chebi:hasParent` Quins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationHit {
    pub child_hash: u64,
    pub parent_hash: u64,
    pub release_hash: u64,
    /// Child's `fromRelease` source_line when available; `0` + Partial otherwise.
    pub source_line: u32,
    pub uncertainty: Uncertainty,
    pub child_accession: String,
    pub parent_accession: String,
    pub licence_note: String,
}

/// Provenance row from `chebi:fromRelease`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceHit {
    pub subject_hash: u64,
    pub release_hash: u64,
    pub source_line: u32,
    pub uncertainty: Uncertainty,
    pub accession: String,
    pub licence_note: String,
}

/// Cold summary of one release / imported asset slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDescription {
    pub release_label: String,
    pub release_hash: u64,
    /// Distinct compound subjects (have `chebi:accession`) in this release context.
    pub record_count: usize,
    /// Quin count in this release context.
    pub quin_count: usize,
    /// Licence obligation stub (caller or catalogue).
    pub licence_note: String,
    /// Always true when `licence_note` is non-empty after resolution.
    pub licence_obligation_present: bool,
}
