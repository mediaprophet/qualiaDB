//! Resource Catalog — canonical types, Quin encoding, and CapabilityProfile binding.
//!
//! This module defines the authoritative Rust types for the YAML-backed resource
//! catalog (`resources/*.yaml`). It is the single source of truth — the Flutter
//! bridge and CLI both reference these types, not their own copies.
//!
//! # Quin encoding
//!
//! Each catalog entry can be encoded as a set of `NQuin`s for first-class
//! graph membership. Predicate convention:
//!
//! | Predicate string         | Meaning                        |
//! |--------------------------|--------------------------------|
//! | `llm:hasFormat`          | Model file format (e.g. `gguf`)|
//! | `llm:hasQuantization`    | Quantization level             |
//! | `llm:hasSizeMb`          | File size (inline integer)     |
//! | `llm:hasRamEstimateMb`   | Peak RAM needed (inline int)   |
//! | `llm:hasSourceRepo`      | Source repo hash               |
//! | `llm:hasLicense`         | License identifier hash        |
//! | `ont:hasFormat`          | Ontology serialisation format  |
//! | `ont:hasDomain`          | Domain tag hash                |
//! | `prov:wasGeneratedBy`    | Provenance: download event     |
//! | `prov:atPath`            | Local file path hash           |
//! | `prov:atTimestamp`       | Unix timestamp (inline int)    |

use crate::llm_agent::AgentBackend;
use crate::profiles::CapabilityProfile;
use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

// ─── Inline type tag for object field (resolver.rs convention) ───────────────
const INLINE_TAG_INTEGER: u64 = 0x1u64 << 60;

// ─── Catalog context hashes (compile-time) ───────────────────────────────────
const CTX_LLM: u64 = q_hash("catalog:llm");
const CTX_ONT: u64 = q_hash("catalog:ontology");
const CTX_PROV: u64 = q_hash("catalog:provenance");

// ─── Download info ────────────────────────────────────────────────────────────

/// Where and how to fetch a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    /// Source type: `"huggingface"`, `"direct"`, `"github"`.
    #[serde(rename = "type")]
    pub download_type: String,
    /// HuggingFace repo slug (e.g. `"unsloth/Phi-3-mini-4k-instruct-GGUF"`).
    pub repo: Option<String>,
    /// Filename within the repo (e.g. `"Phi-3-mini-4k-instruct.Q4_K_M.gguf"`).
    pub file: Option<String>,
    /// Direct URL for `"direct"` or `"github"` sources.
    pub url: Option<String>,
    /// Path within a GitHub repo (used with `type: github`).
    pub path: Option<String>,
}

impl DownloadInfo {
    /// Resolve to a concrete download URL.
    pub fn resolved_url(&self) -> Option<String> {
        match self.download_type.as_str() {
            "huggingface" => {
                let repo = self.repo.as_deref()?;
                let file = self.file.as_deref()?;
                Some(format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    repo, file
                ))
            }
            "github" => {
                let repo = self.repo.as_deref()?;
                let path = self.path.as_deref()?;
                let path = path.trim_start_matches('/');
                Some(format!(
                    "https://raw.githubusercontent.com/{}/main/{}",
                    repo, path
                ))
            }
            _ => self.url.clone(),
        }
    }

    /// Local filename to use when saving (falls back to last URL segment).
    pub fn local_filename(&self) -> Option<String> {
        if let Some(ref f) = self.file {
            return Some(f.clone());
        }
        self.url
            .as_ref()
            .and_then(|u| u.split('/').last().map(|s| s.to_string()))
    }
}

// ─── LLM resource ─────────────────────────────────────────────────────────────

/// A downloadable LLM / model package (primarily GGUF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResource {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    /// File format — always `"gguf"` for Qualia-compatible models.
    pub format: String,
    /// Quantization level (e.g. `"Q4_K_M"`, `"Q8_0"`).
    pub quantization: Option<String>,
    /// Compressed file size in MB.
    pub size_mb: Option<u32>,
    pub download: DownloadInfo,
    pub license: Option<String>,
    pub recommended_for: Option<Vec<String>>,
    /// Peak RAM required (model + runtime overhead) in MB.
    pub ram_estimate_mb: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
    /// `text` (default) or `multimodal` (requires `vision_projector`).
    pub modality: Option<String>,
    /// Vision / CLIP projector GGUF paired with the language model.
    pub vision_projector: Option<DownloadInfo>,
    /// Architecture family for multimodal routing: `llava`, `qwen2vl`, `smolvlm`, `gemma3`.
    pub architecture: Option<String>,
    /// Recommended decode context window in tokens.
    pub context_window: Option<u32>,
}

impl LLMResource {
    pub fn is_multimodal(&self) -> bool {
        matches!(self.modality.as_deref(), Some("multimodal" | "vision"))
            || self.vision_projector.is_some()
    }

