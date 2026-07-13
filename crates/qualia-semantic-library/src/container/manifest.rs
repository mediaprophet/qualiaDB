//! The `manifest.json` that sits at the root of every `.hmc` container.
//!
//! The manifest is the self-describing index of a hypermedia container: it names
//! the original source, every derived asset, content hashes, and the pipeline
//! that produced them. A reader can understand a container fully from the
//! manifest alone, without unpacking the heavy assets.

use serde::{Deserialize, Serialize};

/// Current on-disk format version of the `.hmc` container.
pub const HMC_FORMAT_VERSION: u32 = 1;

/// Logical role of an asset inside a container. The path in [`AssetEntry`] is
/// authoritative; `kind` is the typed lens used for retrieval/UX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// The original, verbatim source file (e.g. the PDF). Never modified.
    Source,
    /// Canonical structured HTML (headings, MathML, provenance `data-*` attrs).
    Html,
    /// Plain text with `[[page N]]` markers (compat + cheap full-text).
    Text,
    /// CML / RDF (Turtle) semantic annotation layer over the HTML.
    Cml,
    /// Structural chunks as JSON-Lines (one object per line).
    Chunks,
    /// Binary embedding matrix for the chunks (`f32`, row-major).
    Embeddings,
    /// Extraction/analysis metadata (tool versions, timings, confidence).
    Meta,
    /// Anything else (figures, tables, page images…).
    Other,
}

impl AssetKind {
    /// Conventional directory prefix for this kind inside the zip.
    pub fn dir(self) -> &'static str {
        match self {
            AssetKind::Source => "source",
            AssetKind::Html | AssetKind::Text | AssetKind::Cml | AssetKind::Chunks => "derived",
            AssetKind::Embeddings => "embeddings",
            AssetKind::Meta => "meta",
            AssetKind::Other => "assets",
        }
    }
}

/// One stored member of the container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    /// Path of the member inside the zip (e.g. `derived/document.html`).
    pub path: String,
    /// Logical role.
    pub kind: AssetKind,
    /// MIME type, best-effort (e.g. `text/html`, `application/pdf`).
    pub mime: String,
    /// BLAKE3 hex of the asset bytes (integrity + dedup).
    pub blake3: String,
    /// Uncompressed size in bytes.
    pub bytes: u64,
}

/// Description of the original source document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Original filename as found on disk.
    pub filename: String,
    pub mime: String,
    pub size_bytes: u64,
    /// BLAKE3 hex of the original bytes — the document identity.
    pub blake3: String,
    /// Title (from document metadata, else derived from filename).
    #[serde(default)]
    pub title: String,
    /// Page count when known (PDFs); 0 otherwise.
    #[serde(default)]
    pub page_count: u32,
}

/// Which tools produced this container — for reproducibility and re-runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineInfo {
    /// Producing tool + version (e.g. `qualia-semantic-library 0.0.20`).
    pub tool: String,
    /// Text/structure extractor used (e.g. `pdf-extract`, `pymupdf`, `nougat`).
    #[serde(default)]
    pub extractor: String,
    /// Embedding model id, if embeddings were computed.
    #[serde(default)]
    pub embedder: String,
    /// Embedding dimensionality (0 if none).
    #[serde(default)]
    pub embed_dim: u32,
}

/// Coarse processing status carried in the manifest so a library scan can route
/// work (what still needs extraction / embedding / analysis) without unpacking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusFlags {
    pub extracted: bool,
    pub chunked: bool,
    pub embedded: bool,
    pub analyzed: bool,
    /// Free-form notes / non-fatal extraction warnings.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// The root manifest of an `.hmc` container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmcManifest {
    /// Always `"hmc"`.
    pub format: String,
    pub format_version: u32,
    /// Document identity = `blake3` of the source bytes (hex).
    pub doc_id: String,
    pub source: SourceInfo,
    /// RFC3339 creation timestamp.
    pub created: String,
    #[serde(default)]
    pub pipeline: PipelineInfo,
    #[serde(default)]
    pub status: StatusFlags,
    /// Topical tags (human or analysis assigned). Used by reorganise/route.
    #[serde(default)]
    pub tags: Vec<String>,
    /// All stored members (excluding the manifest itself).
    #[serde(default)]
    pub assets: Vec<AssetEntry>,
}

impl HmcManifest {
    pub fn new(source: SourceInfo) -> Self {
        let doc_id = source.blake3.clone();
        HmcManifest {
            format: "hmc".to_string(),
            format_version: HMC_FORMAT_VERSION,
            doc_id,
            source,
            created: chrono::Utc::now().to_rfc3339(),
            pipeline: PipelineInfo::default(),
            status: StatusFlags::default(),
            tags: Vec::new(),
            assets: Vec::new(),
        }
    }

    /// First asset of a given kind, if present.
    pub fn asset_of(&self, kind: AssetKind) -> Option<&AssetEntry> {
        self.assets.iter().find(|a| a.kind == kind)
    }
}
