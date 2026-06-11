//! Multimodal image ingest — binds assets to active mmproj + WAL (no Ollama).

use std::io::Read;
use std::path::{Path, PathBuf};

use qualia_core_db::{gguf_sharder::GGufSharder, q_hash, wal::WriteAheadLog, NQuin};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model_lifecycle::{self, ActiveModelRecord, InstallManifest};

#[derive(Debug)]
pub enum VisionError {
    NoActiveModel,
    NotMultimodal,
    MissingProjector,
    InactiveLifecycle,
    Io(std::io::Error),
    Wal(String),
    Json(serde_json::Error),
}

impl std::fmt::Display for VisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisionError::NoActiveModel => {
                write!(
                    f,
                    "No active model — activate a multimodal model in LLM Hub"
                )
            }
            VisionError::NotMultimodal => write!(f, "Active model is text-only; install a VLM"),
            VisionError::MissingProjector => {
                write!(f, "Active model is missing mmproj projector path")
            }
            VisionError::InactiveLifecycle => {
                write!(
                    f,
                    "Model lifecycle is not Active — activate model before image ingest"
                )
            }
            VisionError::Io(e) => write!(f, "IO error: {e}"),
            VisionError::Wal(e) => write!(f, "WAL error: {e}"),
            VisionError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl From<std::io::Error> for VisionError {
    fn from(e: std::io::Error) -> Self {
        VisionError::Io(e)
    }
}

impl From<serde_json::Error> for VisionError {
    fn from(e: serde_json::Error) -> Self {
        VisionError::Json(e)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VisionIngestResult {
    pub status: String,
    pub file: String,
    pub typology: String,
    pub lexicon_id: String,
    pub image_sha256: String,
    pub model_id: String,
    pub mmproj_path: String,
    pub architecture: Option<String>,
    pub facet: String,
    pub wal_path: String,
    pub vision_quins_appended: usize,
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

fn facet_for_typology(typology: &str, image_hash: &str, arch: Option<&str>) -> String {
    let arch_label = arch.unwrap_or("vlm");
    match typology {
        "Meme" => format!("{arch_label} meme tensor | irony-bound | sha256:{image_hash}"),
        "Heraldry" => {
            format!("{arch_label} heraldry charge tensor | tincture-bound | sha256:{image_hash}")
        }
        "Clinical" | "DICOM" => {
            format!("{arch_label} clinical imaging facet | sha256:{image_hash}")
        }
        _ => format!("{arch_label} asset facet | typology:{typology} | sha256:{image_hash}"),
    }
}

fn provenance_quin(image_path: &str, typology: &str, timestamp: u64) -> NQuin {
    let subject = q_hash(&format!("vision:{}", image_path));
    let predicate = q_hash("prov:wasDerivedFrom");
    let object = q_hash(typology);
    let context = q_hash("ctx:vision_ingest");
    let metadata = timestamp & 0xFFFF_FFFF;
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

pub fn resolve_active_multimodal(
    storage_root: &Path,
    active: &ActiveModelRecord,
) -> Result<(InstallManifest, PathBuf), VisionError> {
    if active.modality != "multimodal" {
        return Err(VisionError::NotMultimodal);
    }
    if active.lifecycle_state != "Active" {
        return Err(VisionError::InactiveLifecycle);
    }
    let manifest = model_lifecycle::load_install_manifest(storage_root, &active.model_id)
        .ok_or(VisionError::NoActiveModel)?;
    let mmproj = active
        .mmproj_path
        .as_deref()
        .or(manifest.mmproj_path.as_deref())
        .ok_or(VisionError::MissingProjector)?;
    let mmproj_path = PathBuf::from(mmproj);
    if !mmproj_path.is_file() {
        return Err(VisionError::MissingProjector);
    }
    Ok((manifest, mmproj_path))
}

pub fn ingest_image_file(
    storage_root: &Path,
    active: &ActiveModelRecord,
    file_path: &Path,
    typology: &str,
) -> Result<VisionIngestResult, VisionError> {
    let (manifest, mmproj_path) = resolve_active_multimodal(storage_root, active)?;

    if !file_path.is_file() {
        return Err(VisionError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Image not found: {}", file_path.display()),
        )));
    }

    let image_sha256 = sha256_file(file_path)?;
    let lexicon_id = format!("0x{:016X}", q_hash(&image_sha256) & 0xFFFF_FFFF_FFFF_FFFF);

    let mmproj_str = mmproj_path.to_string_lossy().into_owned();
    let vision_quins = GGufSharder::new(mmproj_str).generate_bidx_pointer_map();

    let wal_path = model_lifecycle::models_dir(storage_root).join("vision_ingest.wal");
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| VisionError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let prov = provenance_quin(&file_path.to_string_lossy(), typology, timestamp);
    wal.append_mutation(&prov)
        .map_err(|e| VisionError::Wal(e.to_string()))?;

    for q in &vision_quins {
        wal.append_mutation(q)
            .map_err(|e| VisionError::Wal(e.to_string()))?;
    }

    let facet = facet_for_typology(typology, &image_sha256, manifest.architecture.as_deref());

    Ok(VisionIngestResult {
        status: "success".to_string(),
        file: file_path.to_string_lossy().into_owned(),
        typology: typology.to_string(),
        lexicon_id,
        image_sha256,
        model_id: active.model_id.clone(),
        mmproj_path: mmproj_path.to_string_lossy().into_owned(),
        architecture: manifest.architecture.clone(),
        facet,
        wal_path: wal_path.to_string_lossy().into_owned(),
        vision_quins_appended: vision_quins.len(),
    })
}

pub fn ingest_image_with_active_record(
    storage_root: &Path,
    active: Option<ActiveModelRecord>,
    file_path: &Path,
    typology: &str,
) -> Result<VisionIngestResult, VisionError> {
    let active = active.ok_or(VisionError::NoActiveModel)?;
    ingest_image_file(storage_root, &active, file_path, typology)
}
