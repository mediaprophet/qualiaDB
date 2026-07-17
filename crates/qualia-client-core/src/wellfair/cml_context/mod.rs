//! Rust-native **CML context graph** + **COF HTML+RDFa** for the hypermedia Library.
//!
//! Goal: every ingested document (legislation, notes, policies, contracts) gets a
//! **TEXT → CONCEPT → LOGIC** layer that is honest about provenance:
//!
//! - All machine output is **`cml:Proposed`** (never `cml:Attested`).
//! - Structure + signal extraction is **deterministic** (no Python, no Ollama required).
//! - Deontic cues compile into real `NQuin` norms (`OP_OBLIGATE` / `OP_PERMIT` / `OP_FORBID`).
//! - Privacy / GDPR-family / rights / temporal / cross-ref signals become topics + graph edges.
//! - **COF** (`cof_html`) serialises the same graph as **HTML+RDFa** for agent windows, with
//!   **token-bounded segments** (index + body packs) when a document is large.
//!
//! Optional local LLM enrichment can attach later on the same `ContextUnit` surface; the
//! product path stays pure Rust.

mod cof_html;
mod extract;
mod graph;

pub use cof_html::{
    build_cof_package, pack_units_into_segments, render_cof_document, render_unit_fragment,
    CofPackage, CofSegment, CofStyle, COF_PROFILE, DEFAULT_SEGMENT_MAX_CHARS, MEDIA_TYPE_COF,
};
pub use extract::{
    classify_deontic, extract_cross_refs, extract_privacy_signals, extract_rights_signals,
    extract_temporal_signals, DeonticClass, PrivacySignal, SignalHit,
};
pub use graph::{
    build_document_context, build_unit_context, units_from_headings, units_from_paragraphs,
    CmlContextGraph, CmlConcept, ContextUnit,
};
