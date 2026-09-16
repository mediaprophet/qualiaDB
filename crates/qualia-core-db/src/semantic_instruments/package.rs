//! Logical collectable: HMC archive of HCF + definition N3 + `.10d` visual.
//!
//! Large Q42 volumes are dependencies, not this package. Native builds may also
//! embed a small `.q42` compiled from the definition graph.

use crate::bundle::{BundleReader, BundleWriter};

use super::canonical::{canonical_manifest_bytes, sha256_prefixed};
use super::errors::InstrumentError;
use super::manifest::InstrumentManifest;
use super::visual::{demo_badge_10d, VISUAL_MEDIA_TYPE};

pub const KEY_MANIFEST: &str = "manifest.json";
pub const KEY_HCF: &str = "instrument.hcf";
pub const KEY_GRAPH: &str = "graphs/instrument-definition.n3";
pub const KEY_VISUAL: &str = "assets/badge.10d";
pub const KEY_Q42: &str = "graphs/instrument.q42";

pub const MAX_COLLECTABLE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_MEMBER_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_MEMBERS: usize = 16;

/// Parts of one collectable before digest is sealed.
pub struct CollectableParts {
    pub manifest: InstrumentManifest,
    pub definition_n3: Vec<u8>,
    pub hcf: Vec<u8>,
    pub visual_10d: Vec<u8>,
    pub small_q42: Option<Vec<u8>>,
}

/// Opened HMC collectable.
#[derive(Debug, Clone)]
pub struct OpenedCollectable {
    pub manifest: InstrumentManifest,
    pub definition_n3: Vec<u8>,
    pub hcf: Vec<u8>,
    pub visual_10d: Vec<u8>,
    pub small_q42: Option<Vec<u8>>,
    pub bundle_digest: String,
}

fn reject_unsafe_key(key: &str) -> Result<(), InstrumentError> {
    if key.is_empty()
        || key.starts_with('/')
        || key.starts_with('\\')
        || key.contains("..")
        || key.contains('\\')
        || key.contains('\0')
    {
        return Err(InstrumentError::PathTraversal);
    }
    Ok(())
}

fn push_member(
    writer: &mut BundleWriter,
    key: &'static str,
    kind: &str,
    bytes: Vec<u8>,
) -> Result<(), InstrumentError> {
    reject_unsafe_key(key)?;
    if bytes.is_empty() {
        return Err(InstrumentError::EmptyMember(key));
    }
    if bytes.len() > MAX_MEMBER_BYTES {
        return Err(InstrumentError::EntryTooLarge {
            key: key.into(),
            bytes: bytes.len(),
        });
    }
    writer.add_file(key, kind, bytes, None)?;
    Ok(())
}

/// yaml-ld-q42 style HCF document for a collectable (authoring layer).
pub fn hcf_from_manifest(manifest: &InstrumentManifest) -> Vec<u8> {
    let yaml = format!(
        "# Hypermedia Content Format (HCF) — semantic instrument collectable\n\
         # Category: {cat}\n\
         \"@context\":\n\
         \"  si\": \"https://ns.webizen.org/semantic-instrument/\"\n\
         \"@id\": \"{release}\"\n\
         \"@type\": \"si:InstrumentRelease\"\n\
         si:instrument: \"{instrument}\"\n\
         si:version: \"{version}\"\n\
         si:name: \"{name}\"\n\
         si:purpose: \"{purpose}\"\n\
         si:contentCategory: \"{cat_iri}\"\n\
         si:honestyNotice: \"{notice}\"\n\
         si:accessibleText: \"{a11y}\"\n\
         si:visualMediaType: \"{media}\"\n",
        cat = format!("{:?}", manifest.content_category).to_lowercase(),
        release = manifest.release_id,
        instrument = manifest.instrument_id,
        version = manifest.version,
        name = manifest.name,
        purpose = manifest.purpose,
        cat_iri = manifest.content_category.as_iri(),
        notice = manifest.honesty_notice,
        a11y = manifest.accessible_text,
        media = manifest.visual_media_type,
    );
    yaml.into_bytes()
}

/// Build an HMC collectable. The sealed `content_digest` is the SHA-256 of the
/// bundle bytes after members (including a digest-less manifest) are packed,
/// then the manifest is rewritten... To keep a single pass: digest is of all
/// members except the manifest; the manifest records that digest; the bundle
/// digest is SHA-256 of the finished HMC.
pub fn build_collectable(mut parts: CollectableParts) -> Result<Vec<u8>, InstrumentError> {
    parts.manifest.validate()?;
    if parts.manifest.visual_media_type.is_empty() {
        parts.manifest.visual_media_type = VISUAL_MEDIA_TYPE.into();
    }
    if parts.visual_10d.is_empty() {
        parts.visual_10d = demo_badge_10d().to_vec();
    }
    if parts.hcf.is_empty() {
        parts.hcf = hcf_from_manifest(&parts.manifest);
    }

    use sha2::{Digest, Sha256};
    let mut payload_hasher = Sha256::new();
    payload_hasher.update(&parts.hcf);
    payload_hasher.update(&parts.definition_n3);
    payload_hasher.update(&parts.visual_10d);
    if let Some(q) = &parts.small_q42 {
        payload_hasher.update(q);
    }
    let payload_digest = payload_hasher.finalize();
    let mut hex = String::from("sha256:");
    for b in payload_digest {
        hex.push_str(&format!("{b:02x}"));
    }
    parts.manifest.content_digest = hex;
    parts.manifest.validate()?;

    let mut writer = BundleWriter::new();
    push_member(
        &mut writer,
        KEY_MANIFEST,
        "manifest",
        canonical_manifest_bytes(&parts.manifest)?,
    )?;
    push_member(&mut writer, KEY_HCF, "hcf", parts.hcf)?;
    push_member(&mut writer, KEY_GRAPH, "n3", parts.definition_n3)?;
    push_member(&mut writer, KEY_VISUAL, "10d", parts.visual_10d)?;
    if let Some(q42) = parts.small_q42 {
        push_member(&mut writer, KEY_Q42, "q42", q42)?;
    }
    if writer.len() > MAX_MEMBERS {
        return Err(InstrumentError::TooManyEntries {
            count: writer.len(),
        });
    }
    let bytes = writer.build()?;
    if bytes.len() > MAX_COLLECTABLE_BYTES {
        return Err(InstrumentError::BundleTooLarge {
            bytes: bytes.len(),
        });
    }
    Ok(bytes)
}

