//! Validation / mapping report types for ChEBI → Quin (AST-04).
//!
//! Cold construction: heap `String` / `Vec` fields are intentional. The hot
//! projection path writes only into the caller-supplied `&mut [NQuin]` buffer.

use std::fmt;

/// Caller ceilings for one mapping pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapBudgets {
    /// Maximum Quins this pass may write (also limited by `out.len()`).
    pub max_quins: usize,
    /// Maximum conflicts retained in the report before fail-closed.
    pub max_conflicts: usize,
}

impl MapBudgets {
    /// Reject zero ceilings (fail closed before any work).
    pub fn validate(self) -> Result<Self, MapError> {
        if self.max_quins == 0 || self.max_conflicts == 0 {
            return Err(MapError::InvalidBudgets);
        }
        Ok(self)
    }
}

/// One identifier collision or mapping refusal inside a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapConflict {
    /// Accession string involved (may be empty when only numeric id collided).
    pub accession: String,
    /// Stable reason code (no formatted heap strings).
    pub reason: &'static str,
}

/// Bounded lexicon / hash note for a surface string projected as a 60-bit object.
///
/// Names and accessions are stored as `q_hash` / 60-bit tokens in Quin fields;
/// this cold side-table preserves the original surface for evidence display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntry {
    /// 60-bit token written into a Quin field.
    pub hash: u64,
    /// Original surface text.
    pub surface: String,
    /// Role: `"accession"`, `"name"`, `"release"`, or `"parent"`.
    pub kind: &'static str,
}

/// Result of a successful (possibly partially conflicted) mapping pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapReport {
    pub quins_written: usize,
    pub records_mapped: usize,
    pub conflicts: Vec<MapConflict>,
    pub lexicon: Vec<LexiconEntry>,
    pub release_label: String,
}

/// Fail-closed errors for ChEBI Quin mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// Budget fields must be strictly positive.
    InvalidBudgets,
    /// Caller buffer / `max_quins` cannot hold the next record's Quins.
    OutputFull {
        written: usize,
        needed: usize,
        capacity: usize,
    },
    /// Conflict list would exceed [`MapBudgets::max_conflicts`].
    ConflictBudgetExceeded { used: usize, max: usize },
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBudgets => write!(f, "map budgets must be positive"),
            Self::OutputFull {
                written,
                needed,
                capacity,
            } => write!(
                f,
                "quin output full (written={written}, needed={needed}, capacity={capacity})"
            ),
            Self::ConflictBudgetExceeded { used, max } => {
                write!(f, "conflict budget exceeded (used={used}, max={max})")
            }
        }
    }
}

impl std::error::Error for MapError {}
