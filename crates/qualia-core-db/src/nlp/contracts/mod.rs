//! Versioned annotation contracts and caller-buffered document views (NLP-100).
//!
//! **v1 ABI (honest):** in-process Rust records. Views may contain `&'static str`
//! interned lexicon pointers and `usize` counts (native pointer width). They are
//! **not** a C FFI layout. `#[repr(C)]` is not applied to fat-pointer views.
//! Cross-language ABI notes belong to NLP-900.
//!
//! **v1 span model:** half-open UTF-8 byte [`crate::nlp::span::DocSpan`] on the
//! immutable source. Distinct UTF-16 / code-point / grapheme / model-subtoken
//! types and reversible alignment maps are NLP-101. Token layering (MWT) is
//! NLP-103. Nested/discontinuous mention lists are NLP-405.
//!
//! **v1 state:** [`AnalysisState::Complete`] is the only constructed success
//! state. Other variants are reserved for later processors; they are not
//! emitted here. Oversize input on the buffered path is
//! [`NlpContractError::SourceTooLarge`], not a `BudgetExceeded` summary.
//!
//! Hot-path records borrow the immutable source through `DocSpan`.
//! Owned strings stay on the cold [`analyze_document`] adapter.

mod analyze;
mod error;
mod views;

pub use analyze::{
    analyze_document, analyze_document_into, required_capacities, DocumentCapacities,
};
pub use error::{AnalysisState, BufferChannel, NlpContractError};
pub use views::{
    slice_source, DocumentSummary, DocumentViewBuffers, HitView, NormView, PlanView, SentenceView,
    TokenView, NORM_INDEX_NONE, SENTENCE_ID_NONE,
};

/// Contract schema version for these view records.
pub const ANNOTATION_CONTRACT_VERSION: u16 = 1;

#[cfg(test)]
mod tests;
