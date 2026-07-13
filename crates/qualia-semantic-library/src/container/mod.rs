//! The `.hmc` **hypermedia container**: a single, fast-compression (ZIP) file
//! that bundles an original source document together with all of its derived
//! assets (canonical HTML, CML/RDF, plain text, structural chunks, embeddings,
//! extraction metadata) behind one self-describing [`HmcManifest`].
//!
//! Why a container per document:
//!   * the original and everything derived from it travel together — provenance
//!     never gets separated from the source;
//!   * ZIP gives random access, partial reads, and is readable by every tool;
//!   * re-running the pipeline updates derived assets in place without touching
//!     the verbatim source.
//!
//! Layout inside the zip:
//! ```text
//! manifest.json              # the index (HmcManifest)
//! source/<original>          # verbatim source bytes
//! derived/document.html      # canonical structured HTML
//! derived/document.txt       # plain text with [[page N]] markers
//! derived/document.cml.ttl   # CML / RDF annotations (optional)
//! derived/chunks.jsonl       # structural chunks (optional)
//! embeddings/vectors.f32     # row-major f32 embedding matrix (optional)
//! meta/extraction.json       # tool versions / timings / confidence (optional)
//! ```

mod manifest;
pub use manifest::*;

use std::io::{Cursor, Read, Write};
use std::path::Path;

use thiserror::Error;
use zip::write::SimpleFileOptions;

/// Conventional file extension for a hypermedia container.
pub const HMC_EXTENSION: &str = "hmc";
const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Error)]
pub enum HmcError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("container has no {MANIFEST_NAME}")]
    MissingManifest,
    #[error("asset not found: {0}")]
    AssetNotFound(String),
    #[error("integrity: asset {path} blake3 mismatch (manifest {want}, actual {got})")]
    Integrity { path: String, want: String, got: String },
    #[error("invalid container: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, HmcError>;

/// BLAKE3 hex of a byte slice — the hashing used throughout the container.
pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Builder that accumulates assets in memory and writes a `.hmc` zip in one shot.
///
/// Kept in-memory because individual documents are small relative to RAM and a
/// single atomic write avoids half-written containers on crash/interrupt.
pub struct HmcWriter {
    manifest: HmcManifest,
    /// (path inside zip, bytes)
    members: Vec<(String, Vec<u8>)>,
}

impl HmcWriter {
    /// Start a container from the (already-hashed) source description and bytes.
    pub fn new(mut source: SourceInfo, source_bytes: &[u8]) -> Self {
        // Ensure the source hash is authoritative.
        source.blake3 = blake3_hex(source_bytes);
        source.size_bytes = source_bytes.len() as u64;
        let manifest = HmcManifest::new(source.clone());

        let mut w = HmcWriter { manifest, members: Vec::new() };
        let path = format!("{}/{}", AssetKind::Source.dir(), sanitize(&source.filename));
        w.push_asset(AssetKind::Source, &path, &source.mime.clone(), source_bytes.to_vec());
        w
    }

    fn push_asset(&mut self, kind: AssetKind, path: &str, mime: &str, bytes: Vec<u8>) {
        let entry = AssetEntry {
            path: path.to_string(),
            kind,
            mime: mime.to_string(),
            blake3: blake3_hex(&bytes),
            bytes: bytes.len() as u64,
        };
        // Replace an existing asset at the same path (idempotent re-runs).
        if let Some(i) = self.manifest.assets.iter().position(|a| a.path == path) {
            self.manifest.assets[i] = entry;
            if let Some(m) = self.members.iter_mut().find(|(p, _)| p == path) {
                m.1 = bytes;
                return;
            }
        } else {
            self.manifest.assets.push(entry);
        }
        self.members.push((path.to_string(), bytes));
    }

    /// Add a derived asset under its conventional directory using `name`
    /// (e.g. `document.html`). Returns the in-zip path.
    pub fn add_derived(&mut self, kind: AssetKind, name: &str, mime: &str, bytes: Vec<u8>) -> String {
        let path = format!("{}/{}", kind.dir(), sanitize(name));
        self.push_asset(kind, &path, mime, bytes);
        path
    }

    /// Mutable access to the manifest to set pipeline/status/tags before finalize.
    pub fn manifest_mut(&mut self) -> &mut HmcManifest {
        &mut self.manifest
    }

    /// Serialize the manifest + all members into ZIP bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let stored = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let deflated = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // Manifest first, uncompressed, for instant header reads.
            let manifest_json = serde_json::to_vec_pretty(&self.manifest)?;
            zw.start_file(MANIFEST_NAME, stored)?;
            zw.write_all(&manifest_json)?;