    pub fn effective_context_window(&self) -> u32 {
        self.context_window
            .unwrap_or(if self.is_multimodal() { 8192 } else { 4096 })
    }
    /// Encode this catalog entry as a set of `NQuin`s.
    ///
    /// Returns Quins using the `catalog:llm` context graph.  Vec allocation is
    /// intentional — this runs once at catalog load, not in a hot evaluator loop.
    pub fn to_quins(&self) -> Vec<NQuin> {
        let subject = q_hash(&format!("llm:{}", self.id));
        let mut out = Vec::with_capacity(8);

        // Format (always present)
        out.push(Self::quin(
            subject,
            q_hash("llm:hasFormat"),
            q_hash(&self.format),
            CTX_LLM,
        ));

        // Quantization
        if let Some(ref q) = self.quantization {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasQuantization"),
                q_hash(q),
                CTX_LLM,
            ));
        }

        // File size (inline integer)
        if let Some(sz) = self.size_mb {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasSizeMb"),
                INLINE_TAG_INTEGER | sz as u64,
                CTX_LLM,
            ));
        }

        // Peak RAM (inline integer)
        if let Some(ram) = self.ram_estimate_mb {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasRamEstimateMb"),
                INLINE_TAG_INTEGER | ram as u64,
                CTX_LLM,
            ));
        }

        // Source repo
        if let Some(ref repo) = self.download.repo {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasSourceRepo"),
                q_hash(&format!("hf:{}", repo)),
                CTX_LLM,
            ));
        }

        // License
        if let Some(ref lic) = self.license {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasLicense"),
                q_hash(&format!("license:{}", lic)),
                CTX_LLM,
            ));
        }

        // Provider
        if let Some(ref prov) = self.provider {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasProvider"),
                q_hash(prov),
                CTX_LLM,
            ));
        }

        if self.is_multimodal() {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasModality"),
                q_hash("multimodal"),
                CTX_LLM,
            ));
        }

        if let Some(ref arch) = self.architecture {
            out.push(Self::quin(
                subject,
                q_hash("llm:hasArchitecture"),
                q_hash(arch),
                CTX_LLM,
            ));
        }

        out
    }

    /// Generate a provenance `NQuin` recording a completed download event.
    ///
    /// Written to the WAL immediately after the file is saved to disk.
    pub fn provenance_quin(&self, timestamp_unix: u64, local_path: &str) -> NQuin {
        let subject = q_hash(&format!("download:{}", self.id));
        let predicate = q_hash("prov:wasGeneratedBy");
        let object = q_hash(local_path);
        let context = CTX_PROV;
        // Store lower 32 bits of Unix timestamp in metadata
        let metadata = timestamp_unix & 0xFFFF_FFFF;
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }

    /// Generate a supplementary provenance Quin recording the source URL.
    pub fn source_url_quin(&self) -> Option<NQuin> {
        let url = self.download.resolved_url()?;
        let subject = q_hash(&format!("download:{}", self.id));
        let predicate = q_hash("prov:hadPrimarySource");
        let object = q_hash(&url);
        let parity = subject ^ predicate ^ object ^ CTX_PROV;
        Some(NQuin {
            subject,
            predicate,
            object,
            context: CTX_PROV,
            metadata: 0,
            parity,
        })
    }

    /// Build a `CapabilityProfile` for this model, bound to its local file path.
    ///
    /// The profile ID is deterministic: `q_hash("profile:{id}")`, so callers
    /// can look it up without storing a separate reference.
    pub fn to_capability_profile(&self, local_path: &str) -> CapabilityProfile {
        self.to_capability_profile_with_projector(local_path, None)
    }

    pub fn to_capability_profile_with_projector(
        &self,
        local_path: &str,
        vision_projector_path: Option<&str>,
    ) -> CapabilityProfile {
        let modality = if self.is_multimodal() {
            "multimodal".to_string()
        } else {
            "text".to_string()
        };
        CapabilityProfile {
            profile_id: q_hash(&format!("profile:{}", self.id)),
            active_engines: vec![],
            loaded_ontologies: vec![],
            preferred_backend: AgentBackend::Local {
                model_path: local_path.to_string(),
                context_window: self.effective_context_window(),
                quantization: self
                    .quantization
                    .clone()
                    .unwrap_or_else(|| "Q4_K_M".to_string()),
                vision_projector_path: vision_projector_path.map(|s| s.to_string()),
                modality,
                architecture: self.architecture.clone(),
            },
            permitted_intent_frames: vec![],
        }
    }

    fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        let parity = subject ^ predicate ^ object ^ context;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity,
        }
    }
}

// ─── Ontology resource ────────────────────────────────────────────────────────

