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
    pub modality: Option<String>,
    pub architecture: Option<String>,
    pub context_window: Option<u32>,
    pub is_multimodal: bool,
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
        modality: r.modality,
        architecture: r.architecture,
        context_window: r.context_window,
        is_multimodal: r.is_multimodal(),
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
pub async fn install_catalog_llm(id: String) -> Result<String, String> {
    let val = qualia_client_core::install_catalog_llm(id).await?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

#[frb]
pub fn get_model_lifecycle_status() -> Result<String, String> {
    let val = qualia_client_core::get_model_lifecycle_status()?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

#[frb]
pub async fn import_ontology(id: String) -> Result<String, String> {
    let val = qualia_client_core::import_catalog_ontology(id).await?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}
