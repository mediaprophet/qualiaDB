//! Seekable files, HTTP streams, gzip/zstd inflate, and SHA-256 of what Rio sees.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use super::job::append_window_hash;

pub const WINDOW_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestEncoding {
    Identity,
    Gzip,
    Zstd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestRdfFormat {
    Auto,
    Turtle,
    NTriples,
    NQuads,
    TriG,
    RdfXml,
    N3,
    JsonLd,
    CborLd,
    YamlLd,
    RdfJson,
}

impl IngestRdfFormat {
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::NTriples => "nt",
            Self::NQuads => "nq",
            Self::TriG => "trig",
            Self::RdfXml => "rdf",
            Self::N3 => "n3",
            Self::JsonLd => "jsonld",
            Self::CborLd => "cbor",
            Self::YamlLd => "yamlld",
            Self::RdfJson => "rj",
            Self::Turtle | Self::Auto => "ttl",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestSourceKind {
    File { path: String },
    Url { url: String },
}

impl IngestSourceKind {
    pub fn locator(&self) -> &str {
        match self {
            Self::File { path } => path,
            Self::Url { url } => url,
        }
    }

    pub fn is_seekable_identity(&self, encoding: IngestEncoding) -> bool {
        matches!(self, Self::File { .. }) && encoding == IngestEncoding::Identity
    }
}

#[derive(Default)]
pub struct DigestOutcome {
    pub full: std::sync::Mutex<Option<[u8; 32]>>,
    pub bytes: std::sync::atomic::AtomicU64,
    pub windows: std::sync::atomic::AtomicU64,
}

pub struct DigestingReader<R: Read> {
    inner: R,
    hasher: Sha256,
    window: Sha256,
    window_filled: usize,
    uncompressed_bytes: u64,
    windows_path: Option<PathBuf>,
    window_count: u64,
    outcome: Option<std::sync::Arc<DigestOutcome>>,
    finalized: bool,
}

impl<R: Read> DigestingReader<R> {
    pub fn new(inner: R, windows_path: Option<PathBuf>) -> Self {
        Self::with_outcome(inner, windows_path, None)
    }

    pub fn with_outcome(
        inner: R,
        windows_path: Option<PathBuf>,
        outcome: Option<std::sync::Arc<DigestOutcome>>,
    ) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            window: Sha256::new(),
            window_filled: 0,
            uncompressed_bytes: 0,
            windows_path,
            window_count: 0,
            outcome,
            finalized: false,
        }
    }

    pub fn uncompressed_bytes(&self) -> u64 {
        self.uncompressed_bytes
    }

    pub fn window_count(&self) -> u64 {
        self.window_count
    }

    pub fn finish(mut self) -> io::Result<([u8; 32], u64, u64, Option<[u8; 32]>)> {
        self.flush_tail()?;
        let full: [u8; 32] = std::mem::replace(&mut self.hasher, Sha256::new())
            .finalize()
            .into();
        self.publish_outcome(full);
        self.finalized = true;
        Ok((full, self.uncompressed_bytes, self.window_count, None))
    }

    fn publish_outcome(&self, full: [u8; 32]) {
        if let Some(out) = &self.outcome {
            *out.full.lock().unwrap_or_else(|e| e.into_inner()) = Some(full);
            out.bytes.store(
                self.uncompressed_bytes,
                std::sync::atomic::Ordering::Relaxed,
            );
            out.windows
                .store(self.window_count, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn flush_tail(&mut self) -> io::Result<()> {
        if self.window_filled > 0 {
            let digest: [u8; 32] = std::mem::replace(&mut self.window, Sha256::new())
                .finalize()
                .into();
            if let Some(path) = &self.windows_path {
                append_window_hash(path, &digest)?;
            }
            self.window_count += 1;
            self.window_filled = 0;
        }
        Ok(())
    }
}

impl<R: Read> Drop for DigestingReader<R> {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        let _ = self.flush_tail();
        let full: [u8; 32] = std::mem::replace(&mut self.hasher, Sha256::new())
            .finalize()
            .into();
        self.publish_outcome(full);
    }
}

impl<R: Read> Read for DigestingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            return Ok(0);
        }
        let got = &buf[..n];
        self.hasher.update(got);
        self.uncompressed_bytes += n as u64;
        let mut rest = got;
        while !rest.is_empty() {
            let need = WINDOW_BYTES - self.window_filled;
            let take = rest.len().min(need);
            self.window.update(&rest[..take]);
            self.window_filled += take;
            rest = &rest[take..];
            if self.window_filled == WINDOW_BYTES {
                let digest: [u8; 32] = self.window.finalize_reset().into();
                if let Some(path) = &self.windows_path {
                    append_window_hash(path, &digest)?;
                }
                self.window_count += 1;
                self.window_filled = 0;
            }
        }
        Ok(n)
    }
}