/// A downloadable ontology (OWL, SKOS, RDF/Turtle, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyResource {
    pub id: String,
    pub name: String,
    pub acronym: Option<String>,
    pub source: Option<String>,
    pub format: String,
    /// YAML may use fractional estimates (e.g. `0.2` MB).
    pub size_estimate_mb: Option<f64>,
    pub download: DownloadInfo,
    pub license: Option<String>,
    pub domain: Option<String>,
    pub tags: Option<Vec<String>>,
    pub import_strategy: Option<String>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
}

impl OntologyResource {
    /// Encode this ontology entry as `NQuin`s in the `catalog:ontology` context.
    pub fn to_quins(&self) -> Vec<NQuin> {
        let subject = q_hash(&format!("ont:{}", self.id));
        let mut out = Vec::with_capacity(5);

        out.push(Self::quin(
            subject,
            q_hash("ont:hasFormat"),
            q_hash(&self.format),
            CTX_ONT,
        ));

        if let Some(ref domain) = self.domain {
            out.push(Self::quin(
                subject,
                q_hash("ont:hasDomain"),
                q_hash(domain),
                CTX_ONT,
            ));
        }

        if let Some(sz) = self.size_estimate_mb {
            let mb = sz.ceil().max(0.0) as u64;
            out.push(Self::quin(
                subject,
                q_hash("ont:hasSizeMb"),
                INLINE_TAG_INTEGER | mb,
                CTX_ONT,
            ));
        }

        if let Some(ref lic) = self.license {
            out.push(Self::quin(
                subject,
                q_hash("ont:hasLicense"),
                q_hash(&format!("license:{}", lic)),
                CTX_ONT,
            ));
        }

        if let Some(ref src) = self.source {
            out.push(Self::quin(
                subject,
                q_hash("ont:hasSource"),
                q_hash(src),
                CTX_ONT,
            ));
        }

        out
    }

    /// Provenance Quin for a completed ontology import.
    pub fn provenance_quin(&self, timestamp_unix: u64, local_path: &str) -> NQuin {
        let subject = q_hash(&format!("import:{}", self.id));
        let predicate = q_hash("prov:wasGeneratedBy");
        let object = q_hash(local_path);
        let metadata = timestamp_unix & 0xFFFF_FFFF;
        let parity = subject ^ predicate ^ object ^ CTX_PROV ^ metadata;
        NQuin {
            subject,
            predicate,
            object,
            context: CTX_PROV,
            metadata,
            parity,
        }
    }

    fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        let parity = subject ^ predicate ^ object ^ context;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity,
        }
    }
}

// ─── SPARQL endpoint resource ─────────────────────────────────────────────────

/// A public SPARQL endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SPARQLResource {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub gui: Option<String>,
    pub maintainer: Option<String>,
    pub reliability: Option<String>,
    pub domains: Option<Vec<String>>,
    pub rate_limit: Option<String>,
    pub federation_supported: Option<bool>,
    pub example_queries: Option<Vec<ExampleQuery>>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleQuery {
    pub description: Option<String>,
    pub query: Option<String>,
}

// ─── Resource catalog ─────────────────────────────────────────────────────────

/// The complete in-memory resource catalog, loaded from `resources/*.yaml`.
pub struct ResourceCatalog {
    pub llms: Vec<LLMResource>,
    pub ontologies: Vec<OntologyResource>,
    pub sparql_endpoints: Vec<SPARQLResource>,
}

impl ResourceCatalog {
    pub fn empty() -> Self {
        Self {
            llms: vec![],
            ontologies: vec![],
            sparql_endpoints: vec![],
        }
    }

    pub fn find_llm(&self, id: &str) -> Option<&LLMResource> {
        self.llms.iter().find(|r| r.id == id)
    }

    pub fn find_ontology(&self, id: &str) -> Option<&OntologyResource> {
        self.ontologies.iter().find(|r| r.id == id)
    }

    pub fn find_sparql(&self, id: &str) -> Option<&SPARQLResource> {
        self.sparql_endpoints.iter().find(|r| r.id == id)
    }

    /// Serialize a summary for FRB / JSON consumers.
    pub fn summary_json(&self) -> String {
        #[derive(Serialize)]
        struct Summary {
            llm_count: usize,
            ontology_count: usize,
            sparql_count: usize,
        }
        serde_json::to_string(&Summary {
            llm_count: self.llms.len(),
            ontology_count: self.ontologies.len(),
            sparql_count: self.sparql_endpoints.len(),
        })
        .unwrap_or_else(|_| "{}".to_string())
    }
}

// ─── YAML loader (canonical — CLI, Flutter, client-core) ─────────────────────

use std::path::{Path, PathBuf};