pub fn open_collectable(bytes: &[u8]) -> Result<OpenedCollectable, InstrumentError> {
    if bytes.len() > MAX_COLLECTABLE_BYTES {
        return Err(InstrumentError::BundleTooLarge {
            bytes: bytes.len(),
        });
    }
    let reader = BundleReader::parse(bytes)?;
    let manifest_bytes = reader
        .get(KEY_MANIFEST)
        .ok_or(InstrumentError::MissingField("manifest.json"))?;
    let manifest: InstrumentManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|e| InstrumentError::Canonical(e.to_string()))?;
    manifest.validate()?;
    let definition_n3 = reader
        .get(KEY_GRAPH)
        .ok_or(InstrumentError::MissingField(KEY_GRAPH))?
        .to_vec();
    let hcf = reader
        .get(KEY_HCF)
        .ok_or(InstrumentError::MissingField(KEY_HCF))?
        .to_vec();
    let visual_10d = reader
        .get(KEY_VISUAL)
        .ok_or(InstrumentError::MissingField(KEY_VISUAL))?
        .to_vec();
    crate::container_10d::Container10dHeader::parse(&visual_10d)
        .map_err(|e| InstrumentError::Canonical(format!("badge.10d: {e}")))?;
    if visual_10d.is_empty() || !reader.verify_entry(KEY_VISUAL) {
        return Err(InstrumentError::DigestMismatch);
    }
    let small_q42 = reader.get(KEY_Q42).map(|s| s.to_vec());
    Ok(OpenedCollectable {
        manifest,
        definition_n3,
        hcf,
        visual_10d,
        small_q42,
        bundle_digest: sha256_prefixed(bytes),
    })
}

pub fn reject_traversal_key(key: &str) -> Result<(), InstrumentError> {
    reject_unsafe_key(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::category::ContentCategory;
    use crate::semantic_instruments::manifest::{
        InstrumentManifest, COLLECTABLE_FORMAT_VERSION,
    };

    fn demo_parts() -> CollectableParts {
        let manifest = InstrumentManifest {
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
            honesty_notice: "Demo instrument for demonstration and development.".into(),
            incomplete_input: "held".into(),
            result_kind: "demo-claim".into(),
            licence: "https://spdx.org/licenses/CC-BY-4.0.html".into(),
            authored_by: "did:webizen:agent:demo-seed".into(),
            accessible_text: "Unit conversion demo instrument".into(),
            visual_media_type: VISUAL_MEDIA_TYPE.into(),
            content_digest: String::new(),
            extra: Default::default(),
        };
        CollectableParts {
            hcf: hcf_from_manifest(&manifest),
            definition_n3: b"ex:unit a si:SemanticInstrument .\n".to_vec(),
            visual_10d: demo_badge_10d().to_vec(),
            small_q42: None,
            manifest,
        }
    }

    #[test]
    fn round_trip_hmc_collectable() {
        let bytes = build_collectable(demo_parts()).unwrap();
        let opened = open_collectable(&bytes).unwrap();
        assert_eq!(opened.manifest.content_category, ContentCategory::Demo);
        assert!(opened.manifest.content_digest.starts_with("sha256:"));
        assert!(!opened.visual_10d.is_empty());
        assert!(opened.hcf.starts_with(b"# Hypermedia"));
    }

    #[test]
    fn demo_without_notice_fails() {
        let mut parts = demo_parts();
        parts.manifest.honesty_notice = "fine".into();
        assert_eq!(
            build_collectable(parts).unwrap_err(),
            InstrumentError::DemoUnlabelled
        );
    }

    #[test]
    fn path_traversal_keys_rejected() {
        assert!(reject_traversal_key("../secret").is_err());
        assert!(reject_traversal_key("graphs/instrument-definition.n3").is_ok());
    }

    #[test]
    fn extra_field_survives_round_trip() {
        let mut parts = demo_parts();
        parts
            .manifest
            .extra
            .insert("seed".into(), serde_json::json!("demo-catalog"));
        let opened = open_collectable(&build_collectable(parts).unwrap()).unwrap();
        assert_eq!(
            opened.manifest.extra.get("seed").and_then(|v| v.as_str()),
            Some("demo-catalog")
        );
    }
}
