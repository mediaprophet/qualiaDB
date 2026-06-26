//! `query` category — consolidated from crate-root modules (reorg).

pub mod query_compiler;
pub mod query_engine;
pub mod rdf_star;
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
pub mod ingestion;
pub mod graph_index;
pub mod indexing;
pub mod temporal_graph;
