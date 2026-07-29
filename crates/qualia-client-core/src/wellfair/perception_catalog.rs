//! Perception assets catalogue — seed **models** and **ontologies** into the
//! hypermedia Library (Software / Tools shelves) with honest labels.
//!
//! Pattern mirrors `qapp_catalog.rs`. Does **not** claim foundation-model quality
//! or A4 FEA; seed weights and reference paths are explicitly labelled.

use super::hypermedia_store::{CommonsVisibility, HypermediaStore, LibraryEntry, LibrarySection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Media type for on-device model weight manifests.
pub const MODEL_MEDIA_TYPE: &str = "application/x-webizen-model-manifest";
/// Media type for ontology catalogue rows.
pub const ONTOLOGY_MEDIA_TYPE: &str = "application/x-webizen-ontology";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    pub id: &'static str,
    pub title: &'static str,
    /// Relative path under `{storage}/models/` when materialised.
    pub rel_path: &'static str,
    pub role: &'static str,
    /// If true, UI must not present as foundation / certified production model.
    pub is_seed_reference: bool,
    pub category: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OntologyCatalogEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub rel_path: &'static str,
    pub domain: &'static str,
    pub licence_note: &'static str,
}

/// Seed / reference perception models (paths materialised by ensure_*_weights).
pub const PERCEPTION_MODEL_CATALOG: &[ModelCatalogEntry] = &[
    ModelCatalogEntry {
        id: "vision-seed-qvwt",
        title: "Vision seed QVWT (detector head)",
        rel_path: "models/vision_seed.qvwt",
        role: "vision-encoder-detector",
        is_seed_reference: true,
        category: "vision",
    },
    // MIG-V4 — specialized_libs::computer_vision (algorithm library, not a weight file)
    ModelCatalogEntry {
        id: "lib-computer-vision-cv",
        title: "computer_vision::cv (classical filters/edges/features)",
        rel_path: "crates/qualia-core-db/src/specialized_libs/computer_vision/cv/",
        role: "specialized-lib-cv",
        is_seed_reference: false,
        category: "vision-library",
    },
    ModelCatalogEntry {
        id: "lib-computer-vision-sr",
        title: "computer_vision::sr (classical + tiled SR + device policy)",
        rel_path: "crates/qualia-core-db/src/specialized_libs/computer_vision/sr/",
        role: "specialized-lib-sr",
        is_seed_reference: false,
        category: "vision-library",
    },
    ModelCatalogEntry {
        id: "lib-computer-vision-bio",
        title: "computer_vision::bio (histo/radiomics/DICOM-lite)",
        rel_path: "crates/qualia-core-db/src/specialized_libs/computer_vision/bio/",
        role: "specialized-lib-bio",
        is_seed_reference: false,
        category: "vision-library",
    },
    ModelCatalogEntry {
        id: "lib-computer-vision-spatial",
        title: "computer_vision::spatial (MeshIR export/quality/σ)",
        rel_path: "crates/qualia-core-db/src/specialized_libs/computer_vision/spatial/",
        role: "specialized-lib-spatial",
        is_seed_reference: false,
        category: "vision-library",
    },
    ModelCatalogEntry {
        id: "audio-seed-aed",
        title: "Audio seed AED (.qaed)",
        rel_path: "models/aed_seed.qaed",
        role: "auditory-event-detection",
        is_seed_reference: true,
        category: "audio",
    },
    ModelCatalogEntry {
        id: "audio-seed-speech",
        title: "Speech seed phones (.qspk)",
        rel_path: "models/speech_seed.qspk",
        role: "speech-encoder-phones",
        is_seed_reference: true,
        category: "audio",
    },
    ModelCatalogEntry {
        id: "llm-gguf-slot",
        title: "Local GGUF slot (user-supplied)",
        rel_path: "models/",
        role: "llm-gguf",
        is_seed_reference: false,
        category: "language",
    },
    ModelCatalogEntry {
        id: "vision-yunet-face",
        title: "YuNet 2D Face Detector (MIT)",
        rel_path: "models/vision/face/yunet/face_detection_yunet_2023mar.onnx",
        role: "vision-face-detector",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "vision-sface-embed",
        title: "SFace Biometric Embeddings (Apache-2.0)",
        rel_path: "models/vision/face/sface/face_recognition_sface_2021dec.onnx",
        role: "vision-face-embedding",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "vision-mediapipe-facemesh",
        title: "MediaPipe 468-pt Face Mesh (Apache-2.0)",
        rel_path: "models/vision/face/mediapipe_landmarker/face_landmarker.task",
        role: "vision-face-landmarks",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "vision-midas-depth",
        title: "MiDaS 3D Monocular Depth (MIT)",
        rel_path: "models/vision/depth/midas_v21_small.onnx",
        role: "vision-monocular-depth",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "vision-espcn-sr",
        title: "ESPCN Image Super-Resolution 2x/4x (Apache-2.0)",
        rel_path: "models/vision/sr/espcn/espcn_x4.onnx",
        role: "vision-super-resolution",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "vision-yolonas-object",
        title: "YOLO-NAS Object Detector (Apache-2.0)",
        rel_path: "models/vision/detect/yolo_nas/yolo_nas_s.onnx",
        role: "vision-object-detector",
        is_seed_reference: false,
        category: "vision",
    },
    ModelCatalogEntry {
        id: "audio-chatterbox-voice",
        title: "Resemble AI Chatterbox Voice & TTS (Apache-2.0 / MIT)",
        rel_path: "models/audio/tts/chatterbox/chatterbox.onnx",
        role: "audio-voice-cloning-tts",
        is_seed_reference: false,
        category: "audio",
    },
    ModelCatalogEntry {
        id: "audio-kokoro-82m-tts",
        title: "Kokoro-82M Ultra-Lightweight TTS (Apache-2.0)",
        rel_path: "models/audio/tts/kokoro/kokoro_v0_88.onnx",
        role: "audio-fast-tts",
        is_seed_reference: false,
        category: "audio",
    },
    ModelCatalogEntry {
        id: "audio-f5tts-clone",
        title: "F5-TTS Flow-Matching Voice Cloner (MIT)",
        rel_path: "models/audio/tts/f5tts/f5tts_small.onnx",
        role: "audio-voice-cloning",
        is_seed_reference: false,
        category: "audio",
    },
];

