use crate::resources::types::*;
use std::fs;
use std::path::Path;

/// Loads the complete Resource Catalog from the master catalog.yaml file.
pub fn load_catalog(catalog_path: &Path) -> Result<ResourceCatalog, String> {
    let catalog_content = fs::read_to_string(catalog_path)
        .map_err(|e| format!("Failed to read catalog file: {}", e))?;

    let catalog_config: CatalogConfig = serde_yaml::from_str(&catalog_content)
        .map_err(|e| format!("Failed to parse catalog.yaml: {}", e))?;

    let base_dir = catalog_path.parent().unwrap_or(Path::new("."));

    let llms = load_llms(&base_dir.join(&catalog_config.sources.llms))?;
    let ontologies = load_ontologies(&base_dir.join(&catalog_config.sources.ontologies))?;
    let sparql_endpoints = load_sparql_endpoints(&base_dir.join(&catalog_config.sources.sparql_endpoints))?;

    Ok(ResourceCatalog {
        llms,
        ontologies,
        sparql_endpoints,
    })
}

#[derive(Debug, serde::Deserialize)]
struct CatalogConfig {
    sources: Sources,
}

#[derive(Debug, serde::Deserialize)]
struct Sources {
    llms: String,
    ontologies: String,
    sparql_endpoints: String,
}

fn load_llms(path: &Path) -> Result<Vec<LLMResource>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let llms: Vec<LLMResource> = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse llms.yaml: {}", e))?;
    Ok(llms)
}

fn load_ontologies(path: &Path) -> Result<Vec<OntologyResource>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let ontologies: Vec<OntologyResource> = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse ontologies.yaml: {}", e))?;
    Ok(ontologies)
}

fn load_sparql_endpoints(path: &Path) -> Result<Vec<SPARQLResource>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let endpoints: Vec<SPARQLResource> = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse sparql_endpoints.yaml: {}", e))?;
    Ok(endpoints)
}