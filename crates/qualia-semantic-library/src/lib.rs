//! # qualia-semantic-library
//!
//! A Rust-native semantic library for turning a large, messy document corpus
//! into an organised, searchable knowledge base that feeds QualiaDB.
//!
//! The unit of storage is the **hypermedia container** ([`container`]): a single
//! `.hmc` ZIP file holding the original source document *and* every asset derived
//! from it — canonical HTML, a CML/RDF semantic layer, plain text, structural
//! chunks, and embedding vectors — behind one self-describing manifest. The
//! original and its provenance never get separated.
//!
//! Pipeline (each stage is deterministic except the explicitly-marked LLM step):
//!
//! 1. [`ingest`] — hash + dedup a file, extract structured text/HTML, chunk it,
//!    and pack everything into an `.hmc` container.
//! 2. [`llm`] — an optional, swappable backend reached over HTTP (Ollama today;
//!    a cloud API or the native QualiaDB engine later) used only for embeddings
//!    and structured method extraction. This is the *only* external dependency
//!    and it lives behind the [`llm::LlmBackend`] trait. It is never wired into
//!    QualiaDB's core inference path.
//! 3. [`library`] — an index over many containers: dedup on the embedding
//!    manifold, route queries to the relevant region, rank by novelty.
//! 4. [`reorganize`] — propose/apply an organised on-disk layout from container
//!    metadata and clusters.
//!
//! This crate is an **offline developer tool**. It honours the project rule that
//! external LLM runtimes may be used as independent tools but are never compiled
//! into the QualiaDB runtime — hence the HTTP seam and the `llm-http` feature.

pub mod container;
pub mod embedding;

#[cfg(feature = "pdf")]
pub mod ingest;

#[cfg(feature = "llm-http")]
pub mod llm;

pub mod library;
pub mod reorganize;

pub use container::{
    AssetEntry, AssetKind, HmcContainer, HmcError, HmcManifest, HmcWriter, SourceInfo,
    HMC_EXTENSION, HMC_FORMAT_VERSION,
};
