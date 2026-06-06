use serde::{Deserialize, Serialize};

/// Represents a downloadable LLM / model package (primarily GGUF)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResource {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub format: String,           // e.g. "gguf"
    pub quantization: Option<String>,
    pub size_mb: Option<u32>,
    pub download: DownloadInfo,
    pub license: Option<String>,
    pub recommended_for: Option<Vec<String>>,
    pub ram_estimate_mb: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    #[serde(rename = "type")]
    pub download_type: String,   // "huggingface", "direct", "github", etc.
    pub repo: Option<String>,
    pub file: Option<String>,
    pub url: Option<String>,
}

/// Represents an ontology resource
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

/// Represents a public SPARQL endpoint
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