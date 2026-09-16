//! Shared NLP host resource budgets.
//!
//! Transport cap ≠ processing guarantee. [`MAX_SOURCE_BYTES`] is an ingress
//! bound, equal to [`crate::nlp::coref::DEFAULT_MAX_SOURCE_BYTES`] — not a
//! second 256 KiB and not a latency, quality, or expansion claim. Accepting a
//! document at this cap does not mean tokenize, gazetteer, or frame work stays
//! within an SLA, nor that intermediates stay under the Sentinel after
//! construction.
//!
//! The 42 MiB [`SENTINEL_BYTES`] ceiling still applies. These host adapters are
//! **cold** (they may allocate). They must still refuse oversize input so
//! borrowed source plus adapter workspace cannot walk past that envelope.

use crate::nlp::coref::DEFAULT_MAX_SOURCE_BYTES;
use core::fmt;

/// 42 MiB Sentinel ceiling. Re-exported from coreference limits so NLP hosts
/// do not invent a second constant.
pub use crate::nlp::coref::SENTINEL_BYTES;

/// Shared source cap. Equal to [`DEFAULT_MAX_SOURCE_BYTES`].
pub const MAX_SOURCE_BYTES: usize = DEFAULT_MAX_SOURCE_BYTES;

/// GraphRAG query-string cap (transport-sized, not a retrieval-quality claim).
pub const MAX_GRAPHRAG_QUERY_BYTES: usize = 64 * 1024;

/// GraphRAG triple-list cap, checked before index construction.
pub const MAX_TRIPLES: usize = 4096;

/// FST lookup-word cap, checked before dictionary construction.
pub const MAX_FST_WORD_BYTES: usize = 4 * 1024;

/// FST dictionary entry-list cap, checked before host construction.
pub const MAX_FST_ENTRIES: usize = 4096;

/// Resource-budget failure. Host adapters map every variant to `E400`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlpBudgetError {
    SourceTooLarge { bytes: usize, max: usize },
    QueryTooLarge { bytes: usize, max: usize },
    TooManyTriples { count: usize, max: usize },
    TooManyFstEntries { count: usize, max: usize },
    FstWordTooLarge { bytes: usize, max: usize },
}

impl NlpBudgetError {
    /// All budget errors are resource failures (host `E400`).
    pub fn is_resource_error(self) -> bool {
        true
    }
}

impl fmt::Display for NlpBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SourceTooLarge { bytes, max } => {
                write!(f, "nlp source is {bytes} bytes; max is {max}")
            }
            Self::QueryTooLarge { bytes, max } => {
                write!(f, "nlp GraphRAG query is {bytes} bytes; max is {max}")
            }
            Self::TooManyTriples { count, max } => {
                write!(f, "nlp GraphRAG received {count} triples; max is {max}")
            }
            Self::TooManyFstEntries { count, max } => {
                write!(f, "nlp FST received {count} entries; max is {max}")
            }
            Self::FstWordTooLarge { bytes, max } => {
                write!(f, "nlp FST word is {bytes} bytes; max is {max}")
            }
        }
    }
}

impl std::error::Error for NlpBudgetError {}

/// Reject an oversize source before any clone or kernel work.
pub fn reject_source(len: usize) -> Result<(), NlpBudgetError> {
    if len > MAX_SOURCE_BYTES {
        Err(NlpBudgetError::SourceTooLarge {
            bytes: len,
            max: MAX_SOURCE_BYTES,
        })
    } else {
        Ok(())
    }
}

/// Reject an oversize GraphRAG query string before index construction.
pub fn reject_query(len: usize) -> Result<(), NlpBudgetError> {
    if len > MAX_GRAPHRAG_QUERY_BYTES {
        Err(NlpBudgetError::QueryTooLarge {
            bytes: len,
            max: MAX_GRAPHRAG_QUERY_BYTES,
        })
    } else {
        Ok(())
    }
}

/// Reject an oversize triple list before cloning or indexing.
pub fn reject_triples(count: usize) -> Result<(), NlpBudgetError> {
    if count > MAX_TRIPLES {
        Err(NlpBudgetError::TooManyTriples {
            count,
            max: MAX_TRIPLES,
        })
    } else {
        Ok(())
    }
}

/// Reject an oversize FST lookup word before dictionary construction.
pub fn reject_fst_word(len: usize) -> Result<(), NlpBudgetError> {
    if len > MAX_FST_WORD_BYTES {
        Err(NlpBudgetError::FstWordTooLarge {
            bytes: len,
            max: MAX_FST_WORD_BYTES,
        })
    } else {
        Ok(())
    }
}

/// Reject an oversize FST entries list before cloning or dictionary build.
pub fn reject_fst_entries(count: usize) -> Result<(), NlpBudgetError> {
    if count > MAX_FST_ENTRIES {
        Err(NlpBudgetError::TooManyFstEntries {
            count,
            max: MAX_FST_ENTRIES,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::coref::CorefLimits;

    #[test]
    fn max_source_bytes_is_the_coref_default() {
        assert_eq!(MAX_SOURCE_BYTES, DEFAULT_MAX_SOURCE_BYTES);
        assert_eq!(MAX_SOURCE_BYTES, CorefLimits::DEFAULT.max_source_bytes);
    }

    #[test]
    fn reject_source_accepts_the_cap() {
        assert!(reject_source(0).is_ok());
        assert!(reject_source(MAX_SOURCE_BYTES).is_ok());
    }

    #[test]
    fn reject_source_rejects_oversize() {
        let err = reject_source(MAX_SOURCE_BYTES + 1).unwrap_err();
        assert_eq!(
            err,
            NlpBudgetError::SourceTooLarge {
                bytes: MAX_SOURCE_BYTES + 1,
                max: MAX_SOURCE_BYTES,
            }
        );
        assert!(err.is_resource_error());
        assert!(err.to_string().contains(&MAX_SOURCE_BYTES.to_string()));
    }

    #[test]
    fn reject_triples_rejects_oversize() {
        assert!(reject_triples(MAX_TRIPLES).is_ok());
        let err = reject_triples(MAX_TRIPLES + 1).unwrap_err();
        assert_eq!(
            err,
            NlpBudgetError::TooManyTriples {
                count: MAX_TRIPLES + 1,
                max: MAX_TRIPLES,
            }
        );
        assert!(err.is_resource_error());
    }

    #[test]
    fn reject_query_rejects_oversize() {
        assert!(reject_query(MAX_GRAPHRAG_QUERY_BYTES).is_ok());
        let err = reject_query(MAX_GRAPHRAG_QUERY_BYTES + 1).unwrap_err();
        assert_eq!(
            err,
            NlpBudgetError::QueryTooLarge {
                bytes: MAX_GRAPHRAG_QUERY_BYTES + 1,
                max: MAX_GRAPHRAG_QUERY_BYTES,
            }
        );
    }

    #[test]
    fn reject_fst_entries_rejects_oversize() {
        assert!(reject_fst_entries(MAX_FST_ENTRIES).is_ok());
        assert!(reject_fst_entries(MAX_FST_ENTRIES + 1).is_err());
    }

    #[test]
    fn sentinel_is_forty_two_mib() {
        assert_eq!(SENTINEL_BYTES, 42 * 1024 * 1024);
        assert!(MAX_SOURCE_BYTES < SENTINEL_BYTES);
        assert!(MAX_GRAPHRAG_QUERY_BYTES < SENTINEL_BYTES);
    }
}
