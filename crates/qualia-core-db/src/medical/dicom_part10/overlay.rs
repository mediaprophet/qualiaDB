//! DICOM organ inference, torso overlay placement specs, and JSON/chat helpers.

use std::collections::BTreeMap;
use std::path::Path;

use super::parser::parse_dicom_file;
use super::types::{
    DicomError, DicomMetadata, DicomOverlaySpec, DicomPlacement, DicomTagMatcher,
};

/// Lowercase, alphanumeric-only, whitespace-collapsed form of `value`
/// — the keyword-matching normal form.
pub fn normalize_dicom_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Built-in token->organ matcher table (heart, lung, liver, brain,
/// kidney, ovary, ...) for DICOM text fields.
pub fn default_organ_matchers() -> Vec<DicomTagMatcher> {
    vec![
        DicomTagMatcher {
            tokens: vec![
                "heart".into(),
                "cardiac".into(),
                "coronary".into(),
                "aorta".into(),
            ],
            organ: Some("Heart".into()),
        },
        DicomTagMatcher {
            tokens: vec![
                "lung".into(),
                "pulmonary".into(),
                "chest ct".into(),
                "thorax".into(),
            ],
            organ: Some("Lung".into()),
        },
        DicomTagMatcher {
            tokens: vec!["liver".into(), "hepatic".into()],
            organ: Some("Liver".into()),
        },
        DicomTagMatcher {
            tokens: vec![
                "brain".into(),
                "cerebral".into(),
                "cranial".into(),
                "head".into(),
            ],
            organ: Some("Brain (Allen)".into()),
        },
        DicomTagMatcher {
            tokens: vec!["kidney".into(), "renal".into()],
            organ: Some("Kidney (Left)".into()),
        },
        DicomTagMatcher {
            tokens: vec!["pancrea".into(), "pancreatic".into()],
            organ: Some("Pancreas".into()),
        },
        DicomTagMatcher {
            tokens: vec!["spleen".into(), "splenic".into()],
            organ: Some("Spleen".into()),
        },
        DicomTagMatcher {
            tokens: vec!["intestin".into(), "bowel".into(), "abdomen".into()],
            organ: Some("Small Intestine".into()),
        },
        DicomTagMatcher {
            tokens: vec!["prostate".into()],
            organ: Some("Prostate".into()),
        },
        DicomTagMatcher {
            tokens: vec!["uterus".into(), "uterine".into()],
            organ: Some("Uterus".into()),
        },
        DicomTagMatcher {
            tokens: vec!["ovary".into(), "ovarian".into()],
            organ: Some("Ovary (Left)".into()),
        },
    ]
}

/// Infer the organ from the file's text fields (body part, series/
/// study descriptions, protocol name) via `matchers`.
pub fn infer_organ_from_metadata(
    meta: &DicomMetadata,
    matchers: &[DicomTagMatcher],
) -> Option<String> {
    let haystack = normalize_dicom_token(
        &[
            meta.body_part_examined.as_str(),
            meta.series_description.as_str(),
            meta.study_description.as_str(),
            meta.protocol_name.as_str(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" "),
    );

    infer_organ_from_haystack(&haystack, matchers)
}

/// First-matching-token organ inference over normalized `haystack`.
pub fn infer_organ_from_haystack(haystack: &str, matchers: &[DicomTagMatcher]) -> Option<String> {
    let normalized = normalize_dicom_token(haystack);
    for matcher in matchers {
        for token in &matcher.tokens {
            let token_norm = normalize_dicom_token(token);
            if !token_norm.is_empty() && normalized.contains(&token_norm) {
                return matcher.organ.clone();
            }
        }
    }
    None
}

/// Curated 3D torso-overlay placement per organ label (offset/scale/
/// rotation), with a generic fallback for unlisted organs.
pub fn default_placement_for_organ(organ: &str) -> DicomPlacement {
    let table: BTreeMap<&str, DicomPlacement> = BTreeMap::from([
        (
            "Heart",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.05,
                offset_z: 0.32,
                scale: 0.9,
                rotation_y: 0.0,
            },
        ),
        (
            "Lung",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.08,
                offset_z: 0.3,
                scale: 1.05,
                rotation_y: 0.0,
            },
        ),
        (
            "Liver",
            DicomPlacement {
                offset_x: 0.12,
                offset_y: -0.02,
                offset_z: 0.22,
                scale: 0.95,
                rotation_y: -0.35,
            },
        ),
        (
            "Brain (Allen)",
            DicomPlacement {
                offset_x: 0.0,
                offset_y: 0.18,
                offset_z: 0.12,
                scale: 1.0,
                rotation_y: 0.0,
            },
        ),
        (
            "Kidney (Left)",
            DicomPlacement {
                offset_x: -0.14,
                offset_y: -0.06,
                offset_z: 0.18,
                scale: 0.75,
                rotation_y: 0.2,
            },
        ),
    ]);

    table.get(organ).copied().unwrap_or(DicomPlacement {
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.28,
        scale: 0.85,
        rotation_y: 0.0,
    })
}

