//! Stage 1 — acquire: read a file, establish its identity (BLAKE3), and detect
//! its source kind. No extraction yet; this stage only produces the verbatim
//! bytes + a [`SourceInfo`] the rest of the pipeline keys on.

use std::path::Path;

use crate::container::{blake3_hex, SourceInfo};

/// What kind of source we are looking at — drives extractor selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Pdf,
    Html,
    Text,
    Markdown,
    Epub,
    Unknown,
}

impl SourceKind {
    pub fn mime(self) -> &'static str {
        match self {
            SourceKind::Pdf => "application/pdf",
            SourceKind::Html => "text/html",
            SourceKind::Text => "text/plain",
            SourceKind::Markdown => "text/markdown",
            SourceKind::Epub => "application/epub+zip",
            SourceKind::Unknown => "application/octet-stream",
        }
    }
}

/// The verbatim source plus its derived identity/description.
pub struct Acquired {
    pub bytes: Vec<u8>,
    pub kind: SourceKind,
    pub source: SourceInfo,
}

/// Best-effort source-kind detection from magic bytes first, extension second.
pub fn detect_kind(path: &Path, bytes: &[u8]) -> SourceKind {
    if bytes.starts_with(b"%PDF-") {
        return SourceKind::Pdf;
    }
    // EPUB / OOXML are zips; only call EPUB if the extension says so.
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => SourceKind::Pdf,
        "html" | "htm" | "xhtml" => SourceKind::Html,
        "md" | "markdown" => SourceKind::Markdown,
        "txt" | "text" => SourceKind::Text,
        "epub" => SourceKind::Epub,
        _ => {
            // Sniff for an HTML doctype/tag near the start.
            let head = &bytes[..bytes.len().min(512)];
            let lower = String::from_utf8_lossy(head).to_ascii_lowercase();
            if lower.contains("<!doctype html") || lower.contains("<html") {
                SourceKind::Html
            } else if bytes.is_empty() {
                SourceKind::Unknown
            } else if head
                .iter()
                .all(|&b| b == b'\n' || b == b'\r' || b == b'\t' || b >= 0x20)
            {
                SourceKind::Text
            } else {
                SourceKind::Unknown
            }
        }
    }
}

pub fn acquire(path: &Path) -> std::io::Result<Acquired> {
    let bytes = std::fs::read(path)?;
    let kind = detect_kind(path, &bytes);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string();
    let title = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let source = SourceInfo {
        filename,
        mime: kind.mime().to_string(),
        size_bytes: bytes.len() as u64,
        blake3: blake3_hex(&bytes),
        title,
        page_count: 0,
    };
    Ok(Acquired {
        bytes,
        kind,
        source,
    })
}
