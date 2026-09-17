//! Normalized ChEBI compound records (AST-03).

/// One accepted ChEBI compound row from `compounds.tsv`.
///
/// Cold-construction type: heap `String` fields are intentional. Hot-path Quin
/// projection belongs to AST-04 and must not allocate URI strings at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChebiRecord {
    /// Numeric ChEBI id (e.g. `15377`).
    pub id: u64,
    /// Upstream status code preserved as text (`C` / `E` / …). Deleted (`D`)
    /// rows never appear here — they are quarantined.
    pub status: String,
    /// Accession string (e.g. `CHEBI:15377`).
    pub accession: String,
    /// Free-text SOURCE column (may be empty).
    pub source: String,
    /// Parent compound id, or `None` when the PARENT_ID cell is empty.
    pub parent_id: Option<u64>,
    /// Compound display name (non-empty for accepted rows).
    pub name: String,
    /// Definition text (may be empty).
    pub definition: String,
    /// 1-based file line number for provenance (header is line 1).
    pub source_line: u64,
}

/// Caller-supplied parse ceilings for one compounds.tsv pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBudgets {
    /// Reject the whole input when `input.len()` exceeds this.
    pub max_bytes: u64,
    /// Combined ceiling for accepted + quarantined rows.
    pub max_records: u64,
    /// Reject when any single line (excluding the trailing newline) exceeds this.
    pub max_line_bytes: u64,
}

impl ParseBudgets {
    /// Validate positivity of all budget fields.
    pub fn validate(self) -> Result<Self, super::error::ParseError> {
        if self.max_bytes == 0 || self.max_records == 0 || self.max_line_bytes == 0 {
            return Err(super::error::ParseError::InvalidBudgets);
        }
        Ok(self)
    }
}

/// Result of a successful (possibly partially quarantined) parse pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport {
    pub accepted: Vec<ChebiRecord>,
    pub quarantined: Vec<super::error::QuarantinedRow>,
    pub bytes_read: u64,
    /// Caller-supplied release attribution copied verbatim into the report.
    pub release_label: String,
}

impl ParseReport {
    /// Total rows that consumed record budget (accepted + quarantined).
    pub fn total_rows(&self) -> u64 {
        (self.accepted.len() + self.quarantined.len()) as u64
    }
}
