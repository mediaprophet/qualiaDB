//! Ingest orchestration: a single file → a `.hmc` hypermedia container.
//!
//! This whole module is **deterministic** — no LLM. It acquires the source,
//! extracts structured text/HTML, chunks it, emits the CML annotation layer, and
//! packs the original plus every derived asset into one container. Embeddings
//! and LLM method-extraction are separate enrichment passes ([`crate::llm`]) so
//! ingest stays reproducible and offline-clean.

pub mod acquire;
pub mod chunk;
pub mod cml;
pub mod extract;

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::container::{AssetKind, HmcWriter, PipelineInfo};

/// Options controlling an ingest run.
pub struct IngestOptions {
    /// Directory the `.hmc` containers are written to.
    pub out_dir: PathBuf,
    /// Recurse into sub-directories when ingesting a directory.
    pub recursive: bool,
    /// Skip a source whose container already exists in `out_dir`.
    pub skip_existing: bool,
}

impl Default for IngestOptions {
    fn default() -> Self {
        IngestOptions {
            out_dir: PathBuf::from("library"),
            recursive: true,
            skip_existing: true,
        }
    }
}

/// Outcome of ingesting one file.
pub struct IngestResult {
    pub container_path: PathBuf,
    pub doc_id: String,
    pub skipped: bool,
    pub notes: Vec<String>,
}

/// Ingest a single file into a container. Returns the container path.
pub fn ingest_file(path: &Path, opts: &IngestOptions) -> anyhow::Result<IngestResult> {
    let acq = acquire::acquire(path)?;
    let doc_id = acq.source.blake3.clone();
    let target = opts
        .out_dir
        .join(format!("{doc_id}.{}", crate::container::HMC_EXTENSION));

    if opts.skip_existing && target.exists() {
        return Ok(IngestResult {
            container_path: target,
            doc_id,
            skipped: true,
            notes: vec![],
        });
    }

    let mut source = acq.source.clone();
    let extracted = extract::extract(&acq);
    source.page_count = extracted.page_count;

    let chunks = chunk::chunk_text(&extracted.text);

    let mut w = HmcWriter::new(source, &acq.bytes);
    // page count from extraction
    w.manifest_mut().source.page_count = extracted.page_count;

    w.add_derived(
        AssetKind::Text,
        "document.txt",
        "text/plain",
        extracted.text.into_bytes(),
    );
    w.add_derived(
        AssetKind::Html,
        "document.html",
        "text/html",
        extracted.html.into_bytes(),
    );

    let jsonl = chunk::chunks_to_jsonl(&chunks);
    w.add_derived(
        AssetKind::Chunks,
        "chunks.jsonl",
        "application/jsonl",
        jsonl.into_bytes(),
    );

    let ttl = cml::build_cml(w.manifest(), &chunks);
    w.add_derived(
        AssetKind::Cml,
        "document.cml.ttl",
        "text/turtle",
        ttl.into_bytes(),
    );

    w.manifest_mut().pipeline = PipelineInfo {
        tool: format!("qualia-semantic-library {}", env!("CARGO_PKG_VERSION")),
        extractor: extracted.extractor,
        embedder: String::new(),
        embed_dim: 0,
    };
    w.manifest_mut().status.extracted = !chunks.is_empty();
    w.manifest_mut().status.chunked = !chunks.is_empty();
    w.manifest_mut().status.notes = extracted.notes.clone();

    let container_path = w.write_to_dir(&opts.out_dir)?;
    Ok(IngestResult {
        container_path,
        doc_id,
        skipped: false,
        notes: extracted.notes,
    })
}

/// Ingest a file or a directory tree. Returns per-file results.
pub fn ingest_path(path: &Path, opts: &IngestOptions) -> anyhow::Result<Vec<IngestResult>> {
    let mut results = Vec::new();
    if path.is_file() {
        results.push(ingest_file(path, opts)?);
        return Ok(results);
    }

    let walker = WalkDir::new(path).max_depth(if opts.recursive { usize::MAX } else { 1 });
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if !is_ingestible(p) {
            continue;
        }
        // Defence in depth: a panic anywhere in extract/chunk/cml on one
        // document must not abort a 60 GB sweep. Isolate every file.
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ingest_file(p, opts)));
        std::panic::set_hook(prev);
        match outcome {
            Ok(Ok(r)) => results.push(r),
            Ok(Err(e)) => eprintln!("[ingest] FAILED {}: {e}", p.display()),
            Err(_) => eprintln!("[ingest] PANIC (skipped) {}", p.display()),
        }
    }
    Ok(results)
}

fn is_ingestible(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("pdf" | "html" | "htm" | "xhtml" | "txt" | "text" | "md" | "markdown" | "epub")
    )
}
