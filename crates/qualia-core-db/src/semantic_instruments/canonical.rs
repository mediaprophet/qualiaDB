//! Deterministic canonical encoding and SHA-256 digest (cold path).

use sha2::{Digest, Sha256};

use super::errors::InstrumentError;
use super::manifest::InstrumentManifest;

/// Lower-case hex SHA-256 with the `sha256:` prefix used in SI-01 fixtures.
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Compact JSON with struct field order plus sorted `extra` keys (flatten).
pub fn canonical_manifest_bytes(manifest: &InstrumentManifest) -> Result<Vec<u8>, InstrumentError> {
    serde_json::to_vec(manifest).map_err(|e| InstrumentError::Canonical(e.to_string()))
}

pub fn manifest_digest(manifest: &InstrumentManifest) -> Result<String, InstrumentError> {
    let bytes = canonical_manifest_bytes(manifest)?;
    Ok(sha256_prefixed(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::category::ContentCategory;
    use crate::semantic_instruments::manifest::COLLECTABLE_FORMAT_VERSION;

    fn sample() -> InstrumentManifest {
        InstrumentManifest {
            format_version: COLLECTABLE_FORMAT_VERSION.into(),
            instrument_id: "https://ns.webizen.org/demo/unit-convert".into(),
            release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0".into(),
            version: "1.0.0".into(),
            name: "Unit conversion (demo)".into(),
            purpose: "demonstration-and-development".into(),
            content_category: ContentCategory::Demo,
            prohibited_interpretation: "operational-use".into(),
            domain: "https://ns.webizen.org/demo/concepts/units".into(),
            ontology_reference: "https://ns.webizen.org/semantic-instrument/".into(),
            entry_point: "assess".into(),
            citation: "https://ns.webizen.org/demo/citations/si-units".into(),
            honesty_notice: "Demo instrument for demonstration and development. Not operational.".into(),
            incomplete_input: "held".into(),
            result_kind: "demo-claim".into(),
            licence: "https://spdx.org/licenses/CC-BY-4.0.html".into(),
            authored_by: "did:webizen:agent:demo-seed".into(),
            accessible_text: "Unit conversion demo instrument".into(),
            visual_media_type: "application/vnd.qualia.10d".into(),
            content_digest: String::new(),
            extra: Default::default(),
        }
    }

    #[test]
    fn digest_is_stable() {
        let a = manifest_digest(&sample()).unwrap();
        let b = manifest_digest(&sample()).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
        assert_eq!(a.len(), 7 + 64);
    }

    #[test]
    fn extra_fields_round_trip() {
        let mut m = sample();
        m.extra
            .insert("note".into(), serde_json::Value::String("keep".into()));
        let bytes = canonical_manifest_bytes(&m).unwrap();
        let back: InstrumentManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.extra.get("note").and_then(|v| v.as_str()), Some("keep"));
    }
}
