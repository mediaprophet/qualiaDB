//! Catalog ontology download → `.q42` import pipeline (in-process, no subprocess).
//!
//! Mirrors `qualia-cli resources import-ontology` but writes under `{storage}/Index/`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::{
    ingest,
    resource_catalog::{OntologyResource, ResourceCatalog},
    wal::WriteAheadLog,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum ImportError {
    NotFound(String),
    NoDownloadUrl(String),
    Download(String),
    Ingest(String),
    Wal(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::NotFound(id) => write!(f, "Ontology not found in catalog: {id}"),
            ImportError::NoDownloadUrl(id) => write!(f, "No download URL for ontology: {id}"),
            ImportError::Download(e) => write!(f, "Download failed: {e}"),
            ImportError::Ingest(e) => write!(f, "Ingest failed: {e}"),
            ImportError::Wal(e) => write!(f, "WAL write failed: {e}"),
            ImportError::Io(e) => write!(f, "IO error: {e}"),
            ImportError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e)
    }
}

impl From<serde_json::Error> for ImportError {
    fn from(e: serde_json::Error) -> Self {
        ImportError::Json(e)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyImportResult {
    pub ontology_id: String,
    pub source_path: String,
    pub q42_path: String,
    pub wal_path: String,
    pub quin_count: u64,
    pub catalog_quins: usize,
    pub sha256: String,
    pub imported_at: u64,
}

#[derive(Serialize)]
struct OntologyMetaSidecar {
    ontology_id: String,
    quin_count: u64,
    sha256: String,
    imported_at: u64,
    source_path: String,
    q42_path: String,
}

pub fn index_dir(storage_root: &Path) -> PathBuf {
    storage_root.join("Index")
}

fn ontology_source_filename(ont: &OntologyResource) -> String {
    ont.download
        .local_filename()
        .unwrap_or_else(|| format!("{}.{}", ont.id, ont.format))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn write_meta_sidecar(
    ontology_id: &str,
    q42_path: &Path,
    source_path: &Path,
    quin_count: u64,
    sha256: &str,
    imported_at: u64,
) -> Result<(), ImportError> {
    let meta_path = q42_path.with_extension("q42.meta.json");
    let meta = OntologyMetaSidecar {
        ontology_id: ontology_id.to_string(),
        quin_count,
        sha256: sha256.to_string(),
        imported_at,
        source_path: source_path.to_string_lossy().into_owned(),
        q42_path: q42_path.to_string_lossy().into_owned(),
    };
    let json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(&meta_path, json)?;
    Ok(())
}

pub async fn stream_download(url: &str, dest: &Path) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(
            "User-Agent",
            concat!("qualiaDB-client/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("Cannot create {}: {}", dest.display(), e))?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Ingest a local RDF file into `{storage}/Index/{ontology_id}.q42`.
pub fn ingest_local_rdf(
    source_path: &Path,
    ontology_id: &str,
    storage_root: &Path,
    ont: Option<&OntologyResource>,
) -> Result<u64, ImportError> {
    let index = index_dir(storage_root);
    std::fs::create_dir_all(&index)?;

    let q42_path = index.join(format!("{ontology_id}.q42"));
    let in_str = source_path.to_string_lossy().into_owned();
    let out_str = q42_path.to_string_lossy().into_owned();

    let quin_count = ingest::streaming_import_rdf(&in_str, &out_str)
        .map_err(|e| ImportError::Ingest(e.to_string()))?;

    let imported_at = unix_now();
    let sha256 = sha256_file(&q42_path)?;

    if let Some(ont) = ont {
        append_ontology_wal(storage_root, ont, &q42_path, imported_at)?;
    }

    write_meta_sidecar(
        ontology_id,
        &q42_path,
        source_path,
        quin_count,
        &sha256,
        imported_at,
    )?;

    Ok(quin_count)
}

fn append_ontology_wal(
    storage_root: &Path,
    ont: &OntologyResource,
    q42_path: &Path,
    timestamp: u64,
) -> Result<usize, ImportError> {
    let wal_path = index_dir(storage_root).join("ontologies.wal");
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| ImportError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;

    let prov = ont.provenance_quin(timestamp, &q42_path.to_string_lossy());
    wal.append_mutation(&prov)
        .map_err(|e| ImportError::Wal(e.to_string()))?;

    let catalog_quins = ont.to_quins();
    for q in &catalog_quins {
        wal.append_mutation(q)
            .map_err(|e| ImportError::Wal(e.to_string()))?;
    }

    Ok(catalog_quins.len())
}

/// Download a catalog ontology and compile it to `.q42` under `{storage}/Index/`.
pub async fn import_catalog_ontology(
    catalog: &ResourceCatalog,
    id: &str,
    storage_root: &Path,
) -> Result<OntologyImportResult, ImportError> {
    let ont = catalog
        .find_ontology(id)
        .ok_or_else(|| ImportError::NotFound(id.to_string()))?;

    let url = ont
        .download
        .resolved_url()
        .ok_or_else(|| ImportError::NoDownloadUrl(id.to_string()))?;

    let index = index_dir(storage_root);
    std::fs::create_dir_all(&index)?;

    let filename = ontology_source_filename(ont);
    let source_path = index.join(&filename);

    stream_download(&url, &source_path)
        .await
        .map_err(ImportError::Download)?;

    let quin_count = ingest_local_rdf(&source_path, id, storage_root, Some(ont))?;
    let q42_path = index.join(format!("{id}.q42"));
    let wal_path = index.join("ontologies.wal");
    let catalog_quins = ont.to_quins().len();
    let sha256 = sha256_file(&q42_path)?;
    let imported_at = unix_now();

    Ok(OntologyImportResult {
        ontology_id: id.to_string(),
        source_path: source_path.to_string_lossy().into_owned(),
        q42_path: q42_path.to_string_lossy().into_owned(),
        wal_path: wal_path.to_string_lossy().into_owned(),
        quin_count,
        catalog_quins,
        sha256,
        imported_at,
    })
}
