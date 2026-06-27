//! `query` category (reorg).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod query_compiler;
pub mod query_engine;
pub mod rdf_star;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-ontology",
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod shacl_compiler;
pub mod cbor_compiler;
pub mod resolve;
pub mod resolver;
pub mod lexicon;
#[cfg(not(target_arch = "wasm32"))]
pub mod ontology_loader;
pub mod mini_parser;
#[cfg(not(target_arch = "wasm32"))]
pub mod ingest;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod ingestion;
pub mod graph_index;
pub mod indexing;
pub mod temporal_graph;
