//! Fail-closed errors for ChEBI in-memory queries (AST-05).

use std::fmt;

/// Typed query failures — empty / ambiguous / limit paths never succeed quietly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// Caller supplied an empty Quin slice where at least one Quin is required.
    EmptyInput,
    /// Query string was empty or could not be normalized to an accession.
    EmptyQuery,
    /// No compound matched the query in the supplied slice.
    NotFound,
    /// More than one distinct compound matched a unique-style resolve.
    Ambiguous { hits: usize },
    /// Result count would exceed the caller limit / output capacity.
    LimitExceeded { limit: usize, needed: usize },
    /// Output buffer filled before the walk/export completed.
    OutputFull { written: usize, capacity: usize },
    /// [`super::QueryLimits`] fields must be strictly positive.
    InvalidLimits,
    /// Subgraph walk stopped because depth would exceed the ceiling.
    DepthExceeded { depth: usize, max: usize },
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "empty quin input"),
            Self::EmptyQuery => write!(f, "empty or unnormalizable chemical query"),
            Self::NotFound => write!(f, "chemical not found in asset slice"),
            Self::Ambiguous { hits } => write!(f, "ambiguous chemical match (hits={hits})"),
            Self::LimitExceeded { limit, needed } => {
                write!(f, "query limit exceeded (limit={limit}, needed={needed})")
            }
            Self::OutputFull { written, capacity } => {
                write!(f, "output full (written={written}, capacity={capacity})")
            }
            Self::InvalidLimits => write!(f, "query limits must be positive"),
            Self::DepthExceeded { depth, max } => {
                write!(f, "subgraph depth exceeded (depth={depth}, max={max})")
            }
        }
    }
}

impl std::error::Error for QueryError {}
