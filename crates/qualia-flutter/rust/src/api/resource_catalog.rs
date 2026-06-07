//! Flutter bridge API — Resource Catalog.
//!
//! Thin FRB wrapper over `qualia_core_db::resource_catalog::load_from_dir`.

use flutter_rust_bridge::frb;
use qualia_core_db::resource_catalog::{
    self, LLMResource as CoreLlm, OntologyResource as CoreOnt, ResourceCatalog,
};

#[frb]
#[derive(Debug, Clone)]
pub struct LLMResource {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub format: String,
    pub quantization: Option<String>,
    pub size_mb: Option<u32>,
    pub ram_estimate_mb: Option<u32>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub recommended_for: Option<Vec<String>>,
    pub download_url: Option<String>,
    pub notes: Option<String>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct OntologyResource {
    pub id: String,
    pub name: String,
    pub acronym: Option<String>,
    pub domain: Option<String>,
    pub format: String,
    pub size_estimate_mb: Option<u32>,
    pub license: Option<String>,
    pub download_url: Option<String>,
    pub notes: Option<String>,
}

fn load_catalog() -> ResourceCatalog {
    resource_catalog::load_default().unwrap_or_else(|_| ResourceCatalog::empty())
}

fn map_llm(r: CoreLlm) -> LLMResource {
    LLMResource {
        id: r.id,
        name: r.name,
        provider: r.provider,
        format: r.format,
        quantization: r.quantization,
        size_mb: r.size_mb,
        ram_estimate_mb: r.ram_estimate_mb,
        license: r.license,
        tags: r.tags,
        recommended_for: r.recommended_for,
        download_url: r.download.resolved_url(),
        notes: r.notes,
    }
}

fn map_ontology(r: CoreOnt) -> OntologyResource {
    OntologyResource {
        id: r.id,
        name: r.name,
        acronym: r.acronym,
        domain: r.domain,
        format: r.format,
        size_estimate_mb: r.size_estimate_mb.map(|s| s.ceil() as u32),
        license: r.license,
        download_url: r.download.resolved_url(),
        notes: r.notes,
    }
}

#[frb]
pub fn load_llm_resources() -> Vec<LLMResource> {
    load_catalog().llms.into_iter().map(map_llm).collect()
}

#[frb]
pub fn load_ontology_resources() -> Vec<OntologyResource> {
    load_catalog()
        .ontologies
        .into_iter()
        .map(map_ontology)
        .collect()
}

#[frb]
pub fn load_resource_catalog_summary() -> String {
    load_catalog().summary_json()
}

#[frb]
pub fn download_llm(id: String) -> Result<String, String> {
    let status = std::process::Command::new("qualia-cli")
        .args(["resources", "download", &id])
        .status()
        .map_err(|e| format!("Failed to launch qualia-cli: {}", e))?;

    if status.success() {
        Ok(format!("Download complete for: {}", id))
    } else {
        Err(format!(
            "qualia-cli resources download {} failed (exit {:?})",
            id,
            status.code()
        ))
    }
}

#[frb]
pub fn import_ontology(id: String) -> Result<String, String> {
    let status = std::process::Command::new("qualia-cli")
        .args(["resources", "import-ontology", &id])
        .status()
        .map_err(|e| format!("Failed to launch qualia-cli: {}", e))?;

    if status.success() {
        Ok(format!("Ontology imported: {}", id))
    } else {
        Err(format!(
            "qualia-cli resources import-ontology {} failed (exit {:?})",
            id,
            status.code()
        ))
    }
}