/// Errors loading `resources/*.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    Io { path: PathBuf, message: String },
    Parse { path: PathBuf, message: String },
    Index { message: String },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::Io { path, message } => {
                write!(f, "cannot read {}: {}", path.display(), message)
            }
            CatalogError::Parse { path, message } => {
                write!(f, "{}: {}", path.display(), message)
            }
            CatalogError::Index { message } => write!(f, "catalog.yaml: {}", message),
        }
    }
}

impl std::error::Error for CatalogError {}

#[derive(Debug, Deserialize)]
struct CatalogRoot {
    catalog: CatalogMeta,
}

#[derive(Debug, Deserialize)]
struct CatalogMeta {
    sources: CatalogSources,
}

#[derive(Debug, Deserialize)]
struct CatalogSources {
    llms: String,
    ontologies: String,
    sparql_endpoints: String,
}

#[derive(Debug, Deserialize)]
struct LlmsFile {
    llms: Vec<LLMResource>,
}

#[derive(Debug, Deserialize)]
struct OntologiesFile {
    ontologies: Vec<OntologyResource>,
}

#[derive(Debug, Deserialize)]
struct SparqlFile {
    sparql_endpoints: Vec<SPARQLResource>,
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, CatalogError> {
    let raw = std::fs::read_to_string(path).map_err(|e| CatalogError::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    serde_yaml::from_str(&raw).map_err(|e| CatalogError::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Resolve the resources directory for desktop / dev builds.
///
/// Order: `QUALIA_RESOURCES_DIR` → `{exe}/bundled/resources/` → dev tree `../../resources`.
pub fn resolve_resources_dir() -> PathBuf {
    if let Ok(extra) = std::env::var("QUALIA_RESOURCES_DIR") {
        return PathBuf::from(extra);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent() {
            for rel in ["bundled/resources", "resources", "bundled"] {
                let candidate = root.join(rel);
                if candidate.join("catalog.yaml").is_file() {
                    return candidate;
                }
            }
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources")
}

/// Load the full catalog from `dir/catalog.yaml` and referenced child YAML files.
pub fn load_from_dir(dir: &Path) -> Result<ResourceCatalog, CatalogError> {
    let index_path = dir.join("catalog.yaml");
    let index: CatalogRoot = read_yaml(&index_path)?;

    let llms_path = dir.join(&index.catalog.sources.llms);
    let ont_path = dir.join(&index.catalog.sources.ontologies);
    let sparql_path = dir.join(&index.catalog.sources.sparql_endpoints);

    let llms_file: LlmsFile = read_yaml(&llms_path)?;
    let ont_file: OntologiesFile = read_yaml(&ont_path)?;
    let sparql_file: SparqlFile = read_yaml(&sparql_path)?;

    Ok(ResourceCatalog {
        llms: llms_file.llms,
        ontologies: ont_file.ontologies,
        sparql_endpoints: sparql_file.sparql_endpoints,
    })
}

/// Load from [`resolve_resources_dir()`].
pub fn load_default() -> Result<ResourceCatalog, CatalogError> {
    load_from_dir(&resolve_resources_dir())
}

#[cfg(test)]
mod load_tests {
    use super::*;

    fn resources_fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources")
    }

    #[test]
    fn loads_all_entries_from_committed_yaml() {
        let dir = resources_fixture_dir();
        let cat = load_from_dir(&dir).expect("catalog should load");
        assert!(
            cat.llms.len() >= 10,
            "expected >=10 LLMs, got {}",
            cat.llms.len()
        );
        let multimodal = cat.llms.iter().filter(|m| m.is_multimodal()).count();
        assert!(
            multimodal >= 3,
            "expected >=3 multimodal LLMs, got {multimodal}"
        );
        assert!(
            cat.ontologies.len() >= 12,
            "expected >=12 ontologies, got {}",
            cat.ontologies.len()
        );
        assert!(
            cat.sparql_endpoints.len() >= 3,
            "expected >=3 SPARQL endpoints, got {}",
            cat.sparql_endpoints.len()
        );
    }

    #[test]
    fn github_download_resolves_raw_url() {
        let info = DownloadInfo {
            download_type: "github".to_string(),
            repo: Some("schemaorg/schemaorg".to_string()),
            file: None,
            url: None,
            path: Some("data/releases/27.0/schemaorg-all-https.rdf".to_string()),
        };
        let url = info.resolved_url().expect("github url");
        assert!(url.contains("raw.githubusercontent.com/schemaorg/schemaorg"));
        assert!(url.contains("schemaorg-all-https.rdf"));
    }

    #[test]
    fn find_llm_by_id() {
        let cat = load_from_dir(&resources_fixture_dir()).unwrap();
        assert!(cat.find_llm("phi-3-mini-4k-instruct-q4km").is_some());
    }
}
