//! Multimodal image ingest — binds assets to active mmproj + WAL (no Ollama).
//!
//! V6: also accepts **native detector** epistemic observation quins (no mmproj required)
//! via `append_native_observation_quins` — machine claims with model digest + optional
//! human reject/correct edges. Dense pixels never enter NQuins.

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
    /// Caller buffer / detection packing error.
    Buffer(String),
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
            VisionError::Buffer(e) => write!(f, "Buffer error: {e}"),
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

// ── V6 native detector / epistemic observation path (no mmproj) ─────────────

/// Fixed-layout detection for WAL packing (mirrors `qualia_vision::Detection` fields).
#[derive(Debug, Clone, Copy)]
pub struct NativeDetection {
    pub class_hash: u64,
    pub instance_hash: u64,
    pub score_u16: u16,
    pub x_min_u16: u16,
    pub y_min_u16: u16,
    pub x_max_u16: u16,
    pub y_max_u16: u16,
    pub frame_index: u32,
    pub track_id: u32,
    pub flags: u32,
}

const P_VISUAL_OBSERVATION: &str = "https://ns.webizen.org/q42/VisualObservation";
const P_PROPOSES_CLASS: &str = "https://ns.webizen.org/q42/proposesClass";
const P_HAS_BBOX: &str = "https://ns.webizen.org/q42/hasBoundingBox";
const P_HAS_TRACK: &str = "https://ns.webizen.org/q42/hasTrackId";
const P_MODEL_DIGEST: &str = "https://ns.webizen.org/q42/modelDigest";
const P_HUMAN_REJECTS: &str = "https://ns.webizen.org/q42/humanRejects";
const P_HUMAN_CORRECTS: &str = "https://ns.webizen.org/q42/humanCorrectsClass";
const CTX_VISION: &str = "https://ns.webizen.org/q42/vision-observation";
const CTX_HUMAN: &str = "https://ns.webizen.org/q42/human-attestation";

#[inline]
fn quin(s: u64, p: u64, o: u64, c: u64, m: u64) -> NQuin {
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: m,
        parity: s ^ p ^ o ^ c ^ m,
    }
}

#[inline]
fn pack_bbox(d: &NativeDetection) -> u64 {
    (d.x_min_u16 as u64)
        | ((d.y_min_u16 as u64) << 16)
        | ((d.x_max_u16 as u64) << 32)
        | ((d.y_max_u16 as u64) << 48)
}

/// Compile native detections into epistemic NQuins (caller buffer).
/// Writes model digest + per-det observation/class/bbox/track. Returns count.
pub fn compile_native_observation_quins(
    media_hash: u64,
    media_byte_len: u64,
    model_hash: u64,
    detections: &[NativeDetection],
    out: &mut [NQuin],
) -> Result<usize, VisionError> {
    if out.is_empty() {
        return Err(VisionError::Buffer("empty out".into()));
    }
    let mut w = 0usize;
    out[w] = quin(
        media_hash,
        q_hash(P_MODEL_DIGEST),
        model_hash,
        q_hash(CTX_VISION),
        media_byte_len,
    );
    w += 1;
    let ctx = q_hash(CTX_VISION) ^ model_hash;
    for d in detections {
        if d.class_hash == 0 && d.score_u16 == 0 {
            continue;
        }
        if w + 4 > out.len() {
            break;
        }
        let meta_score = (d.score_u16 as u64)
            | ((d.frame_index as u64) << 16)
            | ((d.flags as u64) << 48);
        out[w] = quin(
            media_hash,
            q_hash(P_VISUAL_OBSERVATION),
            d.instance_hash,
            ctx,
            meta_score,
        );
        w += 1;
        out[w] = quin(
            d.instance_hash,
            q_hash(P_PROPOSES_CLASS),
            d.class_hash,
            ctx,
            d.score_u16 as u64,
        );
        w += 1;
        out[w] = quin(
            d.instance_hash,
            q_hash(P_HAS_BBOX),
            pack_bbox(d),
            ctx,
            d.score_u16 as u64 | ((d.frame_index as u64) << 16),
        );
        w += 1;
        out[w] = quin(
            d.instance_hash,
            q_hash(P_HAS_TRACK),
            d.track_id as u64,
            ctx,
            d.frame_index as u64,
        );
        w += 1;
    }
    Ok(w)
}

/// Human reject of a machine instance — does **not** erase machine quins.
pub fn human_reject_quin(human_did_hash: u64, instance_hash: u64, reason_hash: u64) -> NQuin {
    quin(
        human_did_hash,
        q_hash(P_HUMAN_REJECTS),
        instance_hash,
        q_hash(CTX_HUMAN),
        reason_hash,
    )
}

/// Human class correction — coexists with machine `proposesClass`.
pub fn human_correct_quin(human_did_hash: u64, instance_hash: u64, new_class_hash: u64) -> NQuin {
    quin(
        human_did_hash,
        q_hash(P_HUMAN_CORRECTS),
        new_class_hash,
        q_hash(CTX_HUMAN) ^ instance_hash,
        instance_hash,
    )
}

/// Append native observation quins to `vision_native.wal` (no active VLM required).
pub fn append_native_observation_quins(
    storage_root: &Path,
    media_hash: u64,
    media_byte_len: u64,
    model_hash: u64,
    detections: &[NativeDetection],
) -> Result<usize, VisionError> {
    let mut buf = [NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    }; 256];
    let n = compile_native_observation_quins(
        media_hash,
        media_byte_len,
        model_hash,
        detections,
        &mut buf,
    )?;
    let wal_path = model_lifecycle::models_dir(storage_root).join("vision_native.wal");
    if let Some(parent) = wal_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| VisionError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;
    for q in buf.iter().take(n) {
        wal.append_mutation(q)
            .map_err(|e| VisionError::Wal(e.to_string()))?;
    }
    Ok(n)
}

/// Append a human reject/correct edge (provenance preserved).
pub fn append_human_attestation(
    storage_root: &Path,
    quin: &NQuin,
) -> Result<(), VisionError> {
    let wal_path = model_lifecycle::models_dir(storage_root).join("vision_native.wal");
    if let Some(parent) = wal_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| VisionError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;
    wal.append_mutation(quin)
        .map_err(|e| VisionError::Wal(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_compile_four_per_det_plus_digest() {
        let d = NativeDetection {
            class_hash: 1,
            instance_hash: 2,
            score_u16: 1000,
            x_min_u16: 0,
            y_min_u16: 0,
            x_max_u16: 100,
            y_max_u16: 100,
            frame_index: 0,
            track_id: 3,
            flags: 0,
        };
        let mut out = [NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 16];
        let n = compile_native_observation_quins(9, 64, 7, &[d], &mut out).unwrap();
        assert_eq!(n, 5);
        assert_eq!(out[0].predicate, q_hash(P_MODEL_DIGEST));
        let rej = human_reject_quin(0xD1D, 2, 0);
        assert_eq!(rej.predicate, q_hash(P_HUMAN_REJECTS));
        // Machine class proposal still in buffer.
        assert!(out[..n]
            .iter()
            .any(|q| q.predicate == q_hash(P_PROPOSES_CLASS)));
    }
}
