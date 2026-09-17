//! Copy view records. Surfaces are [`DocSpan`] slices of the original source.
//!
//! v1 is in-process Rust (interned `&'static str`, native `usize`). It is not a
//! C FFI layout. Cross-language ABI notes belong to NLP-900.

use super::error::NlpContractError;
use crate::nlp::span::DocSpan;
use crate::nlp::tokenize::TokenKind;

/// Token with no containing sentence.
pub const SENTENCE_ID_NONE: u32 = u32::MAX;
/// Token with no overlapping normalized value.
pub const NORM_INDEX_NONE: u32 = u32::MAX;

/// Token borrowed from the original source. In-process Rust; not a C ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenView {
    pub span: DocSpan,
    pub kind: TokenKind,
    pub sentence_id: u32,
    pub norm_index: u32,
}

impl TokenView {
    pub const EMPTY: Self = Self {
        span: DocSpan {
            start_utf8: 0,
            end_utf8: 0,
        },
        kind: TokenKind::Other,
        sentence_id: SENTENCE_ID_NONE,
        norm_index: NORM_INDEX_NONE,
    };

    pub fn text(self, source: &str) -> Result<&str, NlpContractError> {
        slice_source(self.span, source)
    }
}

/// Sentence borrowed from the original source. In-process Rust; not a C ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceView {
    pub span: DocSpan,
}

impl SentenceView {
    pub const EMPTY: Self = Self {
        span: DocSpan {
            start_utf8: 0,
            end_utf8: 0,
        },
    };

    pub fn text(self, source: &str) -> Result<&str, NlpContractError> {
        slice_source(self.span, source)
    }
}

/// Gazetteer hit; IRI and surface are interned lexicon pointers (fat pointers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitView {
    pub span: DocSpan,
    pub iri: &'static str,
    pub surface: &'static str,
}

impl HitView {
    pub const EMPTY: Self = Self {
        span: DocSpan {
            start_utf8: 0,
            end_utf8: 0,
        },
        iri: "",
        surface: "",
    };

    pub fn text(self, source: &str) -> Result<&str, NlpContractError> {
        slice_source(self.span, source)
    }
}

/// Date / number view. Values are fixed-width; units are interned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NormView {
    DateIso {
        span: DocSpan,
        yyyy_mm_dd: [u8; 10],
    },
    Number {
        span: DocSpan,
        value: f64,
        unit: Option<&'static str>,
    },
}

impl NormView {
    pub const EMPTY: Self = Self::DateIso {
        span: DocSpan {
            start_utf8: 0,
            end_utf8: 0,
        },
        yyyy_mm_dd: [0; 10],
    };

    pub fn span(self) -> DocSpan {
        match self {
            Self::DateIso { span, .. } | Self::Number { span, .. } => span,
        }
    }

    pub fn text(self, source: &str) -> Result<&str, NlpContractError> {
        slice_source(self.span(), source)
    }
}

/// Span plan without owned `String` surfaces. Interned `&'static str` is not C ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanView {
    pub span: DocSpan,
    pub content_hash: u64,
    pub source_hash: u64,
    pub term_iri: &'static str,
    pub kind: &'static str,
}

impl PlanView {
    pub const EMPTY: Self = Self {
        span: DocSpan {
            start_utf8: 0,
            end_utf8: 0,
        },
        content_hash: 0,
        source_hash: 0,
        term_iri: "",
        kind: "",
    };

    pub fn text(self, source: &str) -> Result<&str, NlpContractError> {
        slice_source(self.span, source)
    }
}

/// Caller-owned output slices. Overflow is a typed error; counts are not
/// written as a successful summary when any channel is short.
pub struct DocumentViewBuffers<'a> {
    pub tokens: &'a mut [TokenView],
    pub sentences: &'a mut [SentenceView],
    pub hits: &'a mut [HitView],
    pub norms: &'a mut [NormView],
    pub plans: &'a mut [PlanView],
}

/// Filled counts for a successful buffered analysis.
///
/// `usize` counts are native-width (32-bit on wasm32). v1 is not a C layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSummary {
    pub contract_version: u16,
    pub source_hash: u64,
    pub token_count: usize,
    pub sentence_count: usize,
    pub hit_count: usize,
    pub norm_count: usize,
    pub plan_count: usize,
    pub state: super::error::AnalysisState,
}

/// Half-open UTF-8 byte span against `source`, or [`NlpContractError::SpanInvalid`].
pub fn slice_source(span: DocSpan, source: &str) -> Result<&str, NlpContractError> {
    span.slice(source).ok_or(NlpContractError::SpanInvalid {
        start: span.start_utf8,
        end: span.end_utf8,
    })
}
