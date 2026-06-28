//! `query` category (reorg).

pub mod cbor_compiler;
pub mod graph_index;
pub mod indexing;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest;
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
pub mod temporal_graph;