/// Assemble a `DicomOverlaySpec` from parsed metadata — organ
/// inferred when not supplied, placement defaulted per organ.
pub fn build_overlay_spec(
    meta: &DicomMetadata,
    organ: Option<String>,
    source: &str,
) -> DicomOverlaySpec {
    let organ_label = organ.or_else(|| infer_organ_from_metadata(meta, &default_organ_matchers()));
    let placement = organ_label
        .as_deref()
        .map(default_placement_for_organ)
        .unwrap_or_else(|| default_placement_for_organ("Heart"));

    DicomOverlaySpec {
        version: "1.0.0".to_string(),
        organ: organ_label,
        opacity: 0.72,
        visible: true,
        placement,
        series_instance_uid: if meta.series_instance_uid.is_empty() {
            None
        } else {
            Some(meta.series_instance_uid.clone())
        },
        modality: if meta.modality.is_empty() {
            None
        } else {
            Some(meta.modality.clone())
        },
        body_part_examined: if meta.body_part_examined.is_empty() {
            None
        } else {
            Some(meta.body_part_examined.clone())
        },
        source: Some(source.to_string()),
    }
}

/// Read `path`, parse its DICOM metadata, and build the overlay spec.
pub fn overlay_spec_from_file(path: &Path) -> Result<DicomOverlaySpec, DicomError> {
    let meta = parse_dicom_file(path)?;
    Ok(build_overlay_spec(
        &meta,
        None,
        &format!("dicom-file:{}", path.display()),
    ))
}

/// `overlay_spec_from_file` serialized to a JSON string (WASM/FFI seam).
pub fn overlay_spec_json_from_file(path: &Path) -> Result<String, String> {
    let spec = overlay_spec_from_file(path).map_err(|e| e.to_string())?;
    serde_json::to_string(&spec).map_err(|e| e.to_string())
}

/// Parsed `DicomMetadata` from `path` serialized to JSON.
pub fn metadata_json_from_file(path: &Path) -> Result<String, String> {
    let meta = parse_dicom_file(path).map_err(|e| e.to_string())?;
    serde_json::to_string(&meta).map_err(|e| e.to_string())
}

const IMAGING_CHAT_MARKERS: &[&str] = &[
    "dicom",
    "ct scan",
    " ct ",
    "mri",
    "x-ray",
    "xray",
    "radiograph",
    "ultrasound",
    "pet scan",
    "mammogram",
    "imaging study",
    "chest x",
    "slice thickness",
    "window level",
    "hounsfield",
];

/// `true` when `text` mentions imaging (marker-substring scan over `IMAGING_CHAT_MARKERS`).
pub fn chat_mentions_imaging(text: &str) -> bool {
    let haystack = format!(" {} ", text.to_lowercase());
    IMAGING_CHAT_MARKERS
        .iter()
        .any(|marker| haystack.contains(marker))
}

/// Build a "chat-inferred" overlay spec when `text` mentions imaging
/// — organ from keyword match, sensible default opacity/placement.
pub fn infer_overlay_spec_from_text(
    text: &str,
    matchers: &[DicomTagMatcher],
) -> Option<DicomOverlaySpec> {
    if !chat_mentions_imaging(text) {
        return None;
    }

    let organ = infer_organ_from_haystack(text, matchers);
    let placement = organ
        .as_deref()
        .map(default_placement_for_organ)
        .unwrap_or_else(|| default_placement_for_organ("Heart"));

    Some(DicomOverlaySpec {
        version: "1.0.0".to_string(),
        organ,
        opacity: 0.72,
        visible: true,
        placement,
        series_instance_uid: None,
        modality: None,
        body_part_examined: None,
        source: Some("chat-inferred".to_string()),
    })
}