            for (path, bytes) in &self.members {
                // Don't waste CPU re-compressing the source if it's already a
                // compressed format (PDF); deflate the text-like derived assets.
                let already_compressed = path.starts_with("source/")
                    && is_precompressed_mime(self.member_mime(path));
                let opts = if already_compressed { stored } else { deflated };
                zw.start_file(path.as_str(), opts)?;
                zw.write_all(bytes)?;
            }
            zw.finish()?;
        }
        Ok(buf.into_inner())
    }

    fn member_mime(&self, path: &str) -> &str {
        self.manifest
            .assets
            .iter()
            .find(|a| a.path == path)
            .map(|a| a.mime.as_str())
            .unwrap_or("application/octet-stream")
    }

    /// Write the container to `dir/<doc_id>.hmc` and return the path.
    pub fn write_to_dir(&self, dir: &Path) -> Result<std::path::PathBuf> {
        std::fs::create_dir_all(dir)?;
        let out = dir.join(format!("{}.{}", self.manifest.doc_id, HMC_EXTENSION));
        std::fs::write(&out, self.to_bytes()?)?;
        Ok(out)
    }

    pub fn manifest(&self) -> &HmcManifest {
        &self.manifest
    }

    /// Load an existing container back into a writer so an enrichment pass
    /// (embeddings, analysis tags, CML) can add/replace derived assets and
    /// re-emit the container in place. The verbatim source is preserved.
    pub fn reopen(path: &Path) -> Result<Self> {
        let mut c = HmcContainer::open(path)?;
        let manifest = c.manifest().clone();
        let mut members = Vec::with_capacity(manifest.assets.len());
        for a in &manifest.assets {
            members.push((a.path.clone(), c.read_asset_bytes(&a.path)?));
        }
        Ok(HmcWriter { manifest, members })
    }
}

/// Read side: opens a `.hmc`, exposes the manifest, and reads assets on demand.
pub struct HmcContainer {
    manifest: HmcManifest,
    archive: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl HmcContainer {
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        let manifest: HmcManifest = {
            let mut f = archive
                .by_name(MANIFEST_NAME)
                .map_err(|_| HmcError::MissingManifest)?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            serde_json::from_str(&s)?
        };
        if manifest.format != "hmc" {
            return Err(HmcError::Invalid(format!("unexpected format `{}`", manifest.format)));
        }
        Ok(HmcContainer { manifest, archive })
    }

    pub fn manifest(&self) -> &HmcManifest {
        &self.manifest
    }

    /// Raw bytes of a stored member by its in-zip path.
    pub fn read_asset_bytes(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut f = self
            .archive
            .by_name(path)
            .map_err(|_| HmcError::AssetNotFound(path.to_string()))?;
        let mut out = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut out)?;
        Ok(out)
    }

    /// Bytes of the first asset of a given kind.
    pub fn read_kind(&mut self, kind: AssetKind) -> Result<Vec<u8>> {
        let path = self
            .manifest
            .asset_of(kind)
            .map(|a| a.path.clone())
            .ok_or_else(|| HmcError::AssetNotFound(format!("kind {:?}", kind)))?;
        self.read_asset_bytes(&path)
    }

    /// Verify every asset's stored bytes hash to the value recorded in the
    /// manifest. Returns `Ok(())` on a clean container.
    pub fn verify(&mut self) -> Result<()> {
        let entries: Vec<(String, String)> = self
            .manifest
            .assets
            .iter()
            .map(|a| (a.path.clone(), a.blake3.clone()))
            .collect();
        for (path, want) in entries {
            let got = blake3_hex(&self.read_asset_bytes(&path)?);
            if got != want {
                return Err(HmcError::Integrity { path, want, got });
            }
        }
        Ok(())
    }
}

/// MIME types whose bytes are already compressed — re-deflating wastes CPU.
fn is_precompressed_mime(mime: &str) -> bool {
    matches!(
        mime,
        "application/pdf"
            | "image/png"
            | "image/jpeg"
            | "image/webp"
            | "application/zip"
            | "application/epub+zip"
    )
}

/// Make a filename safe as a zip member path component (no traversal, no
/// backslashes, no leading slash).
fn sanitize(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim()
        .replace('\u{0}', "");
    if base.is_empty() {
        "unnamed".to_string()
    } else {
        base
    }
}

#[cfg(test)]
mod tests;
