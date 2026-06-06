//! Resource Catalog — canonical types, Quin encoding, and CapabilityProfile binding.
//!
//! This module defines the authoritative Rust types for the YAML-backed resource
//! catalog (`resources/*.yaml`). It is the single source of truth — the Flutter
//! bridge and CLI both reference these types, not their own copies.
//!
//! # Quin encoding
//!
//! Each catalog entry can be encoded as a set of `QualiaQuin`s for first-class
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

use serde::{Deserialize, Serialize};
use crate::{QualiaQuin, q_hash};
use crate::llm_agent::AgentBackend;
use crate::profiles::CapabilityProfile;

// ─── Inline type tag for object field (resolver.rs convention) ───────────────
const INLINE_TAG_INTEGER: u64 = 0x1u64 << 60;

// ─── Catalog context hashes (compile-time) ───────────────────────────────────
const CTX_LLM: u64      = q_hash("catalog:llm");
const CTX_ONT: u64      = q_hash("catalog:ontology");
const CTX_PROV: u64     = q_hash("catalog:provenance");

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
}

impl DownloadInfo {
    /// Resolve to a concrete download URL.
    pub fn resolved_url(&self) -> Option<String> {
        match self.download_type.as_str() {
            "huggingface" => {
                let repo = self.repo.as_deref()?;
                let file = self.file.as_deref()?;
                Some(format!("https://huggingface.co/{}/resolve/main/{}", repo, file))
            }
            _ => self.url.clone(),
        }
    }

    /// Local filename to use when saving (falls back to last URL segment).
    pub fn local_filename(&self) -> Option<String> {
        if let Some(ref f) = self.file {
            return Some(f.clone());
        }
        self.url.as_ref().and_then(|u| u.split('/').last().map(|s| s.to_string()))
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
}

impl LLMResource {
    /// Encode this catalog entry as a set of `QualiaQuin`s.
    ///
    /// Returns Quins using the `catalog:llm` context graph.  Vec allocation is
    /// intentional — this runs once at catalog load, not in a hot evaluator loop.
    pub fn to_quins(&self) -> Vec<QualiaQuin> {
        let subject = q_hash(&format!("llm:{}", self.id));
        let mut out = Vec::with_capacity(8);

        // Format (always present)
        out.push(Self::quin(subject, q_hash("llm:hasFormat"), q_hash(&self.format), CTX_LLM));

        // Quantization
        if let Some(ref q) = self.quantization {
            out.push(Self::quin(subject, q_hash("llm:hasQuantization"), q_hash(q), CTX_LLM));
        }

        // File size (inline integer)
        if let Some(sz) = self.size_mb {
            out.push(Self::quin(subject, q_hash("llm:hasSizeMb"),
                INLINE_TAG_INTEGER | sz as u64, CTX_LLM));
        }

        // Peak RAM (inline integer)
        if let Some(ram) = self.ram_estimate_mb {
            out.push(Self::quin(subject, q_hash("llm:hasRamEstimateMb"),
                INLINE_TAG_INTEGER | ram as u64, CTX_LLM));
        }

        // Source repo
        if let Some(ref repo) = self.download.repo {
            out.push(Self::quin(subject, q_hash("llm:hasSourceRepo"),
                q_hash(&format!("hf:{}", repo)), CTX_LLM));
        }

        // License
        if let Some(ref lic) = self.license {
            out.push(Self::quin(subject, q_hash("llm:hasLicense"),
                q_hash(&format!("license:{}", lic)), CTX_LLM));
        }

        // Provider
        if let Some(ref prov) = self.provider {
            out.push(Self::quin(subject, q_hash("llm:hasProvider"),
                q_hash(prov), CTX_LLM));
        }

        out
    }

    /// Generate a provenance `QualiaQuin` recording a completed download event.
    ///
    /// Written to the WAL immediately after the file is saved to disk.
    pub fn provenance_quin(&self, timestamp_unix: u64, local_path: &str) -> QualiaQuin {
        let subject   = q_hash(&format!("download:{}", self.id));
        let predicate = q_hash("prov:wasGeneratedBy");
        let object    = q_hash(local_path);
        let context   = CTX_PROV;
        // Store lower 32 bits of Unix timestamp in metadata
        let metadata  = timestamp_unix & 0xFFFF_FFFF;
        let parity    = subject ^ predicate ^ object ^ context ^ metadata;
        QualiaQuin { subject, predicate, object, context, metadata, parity }
    }

