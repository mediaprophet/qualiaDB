//! Flutter bridge API — Resource Catalog.
//!
//! These view types mirror the canonical types in
//! `qualia_core_db::resource_catalog`. They are kept separate because
//! `flutter_rust_bridge` requires `#[frb]`-annotated structs in this crate;
//! the core engine types deliberately carry no FRB annotations.
//!
//! Data is loaded directly from `resources/llms.yaml` and
//! `resources/ontologies.yaml` at runtime — no hardcoding.
//!
//! # Download / import
//! These functions are async stubs. The real implementation runs through
//! `qualia-cli resources download <id>` which wires:
//!   GGUF download → GGufSharder pointer map → WAL provenance Quin → CapabilityProfile

use flutter_rust_bridge::frb;

// ─── Bridge-local view types ──────────────────────────────────────────────────
// Mirror qualia_core_db::resource_catalog::{LLMResource, OntologyResource}.
// Keep field names and types in sync with the canonical definitions there.

#[frb]
#[derive(Debug, Clone)]
pub struct LLMResource {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    /// Always `"gguf"` for Qualia-compatible models.
    pub format: String,
    pub quantization: Option<String>,
    pub size_mb: Option<u32>,
    pub ram_estimate_mb: Option<u32>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub recommended_for: Option<Vec<String>>,
    /// Resolved download URL (HuggingFace or direct).
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

// ─── Internal YAML shapes (not exposed to Flutter) ───────────────────────────

#[derive(serde::Deserialize)]
struct RawLLM {
    id: String,
    name: String,
    provider: Option<String>,
    #[serde(default = "default_gguf")]
    format: String,
    quantization: Option<String>,
    size_mb: Option<u32>,
    ram_estimate_mb: Option<u32>,
    license: Option<String>,
    tags: Option<Vec<String>>,
    recommended_for: Option<Vec<String>>,
    download: Option<RawDownload>,
    notes: Option<String>,
}

#[derive(serde::Deserialize)]
struct RawOntology {
    id: String,
    name: String,
    acronym: Option<String>,
    domain: Option<String>,
    #[serde(default)]
    format: String,
    size_estimate_mb: Option<u32>,
    license: Option<String>,
    download: Option<RawDownload>,
    notes: Option<String>,
}

#[derive(serde::Deserialize)]
struct RawDownload {
    #[serde(rename = "type")]
    download_type: String,
    repo: Option<String>,
    file: Option<String>,
    url: Option<String>,
}

impl RawDownload {
    fn resolved_url(&self) -> Option<String> {
        match self.download_type.as_str() {
            "huggingface" => {
                let repo = self.repo.as_deref()?;
                let file = self.file.as_deref()?;
                Some(format!("https://huggingface.co/{}/resolve/main/{}", repo, file))
            }
            _ => self.url.clone(),
        }
    }
}

fn default_gguf() -> String { "gguf".to_string() }

// ─── API functions ────────────────────────────────────────────────────────────

/// Load all LLM entries from `resources/llms.yaml`.
///
/// Returns an empty vec if the catalog file cannot be read — the UI should
/// surface this as "catalog not found" rather than crashing.
#[frb]
pub fn load_llm_resources() -> Vec<LLMResource> {
    let raw: Vec<RawLLM> = load_yaml("resources/llms.yaml");
    raw.into_iter().map(|r| LLMResource {
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
        download_url: r.download.as_ref().and_then(|d| d.resolved_url()),
        notes: r.notes,
    }).collect()
}

/// Load all ontology entries from `resources/ontologies.yaml`.
#[frb]
pub fn load_ontology_resources() -> Vec<OntologyResource> {
    let raw: Vec<RawOntology> = load_yaml("resources/ontologies.yaml");
    raw.into_iter().map(|r| OntologyResource {
        id: r.id,
        name: r.name,
        acronym: r.acronym,
        domain: r.domain,
        format: r.format,
        size_estimate_mb: r.size_estimate_mb,
        license: r.license,
        download_url: r.download.as_ref().and_then(|d| d.resolved_url()),
        notes: r.notes,
    }).collect()
}

/// Initiate a GGUF model download.
///
/// The full download pipeline (streaming fetch → GGufSharder pointer map →
/// WAL provenance Quin → CapabilityProfile) is implemented in
/// `qualia-cli resources download <id>`. This function invokes that CLI
/// command as a subprocess so the async I/O runs in the CLI's tokio runtime.
#[frb]
pub fn download_llm(id: String) -> Result<String, String> {
    let status = std::process::Command::new("qualia-cli")
        .args(["resources", "download", &id])
        .status()
        .map_err(|e| format!("Failed to launch qualia-cli: {}", e))?;

    if status.success() {
        Ok(format!("Download complete for: {}", id))
    } else {
        Err(format!("qualia-cli resources download {} failed (exit {:?})", id, status.code()))
    }
}

/// Initiate an ontology download + .q42 import.
///
/// Full pipeline: streaming fetch → `ingest::streaming_import_rdf` → WAL
/// provenance Quin. Implemented in `qualia-cli resources import-ontology <id>`.
#[frb]
pub fn import_ontology(id: String) -> Result<String, String> {
    let status = std::process::Command::new("qualia-cli")
        .args(["resources", "import-ontology", &id])
        .status()
        .map_err(|e| format!("Failed to launch qualia-cli: {}", e))?;

    if status.success() {
        Ok(format!("Ontology imported: {}", id))
    } else {
        Err(format!("qualia-cli resources import-ontology {} failed (exit {:?})", id, status.code()))
    }
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn load_yaml<T: serde::de::DeserializeOwned>(path: &str) -> Vec<T> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_yaml::from_str::<Vec<T>>(&s).unwrap_or_default(),
        Err(_) => vec![],
    }
}
