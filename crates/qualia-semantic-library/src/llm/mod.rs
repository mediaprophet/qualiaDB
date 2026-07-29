//! The external-LLM seam. Embeddings and structured method-extraction are the
//! only steps that need a language model; everything else in the crate is
//! deterministic. That model is reached **over HTTP behind this trait**, so it
//! can be a local Ollama server today, a hosted API tomorrow, or the native
//! QualiaDB engine later — without touching the rest of the pipeline. It is
//! never compiled into QualiaDB's core inference path.

pub mod ollama;
pub use ollama::OllamaBackend;

use serde::Deserialize;

use crate::container::{AssetKind, HmcWriter};
use crate::embedding::encode_f32_matrix;

/// A swappable language-model backend.
pub trait LlmBackend {
    /// Embed a batch of texts. Returns one vector per input, all of the same
    /// dimensionality (`embed_dim`).
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;

    /// Free-form completion used for structured extraction. `system` sets the
    /// instruction; `prompt` carries the chunk. Implementations should request
    /// non-streaming output.
    fn generate(&self, system: &str, prompt: &str) -> anyhow::Result<String>;

    /// Embedding dimensionality, if known ahead of a call (0 = discover on first embed).
    fn embed_dim(&self) -> u32;

    /// Identifier recorded in the container manifest (e.g. `ollama/nomic-embed-text`).
    fn model_id(&self) -> String;
}

/// Minimal view of a stored chunk — only what embedding needs (decoupled from
/// the `ingest` module so the LLM seam compiles without the `pdf` feature).
#[derive(Deserialize)]
struct StoredChunk {
    text: String,
}

/// Enrichment pass: open a container, embed its chunks with `backend`, and write
/// the embedding matrix + updated manifest back into the same container.
pub fn embed_container(path: &std::path::Path, backend: &dyn LlmBackend) -> anyhow::Result<usize> {
    let mut writer = HmcWriter::reopen(path)?;

    let texts: Vec<String> = {
        let mut c = crate::container::HmcContainer::open(path)?;
        let jsonl = c.read_kind(AssetKind::Chunks)?;
        String::from_utf8_lossy(&jsonl)
            .lines()
            .filter_map(|l| serde_json::from_str::<StoredChunk>(l).ok())
            .map(|c| c.text)
            .collect()
    };
    if texts.is_empty() {
        return Ok(0);
    }

    let vectors = backend.embed(&texts)?;
    let dim = vectors.first().map(|v| v.len()).unwrap_or(0) as u32;

    let bytes = encode_f32_matrix(&vectors);
    writer.add_derived(
        AssetKind::Embeddings,
        "vectors.f32",
        "application/octet-stream",
        bytes,
    );

    let pipeline = &mut writer.manifest_mut().pipeline;
    pipeline.embedder = backend.model_id();
    pipeline.embed_dim = dim;
    writer.manifest_mut().status.embedded = true;

    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    writer.write_to_dir(dir)?;
    Ok(vectors.len())
}

const TAG_SYSTEM: &str = "You are a librarian for a formal-methods / mathematics / \
logic / computing research library. Given an excerpt, reply with 1 to 4 short \
lowercase topical tags (single words or hyphenated), comma-separated, no prose, \
no explanation. Prefer tags like: logic, deontic, algebra, category-theory, \
optimization, numerics, machine-learning, nlp, crypto, zero-knowledge, \
semantic-web, ontology, graph-theory, probability.";

/// Analysis pass: ask the backend for topical tags from the document's leading
/// chunks and write them into the container manifest (drives reorganise/route).
pub fn analyze_container(
    path: &std::path::Path,
    backend: &dyn LlmBackend,
) -> anyhow::Result<Vec<String>> {
    let excerpt: String = {
        let mut c = crate::container::HmcContainer::open(path)?;
        let jsonl = c.read_kind(AssetKind::Chunks)?;
        String::from_utf8_lossy(&jsonl)
            .lines()
            .filter_map(|l| serde_json::from_str::<StoredChunk>(l).ok())
            .take(4)
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    if excerpt.trim().is_empty() {
        return Ok(vec![]);
    }
    let raw = backend.generate(TAG_SYSTEM, &truncate_chars(&excerpt, 6000))?;
    let tags = parse_tags(&raw);

    let mut writer = HmcWriter::reopen(path)?;
    writer.manifest_mut().tags = tags.clone();
    writer.manifest_mut().status.analyzed = true;
    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    writer.write_to_dir(dir)?;
    Ok(tags)
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split([',', '\n'])
        .map(|t| {
            t.trim()
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
                .to_ascii_lowercase()
        })
        .filter(|t| !t.is_empty() && t.len() <= 30)
        .take(4)
        .collect()
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}
