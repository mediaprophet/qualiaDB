//! Typed coreference failures. None of these convert to empty success.

use core::fmt;

/// Recoverable coreference failure. Host adapters map budget/cancel/output
/// variants to `E400` and the rest to `E100`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorefError {
    SourceTooLarge { bytes: usize, max: usize },
    TooManyMentions { count: usize, max: usize },
    MentionTextTooLarge { bytes: usize, max: usize },
    TooManyChains { count: usize, max: usize },
    WorkspaceTooSmall { required: usize, provided: usize },
    NormalizationBudget { used: u64, max: u64 },
    ComparisonBudget { used: u64, max: u64 },
    AntecedentBudget { used: u64, max: u64 },
    OutputBufferFull,
    Cancelled,
    NegativeOffset,
    OffsetOverflow,
    SpanEmpty,
    SpanOutOfRange,
    SpanNotUtf8Boundary,
    TextMismatch,
    UnsortedMentions,
    InvalidKind,
    InvalidInput,
}

impl CorefError {
    /// Budget, cancellation, and output-capacity failures.
    pub fn is_resource_error(self) -> bool {
        matches!(
            self,
            Self::SourceTooLarge { .. }
                | Self::TooManyMentions { .. }
                | Self::MentionTextTooLarge { .. }
                | Self::TooManyChains { .. }
                | Self::WorkspaceTooSmall { .. }
                | Self::NormalizationBudget { .. }
                | Self::ComparisonBudget { .. }
                | Self::AntecedentBudget { .. }
                | Self::OutputBufferFull
                | Self::Cancelled
        )
    }
}

impl fmt::Display for CorefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SourceTooLarge { bytes, max } => {
                write!(f, "coref source is {bytes} bytes; max is {max}")
            }
            Self::TooManyMentions { count, max } => {
                write!(f, "coref received {count} mentions; max is {max}")
            }
            Self::MentionTextTooLarge { bytes, max } => {
                write!(f, "coref mention text totals {bytes} bytes; max is {max}")
            }
            Self::TooManyChains { count, max } => {
                write!(f, "coref would emit {count} chains; max is {max}")
            }
            Self::WorkspaceTooSmall { required, provided } => {
                write!(f, "coref workspace needs {required} bytes; had {provided}")
            }
            Self::NormalizationBudget { used, max } => {
                write!(f, "coref normalization used {used} bytes; max is {max}")
            }
            Self::ComparisonBudget { used, max } => {
                write!(f, "coref comparisons used {used} bytes; max is {max}")
            }
            Self::AntecedentBudget { used, max } => {
                write!(f, "coref antecedent checks {used}; max is {max}")
            }
            Self::OutputBufferFull => f.write_str("coref output buffer is full"),
            Self::Cancelled => f.write_str("coref cancelled"),
            Self::NegativeOffset => f.write_str("coref span offset is negative"),
            Self::OffsetOverflow => f.write_str("coref span offset exceeds u32"),
            Self::SpanEmpty => f.write_str("coref span is empty or reversed"),
            Self::SpanOutOfRange => f.write_str("coref span is outside the source"),
            Self::SpanNotUtf8Boundary => f.write_str("coref span is not on a UTF-8 boundary"),
            Self::TextMismatch => f.write_str("coref mention text does not match the source span"),
            Self::UnsortedMentions => f.write_str("coref mentions are not in document order"),
            Self::InvalidKind => f.write_str("coref mention kind is not supported"),
            Self::InvalidInput => f.write_str("coref input is missing or wrongly typed"),
        }
    }
}

impl std::error::Error for CorefError {}