/// Ontologies exposed in Library (subset of `bundled/ontologies` + perception-relevant).
pub const PERCEPTION_ONTOLOGY_CATALOG: &[OntologyCatalogEntry] = &[
    OntologyCatalogEntry {
        id: "shacl",
        title: "SHACL",
        rel_path: "bundled/ontologies/shacl.ttl",
        domain: "validation",
        licence_note: "W3C SHACL",
    },
    OntologyCatalogEntry {
        id: "ldp",
        title: "Linked Data Platform (LDP)",
        rel_path: "bundled/ontologies/w3c-archives/ldp.ttl",
        domain: "solid",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "acl",
        title: "Web Access Control (ACL)",
        rel_path: "bundled/ontologies/w3c-archives/auth-acl.ttl",
        domain: "solid",
        licence_note: "W3C / Solid",
    },
    OntologyCatalogEntry {
        id: "solid-terms",
        title: "Solid Terms",
        rel_path: "bundled/ontologies/w3c-archives/solid-terms.ttl",
        domain: "solid",
        licence_note: "Solid",
    },
    OntologyCatalogEntry {
        id: "solid-oidc",
        title: "Solid-OIDC",
        rel_path: "bundled/ontologies/w3c-archives/solid-oidc.ttl",
        domain: "solid",
        licence_note: "Solid",
    },
    OntologyCatalogEntry {
        id: "pim-space",
        title: "PIM Space",
        rel_path: "bundled/ontologies/w3c-archives/pim-space.ttl",
        domain: "solid",
        licence_note: "Solid",
    },
    OntologyCatalogEntry {
        id: "foaf",
        title: "FOAF",
        rel_path: "bundled/ontologies/w3c-archives/foaf.ttl",
        domain: "social",
        licence_note: "FOAF",
    },
    OntologyCatalogEntry {
        id: "prov",
        title: "PROV-O",
        rel_path: "bundled/ontologies/w3c/prov.ttl",
        domain: "provenance",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "skos",
        title: "SKOS",
        rel_path: "bundled/ontologies/w3c/skos.ttl",
        domain: "vocab",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "dcterms",
        title: "Dublin Core Terms",
        rel_path: "bundled/ontologies/purl/dcterms.ttl",
        domain: "metadata",
        licence_note: "Dublin Core",
    },
    OntologyCatalogEntry {
        id: "owl",
        title: "OWL",
        rel_path: "bundled/ontologies/w3c/owl.ttl",
        domain: "logic",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "rdfs",
        title: "RDFS",
        rel_path: "bundled/ontologies/w3c/rdfs.ttl",
        domain: "logic",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "time",
        title: "OWL-Time",
        rel_path: "bundled/ontologies/w3c/time.ttl",
        domain: "temporal",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "sosa",
        title: "SOSA (sensors)",
        rel_path: "bundled/ontologies/w3c/sosa.ttl",
        domain: "sensing",
        licence_note: "W3C",
    },
    OntologyCatalogEntry {
        id: "music",
        title: "Music Ontology (purl)",
        rel_path: "bundled/ontologies/purl/music.ttl",
        domain: "audio",
        licence_note: "Music Ontology",
    },
    OntologyCatalogEntry {
        id: "consent",
        title: "Consent ontology (purl)",
        rel_path: "bundled/ontologies/purl/consent.ttl",
        domain: "rights",
        licence_note: "bundled purl",
    },
];