    /// Generate a supplementary provenance Quin recording the source URL.
    pub fn source_url_quin(&self) -> Option<QualiaQuin> {
        let url = self.download.resolved_url()?;
        let subject   = q_hash(&format!("download:{}", self.id));
        let predicate = q_hash("prov:hadPrimarySource");
        let object    = q_hash(&url);
        let parity    = subject ^ predicate ^ object ^ CTX_PROV;
        Some(QualiaQuin { subject, predicate, object, context: CTX_PROV,
            metadata: 0, parity })
    }

    /// Build a `CapabilityProfile` for this model, bound to its local file path.
    ///
    /// The profile ID is deterministic: `q_hash("profile:{id}")`, so callers
    /// can look it up without storing a separate reference.
    pub fn to_capability_profile(&self, local_path: &str) -> CapabilityProfile {
        CapabilityProfile {
            profile_id: q_hash(&format!("profile:{}", self.id)),
            active_engines: vec![],  // no engine restrictions — all SlgOpcodes permitted
            loaded_ontologies: vec![],
            preferred_backend: AgentBackend::Local {
                model_path: local_path.to_string(),
                context_window: 4096,
                quantization: self.quantization.clone()
                    .unwrap_or_else(|| "Q4_K_M".to_string()),
            },
            permitted_intent_frames: vec![],  // no frame restrictions
        }
    }

    fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> QualiaQuin {
        let parity = subject ^ predicate ^ object ^ context;
        QualiaQuin { subject, predicate, object, context, metadata: 0, parity }
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
    pub size_estimate_mb: Option<u32>,
    pub download: DownloadInfo,
    pub license: Option<String>,
    pub domain: Option<String>,
    pub tags: Option<Vec<String>>,
    pub import_strategy: Option<String>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
}

impl OntologyResource {
    /// Encode this ontology entry as `QualiaQuin`s in the `catalog:ontology` context.
    pub fn to_quins(&self) -> Vec<QualiaQuin> {
        let subject = q_hash(&format!("ont:{}", self.id));
        let mut out = Vec::with_capacity(5);

        out.push(Self::quin(subject, q_hash("ont:hasFormat"), q_hash(&self.format), CTX_ONT));

        if let Some(ref domain) = self.domain {
            out.push(Self::quin(subject, q_hash("ont:hasDomain"), q_hash(domain), CTX_ONT));
        }

        if let Some(sz) = self.size_estimate_mb {
            out.push(Self::quin(subject, q_hash("ont:hasSizeMb"),
                INLINE_TAG_INTEGER | sz as u64, CTX_ONT));
        }

        if let Some(ref lic) = self.license {
            out.push(Self::quin(subject, q_hash("ont:hasLicense"),
                q_hash(&format!("license:{}", lic)), CTX_ONT));
        }

        if let Some(ref src) = self.source {
            out.push(Self::quin(subject, q_hash("ont:hasSource"), q_hash(src), CTX_ONT));
        }

        out
    }

    /// Provenance Quin for a completed ontology import.
    pub fn provenance_quin(&self, timestamp_unix: u64, local_path: &str) -> QualiaQuin {
        let subject   = q_hash(&format!("import:{}", self.id));
        let predicate = q_hash("prov:wasGeneratedBy");
        let object    = q_hash(local_path);
        let metadata  = timestamp_unix & 0xFFFF_FFFF;
        let parity    = subject ^ predicate ^ object ^ CTX_PROV ^ metadata;
        QualiaQuin { subject, predicate, object, context: CTX_PROV, metadata, parity }
    }

    fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> QualiaQuin {
        let parity = subject ^ predicate ^ object ^ context;
        QualiaQuin { subject, predicate, object, context, metadata: 0, parity }
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
        Self { llms: vec![], ontologies: vec![], sparql_endpoints: vec![] }
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
}