pub fn detect_encoding(name: &str, content_encoding: Option<&str>, magic: &[u8]) -> IngestEncoding {
    if let Some(ce) = content_encoding {
        let ce = ce.to_ascii_lowercase();
        if ce.contains("gzip") {
            return IngestEncoding::Gzip;
        }
        if ce.contains("zstd") || ce.contains("zst") {
            return IngestEncoding::Zstd;
        }
    }
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".gz") || lower.ends_with(".tgz") {
        return IngestEncoding::Gzip;
    }
    if lower.ends_with(".zst") || lower.ends_with(".zstd") {
        return IngestEncoding::Zstd;
    }
    if magic.len() >= 2 && magic[0] == 0x1f && magic[1] == 0x8b {
        return IngestEncoding::Gzip;
    }
    if magic.len() >= 4
        && magic[0] == 0x28
        && magic[1] == 0xb5
        && magic[2] == 0x2f
        && magic[3] == 0xfd
    {
        return IngestEncoding::Zstd;
    }
    IngestEncoding::Identity
}

pub fn infer_rdf_format(name: &str) -> IngestRdfFormat {
    let lower = name.to_ascii_lowercase();
    let stem = lower.trim_end_matches(".gz").trim_end_matches(".zst");
    if stem.ends_with(".nt") || stem.ends_with(".ntriples") {
        IngestRdfFormat::NTriples
    } else if stem.ends_with(".nq") || stem.ends_with(".nquads") {
        IngestRdfFormat::NQuads
    } else if stem.ends_with(".trig") {
        IngestRdfFormat::TriG
    } else if stem.ends_with(".rdf") || stem.ends_with(".owl") || stem.ends_with(".xml") {
        IngestRdfFormat::RdfXml
    } else if stem.ends_with(".n3") {
        IngestRdfFormat::N3
    } else if stem.ends_with(".jsonld") || stem.ends_with(".json-ld") || stem.ends_with(".json") {
        IngestRdfFormat::JsonLd
    } else if stem.ends_with(".cbor") || stem.ends_with(".cborld") || stem.ends_with(".cbor-ld") {
        IngestRdfFormat::CborLd
    } else if stem.ends_with(".yamlld")
        || stem.ends_with(".yaml-ld")
        || stem.ends_with(".yml")
        || stem.ends_with(".yaml")
    {
        IngestRdfFormat::YamlLd
    } else if stem.ends_with(".rdfjson") || stem.ends_with(".rj") {
        IngestRdfFormat::RdfJson
    } else {
        IngestRdfFormat::Turtle
    }
}

pub struct OpenedSource {
    pub reader: Box<dyn Read + Send>,
    pub encoding: IngestEncoding,
    pub format: IngestRdfFormat,
    pub kind: IngestSourceKind,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub resumable: bool,
}

pub fn open_ingest_source(
    kind: &IngestSourceKind,
    encoding_hint: Option<IngestEncoding>,
    format_hint: IngestRdfFormat,
) -> io::Result<OpenedSource> {
    match kind {
        IngestSourceKind::File { path } => open_file(Path::new(path), encoding_hint, format_hint),
        IngestSourceKind::Url { url } => open_url(url, encoding_hint, format_hint),
    }
}

fn open_file(
    path: &Path,
    encoding_hint: Option<IngestEncoding>,
    format_hint: IngestRdfFormat,
) -> io::Result<OpenedSource> {
    let file = File::open(path)?;
    let len = file.metadata().ok().map(|m| m.len());
    let mut peek = BufReader::new(file);
    let mut magic = [0u8; 4];
    let n = peek.read(&mut magic)?;
    // Rewind after peek.
    let file = File::open(path)?;
    let name = path.to_string_lossy();
    let encoding = encoding_hint.unwrap_or_else(|| detect_encoding(&name, None, &magic[..n]));
    if matches!(
        encoding_hint,
        None if name.to_ascii_lowercase().ends_with(".bz2")
    ) || name.to_ascii_lowercase().ends_with(".bz2")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bzip2 sources are not streamed yet; use gzip, zstd, or uncompressed",
        ));
    }
    let format = match format_hint {
        IngestRdfFormat::Auto => infer_rdf_format(&name),
        other => other,
    };
    let reader: Box<dyn Read + Send> = wrap_decoder(Box::new(file), encoding)?;
    Ok(OpenedSource {
        reader,
        encoding,
        format,
        kind: IngestSourceKind::File {
            path: path.to_string_lossy().into_owned(),
        },
        content_length: len,
        etag: None,
        resumable: encoding == IngestEncoding::Identity,
    })
}

