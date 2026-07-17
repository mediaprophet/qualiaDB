//! Rust-native **CML context graph** for the hypermedia Library.
//!
//! Goal: every ingested document (legislation, notes, policies, contracts) gets a
//! **TEXT → CONCEPT → LOGIC** layer that is honest about provenance:
//!
//! - All machine output is **`cml:Proposed`** (never `cml:Attested`).
//! - Structure + signal extraction is **deterministic** (no Python, no Ollama required).
//! - Deontic cues compile into real `NQuin` norms (`OP_OBLIGATE` / `OP_PERMIT` / `OP_FORBID`).
//! - Privacy / GDPR-family / rights / temporal / cross-ref signals become topics + graph edges.
//!
//! Optional local LLM enrichment can attach later on the same `ContextUnit` surface; the
//! product path stays pure Rust.

mod extract;
mod graph;

pub use extract::{
    classify_deontic, extract_cross_refs, extract_privacy_signals, extract_rights_signals,
    extract_temporal_signals, DeonticClass, PrivacySignal, SignalHit,
};
pub use graph::{
    build_document_context, build_unit_context, units_from_headings, units_from_paragraphs,
    CmlContextGraph, CmlConcept, ContextUnit,
};
