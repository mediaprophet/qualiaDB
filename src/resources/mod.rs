pub mod types;
pub mod loader;

pub use types::*;
pub use loader::load_catalog;

/// Main Resource Catalog structure.
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

    /// Load resources from the catalog files.
    pub fn load_from_files(&mut self, catalog_path: &std::path::Path) -> Result<(), String> {
        let loaded = loader::load_catalog(catalog_path)?;
        self.llms = loaded.llms;
        self.ontologies = loaded.ontologies;
        self.sparql_endpoints = loaded.sparql_endpoints;
        Ok(())
    }

    pub fn llm_count(&self) -> usize {
        self.llms.len()
    }

    pub fn ontology_count(&self) -> usize {
        self.ontologies.len()
    }

    pub fn sparql_count(&self) -> usize {
        self.sparql_endpoints.len()
    }
}