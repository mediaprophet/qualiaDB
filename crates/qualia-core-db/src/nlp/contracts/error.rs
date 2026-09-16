//! Typed contract failures. None of these convert to empty success.

use core::fmt;

/// Typed processor outcome. Empty success is [`AnalysisState::Complete`] with
/// zero counts; it is not an error and is not an unavailable processor.
///
/// v1 of this module constructs only [`AnalysisState::Complete`]. Other
/// variants are reserved for later processors (partial coverage, cancelled,
/// …) and must not be treated as currently emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AnalysisState {
    Complete = 0,
    Partial = 1,
    Unsupported = 2,
    NotRequested = 3,
    BudgetExceeded = 4,
    Cancelled = 5,
    InvalidInput = 6,
}

/// Which caller slice ran out of room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BufferChannel {
    Tokens = 0,
    Sentences = 1,
    Hits = 2,
    Norms = 3,
    Plans = 4,
}

/// Failures for the buffered contract API. Never converted into empty success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlpContractError {
    SourceTooLarge { bytes: usize, max: usize },
    OutputBufferFull {
        needed: usize,
        capacity: usize,
        channel: BufferChannel,
    },
    SpanInvalid { start: u32, end: u32 },
}

impl fmt::Display for NlpContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SourceTooLarge { bytes, max } => {
                write!(f, "NLP source is {bytes} bytes; max is {max}")
            }
            Self::OutputBufferFull {
                needed,
                capacity,
                channel,
            } => write!(
                f,
                "NLP {channel:?} buffer needs {needed} slots; had {capacity}"
            ),
            Self::SpanInvalid { start, end } => {
                write!(f, "NLP span {start}..{end} is not a UTF-8 source slice")
            }
        }
    }
}

impl std::error::Error for NlpContractError {}
