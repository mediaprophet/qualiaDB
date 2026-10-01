//! Package exposure manifest digests (UE-053).
//!
//! Pins the Civics package receipt fields: JSON-LD context digest, engine
//! version, shapes JSON hash, and Vibe AST tag identity (Tag 4200).

use crate::sparql_library::rdf_formats::{
    context_digest_hex, QUALIA_JSONLD_CONTEXT_ID,
};

/// Vibe homoiconic CBOR AST tag — kept in lockstep with `vibe::cbor_ast::TAG_VIBE_AST`.
pub const VIBE_AST_TAG: u64 = 4200;

/// Receipt fields for a Civics / ontology package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageExposureManifest {
    pub engine_version: &'static str,
    pub context_id: &'static str,
    pub context_digest_sha256: String,
    pub shapes_digest_sha256: String,
    pub vibe_ast_tag: u64,
    pub vibe_ast_digest_sha256: String,
    pub profile: &'static str,
}

/// Build a package exposure manifest.
///
/// `shapes_json` is the ShapeSpec JSON bytes used for admission (Civics primary).
/// `vibe_program_cbor` is optional Tag-4200 program bytes; empty → digest of the
/// tag constant alone (schema pin without a program body).
pub fn package_exposure_manifest(
    shapes_json: &[u8],
    vibe_program_cbor: &[u8],
) -> PackageExposureManifest {
    let shapes_digest_sha256 = sha256_hex(shapes_json);
    let mut vibe_material = Vec::with_capacity(8 + vibe_program_cbor.len());
    vibe_material.extend_from_slice(&VIBE_AST_TAG.to_le_bytes());
    vibe_material.extend_from_slice(vibe_program_cbor);
    PackageExposureManifest {
        engine_version: crate::ENGINE_VERSION,
        context_id: QUALIA_JSONLD_CONTEXT_ID,
        context_digest_sha256: context_digest_hex(),
        shapes_digest_sha256,
        vibe_ast_tag: VIBE_AST_TAG,
        vibe_ast_digest_sha256: sha256_hex(&vibe_material),
        profile: "qualia-package-exposure-v1",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let dig = Sha256::digest(bytes);
    dig.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manifest_is_stable_for_same_inputs() {
        let shapes = br#"[{"targetClass":":P","constraints":[{"kind":"minCount","num":1}]}]"#;
        let a = package_exposure_manifest(shapes, &[]);
        let b = package_exposure_manifest(shapes, &[]);
        assert_eq!(a, b);
        assert_eq!(a.vibe_ast_tag, 4200);
        assert_eq!(a.context_id, QUALIA_JSONLD_CONTEXT_ID);
        assert!(!a.context_digest_sha256.is_empty());
        assert_eq!(a.shapes_digest_sha256.len(), 64);
    }

    #[test]
    fn different_shapes_change_digest() {
        let a = package_exposure_manifest(b"[]", &[]);
        let b = package_exposure_manifest(b"[1]", &[]);
        assert_ne!(a.shapes_digest_sha256, b.shapes_digest_sha256);
    }
}