fn open_url(
    url: &str,
    encoding_hint: Option<IngestEncoding>,
    format_hint: IngestRdfFormat,
) -> io::Result<OpenedSource> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("qualiaDB-ingest/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(io_other)?;
    let resp = client
        .get(url)
        .send()
        .map_err(io_other)?
        .error_for_status()
        .map_err(io_other)?;
    let etag = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let content_encoding = resp
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let content_length = resp.content_length();
    let encoding =
        encoding_hint.unwrap_or_else(|| detect_encoding(url, content_encoding.as_deref(), &[]));
    if url.to_ascii_lowercase().ends_with(".bz2") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bzip2 URL streams are not supported yet; use gzip, zstd, or uncompressed",
        ));
    }
    let format = match format_hint {
        IngestRdfFormat::Auto => infer_rdf_format(url),
        other => other,
    };
    let reader: Box<dyn Read + Send> = wrap_decoder(Box::new(ResponseRead(resp)), encoding)?;
    Ok(OpenedSource {
        reader,
        encoding,
        format,
        kind: IngestSourceKind::Url {
            url: url.to_string(),
        },
        content_length,
        etag,
        resumable: false,
    })
}

fn wrap_decoder(
    inner: Box<dyn Read + Send>,
    encoding: IngestEncoding,
) -> io::Result<Box<dyn Read + Send>> {
    match encoding {
        IngestEncoding::Identity => Ok(inner),
        IngestEncoding::Gzip => Ok(Box::new(flate2::read::GzDecoder::new(inner))),
        IngestEncoding::Zstd => {
            let dec = zstd::stream::read::Decoder::new(inner).map_err(io_other)?;
            Ok(Box::new(dec))
        }
    }
}

/// reqwest 0.13 blocking Response is Read on the default blocking API.
struct ResponseRead(reqwest::blocking::Response);

impl Read for ResponseRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut self.0, buf)
    }
}

fn io_other(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_and_nt_inferred_from_name() {
        assert_eq!(
            detect_encoding("dump.ttl.gz", None, &[]),
            IngestEncoding::Gzip
        );
        assert_eq!(
            infer_rdf_format("https://ex.test/a.nt.gz"),
            IngestRdfFormat::NTriples
        );
        assert_eq!(infer_rdf_format("graph.ttl"), IngestRdfFormat::Turtle);
        assert_eq!(infer_rdf_format("doc.jsonld"), IngestRdfFormat::JsonLd);
        assert_eq!(infer_rdf_format("doc.yamlld"), IngestRdfFormat::YamlLd);
        assert_eq!(infer_rdf_format("doc.rj"), IngestRdfFormat::RdfJson);
        assert_eq!(infer_rdf_format("schema.owl"), IngestRdfFormat::RdfXml);
        assert_eq!(infer_rdf_format("g.nq"), IngestRdfFormat::NQuads);
    }

    #[test]
    fn digesting_reader_hashes_and_windows() {
        let data = vec![7u8; WINDOW_BYTES + 10];
        let dir = tempfile::TempDir::new().unwrap();
        let win = dir.path().join("windows.sha256");
        let mut r = DigestingReader::new(std::io::Cursor::new(data.clone()), Some(win.clone()));
        let mut sink = Vec::new();
        r.read_to_end(&mut sink).unwrap();
        assert_eq!(sink, data);
        let (full, bytes, windows, _) = r.finish().unwrap();
        assert_eq!(bytes, data.len() as u64);
        assert_eq!(windows, 2);
        let expect: [u8; 32] = Sha256::digest(&data).into();
        assert_eq!(full, expect);
        let lines = std::fs::read_to_string(win).unwrap();
        assert_eq!(lines.lines().count(), 2);
    }
}
