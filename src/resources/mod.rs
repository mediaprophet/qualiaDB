pub mod types;
pub mod loader;

pub use types::*;
pub use loader::load_catalog;

/// Main entry point for the Resource Catalog system.
///
/// This module allows QualiaDB to load and work with external resources
/// (LLMs, Ontologies, SPARQL endpoints) defined in YAML files.
pub struct ResourceCatalog {
    pub llms: Vec<LLMResource>,
    pub ontologies: Vec<OntologyResource>,
    pub sparql_endpoints: Vec<SPARQLResource>,
}

impl ResourceCatalog {
    pub fn new() -> Self {
        Self {
            llms: vec![],
            ontologies: vec![],
            sparql_endpoints: vec![],
        }
    }

    /// Load all resources from the configured catalog files.
    pub fn load_from_files(&mut self) -> Result<(), String> {
        // TODO: Implement using loader::load_catalog
        Ok(())
    }
}