fn primary_subject_for_uri(uri: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in uri.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn model_to_library(m: ModelCatalogEntry, storage_root: &Path, now: u64) -> LibraryEntry {
    let uri = format!("model://webizen/{}", m.id);
    let abs = storage_root.join(m.rel_path);
    let on_disk = abs.is_file() || (m.rel_path.ends_with('/') && abs.is_dir());
    let honesty = if m.is_seed_reference {
        "SEED/REFERENCE — not a certified foundation model. Do not use for production accuracy claims."
    } else {
        "User-supplied path slot — activate only licence-compatible weights you control."
    };
    let mut le = LibraryEntry {
        asset_uri: uri.clone(),
        primary_subject: primary_subject_for_uri(&uri),
        media_type: MODEL_MEDIA_TYPE.into(),
        quins: Vec::new(),
        topics: vec![
            "model".into(),
            m.category.into(),
            m.role.into(),
            if m.is_seed_reference {
                "seed-reference".into()
            } else {
                "user-slot".into()
            },
        ],
        projects: vec![format!("perception:{}", m.category)],
        purposes: vec!["model".into(), "software".into(), "perception".into()],
        place: Some(abs.display().to_string()),
        occurred_at: None,
        lat: None,
        lon: None,
        flags: if m.is_seed_reference {
            vec![super::hypermedia_store::LibraryFlag {
                kind: "honesty:seed_reference".into(),
                severity_level: 1,
                detail: honesty.into(),
            }]
        } else {
            Vec::new()
        },
        ingested_unix: now,
        excerpt: format!(
            "{} — role={} path={} on_disk={} — {}",
            m.title, m.role, m.rel_path, on_disk, honesty
        ),
        sensitivity: "public".into(),
        section: LibrarySection::Software.as_str().into(),
        commons_visibility: CommonsVisibility::None,
        cml_signals: Vec::new(),
        cml_concept_count: 0,
        cml_n3: String::new(),
        cof_html: String::new(),
        cof_segment_count: 0,
        cof_segment_index: 0,
        cof_profile: String::new(),
    };
    le.recompute_section();
    le
}

fn ontology_to_library(o: OntologyCatalogEntry, now: u64) -> LibraryEntry {
    let uri = format!("ontology://webizen/{}", o.id);
    let mut le = LibraryEntry {
        asset_uri: uri.clone(),
        primary_subject: primary_subject_for_uri(&uri),
        media_type: ONTOLOGY_MEDIA_TYPE.into(),
        quins: Vec::new(),
        topics: vec!["ontology".into(), o.domain.into(), o.id.into()],
        projects: vec![format!("ontology:{}", o.domain)],
        purposes: vec!["ontology".into(), "software".into(), "knowledge".into()],
        place: Some(o.rel_path.into()),
        occurred_at: None,
        lat: None,
        lon: None,
        flags: Vec::new(),
        ingested_unix: now,
        excerpt: format!(
            "{} — domain={} · {} · source {}",
            o.title, o.domain, o.licence_note, o.rel_path
        ),
        sensitivity: "public".into(),
        section: LibrarySection::Software.as_str().into(),
        commons_visibility: CommonsVisibility::None,
        cml_signals: Vec::new(),
        cml_concept_count: 0,
        cml_n3: String::new(),
        cof_html: String::new(),
        cof_segment_count: 0,
        cof_segment_index: 0,
        cof_profile: String::new(),
    };
    le.recompute_section();
    le
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionSeedReport {
    pub models_added: usize,
    pub models_updated: usize,
    pub ontologies_added: usize,
    pub ontologies_updated: usize,
    pub weights_ensured: Vec<String>,
    pub note: String,
}

/// Ensure seed weight files exist and catalogue models + ontologies into Library.
pub fn seed_perception_into_library(
    store: &HypermediaStore,
    storage_root: &Path,
) -> Result<PerceptionSeedReport, String> {
    let mut weights_ensured = Vec::new();
    if let Ok(s) = crate::audio_pipeline::ensure_aed_weights(storage_root) {
        weights_ensured.push(s);
    }
    if let Ok(s) = crate::audio_pipeline::ensure_speech_weights(storage_root) {
        weights_ensured.push(s);
    }
    if let Ok(s) = crate::vision_pipeline::ensure_vision_weights(storage_root) {
        weights_ensured.push(s);
    }

    let now = now_unix();
    let mut entries = store.load().map_err(|e| e.to_string())?;
    let mut by_uri: std::collections::HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.asset_uri.clone(), i))
        .collect();

    let mut models_added = 0usize;
    let mut models_updated = 0usize;
    for row in PERCEPTION_MODEL_CATALOG {
        let mut entry = model_to_library(*row, storage_root, now);
        let uri = entry.asset_uri.clone();
        if let Some(&idx) = by_uri.get(&uri) {
            entry.ingested_unix = entries[idx].ingested_unix;
            entries[idx] = entry;
            models_updated += 1;
        } else {
            by_uri.insert(uri, entries.len());
            entries.push(entry);
            models_added += 1;
        }
    }

    let mut ontologies_added = 0usize;
    let mut ontologies_updated = 0usize;
    for row in PERCEPTION_ONTOLOGY_CATALOG {
        let mut entry = ontology_to_library(*row, now);
        let uri = entry.asset_uri.clone();
        if let Some(&idx) = by_uri.get(&uri) {
            entry.ingested_unix = entries[idx].ingested_unix;
            entries[idx] = entry;
            ontologies_updated += 1;
        } else {
            by_uri.insert(uri, entries.len());
            entries.push(entry);
            ontologies_added += 1;
        }
    }

    store.replace_all(&entries).map_err(|e| e.to_string())?;

    Ok(PerceptionSeedReport {
        models_added,
        models_updated,
        ontologies_added,
        ontologies_updated,
        weights_ensured,
        note: "Models and ontologies catalogued in Library → Software with honesty flags on seed weights. Not foundation models.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_nonempty() {
        assert!(!PERCEPTION_MODEL_CATALOG.is_empty());
        assert!(PERCEPTION_ONTOLOGY_CATALOG.len() >= 10);
        for m in PERCEPTION_MODEL_CATALOG {
            assert!(!m.id.is_empty());
        }
    }

    #[test]
    fn seed_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = HypermediaStore::open(dir.path()).unwrap();
        let r1 = seed_perception_into_library(&store, dir.path()).unwrap();
        assert!(r1.models_added + r1.ontologies_added > 0);
        let r2 = seed_perception_into_library(&store, dir.path()).unwrap();
        assert_eq!(r2.models_added, 0);
        assert_eq!(r2.ontologies_added, 0);
        assert!(r2.models_updated >= PERCEPTION_MODEL_CATALOG.len());
        let software = store.by_section(LibrarySection::Software).unwrap();
        assert!(software.iter().any(|e| e.asset_uri.starts_with("model://")));
        assert!(software
            .iter()
            .any(|e| e.asset_uri.starts_with("ontology://")));
    }

    #[test]
    fn test_vision_catalog_contains_permissive_models() {
        let model_ids: Vec<&str> = PERCEPTION_MODEL_CATALOG.iter().map(|m| m.id).collect();

        assert!(model_ids.contains(&"vision-yunet-face"));
        assert!(model_ids.contains(&"vision-sface-embed"));
        assert!(model_ids.contains(&"vision-mediapipe-facemesh"));
        assert!(model_ids.contains(&"vision-midas-depth"));
        assert!(model_ids.contains(&"vision-espcn-sr"));
        assert!(model_ids.contains(&"vision-yolonas-object"));
    }
}
