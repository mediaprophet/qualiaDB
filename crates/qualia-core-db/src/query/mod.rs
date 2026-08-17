//! `query` category (reorg).

pub mod cbor_compiler;
pub mod graph_accel;
pub mod graph_index;
#[cfg(not(target_arch = "wasm32"))]
pub mod graph_proof;
#[cfg(test)]
mod graph_proof_tests;
pub mod indexing;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest_formats;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest_resume;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest_job;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest_report;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod ingestion;
pub mod lexicon;
pub mod mini_parser;
#[cfg(not(target_arch = "wasm32"))]
pub mod ontology_loader;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod query_compiler;
pub mod query_engine;
pub mod rdf_star;
pub mod resolve;
pub mod resolver;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-ontology",
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod shacl_compiler;
pub mod spawn_decay;
pub mod temporal_graph;
pub mod temporal_scrub;
pub mod visual_model_bridge;
