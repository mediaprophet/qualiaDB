//! Validate-before-render barrier (P2)
//!
//! A strict, fail-closed gate that must pass before geometry is accepted for rendering.
//! Validates the relational SHACL rules of the manifest, checks the `.10d` whole-file CRC,
//! decodes the provenance sidecar, asserts the immutable `source_digest`, ensures the
//! presence of a licence (context not stripped), and optionally verifies any attached
//! Verifiable Credentials against grounded issuers.

use ed25519_dalek::{Signature, VerifyingKey};

use crate::container_10d::header::Container10dHeader;
use crate::container_10d::provenance_section::{decode_provenance_section, validate_provenance};
use crate::container_10d::section::{parse_section_table, SectionType};
use crate::container_10d::{decode_mesh_section, verify_whole_file_crc32c};
use crate::crypto::verifiable_credential::{decode_credential, verify_grounded, VcError};
use crate::indexing::QuinIndex;
use crate::modalities::logic::geometry_asset_shacl::{
    validate_geometry_manifest, GeometryAssetConfiguration, GeometryConstraintViolation,
    GeometryManifestFacts,
};
use crate::render::assets::Mesh;

/// Errors that can occur during the validate-before-render barrier.
#[derive(Debug, PartialEq, Eq)]
pub enum BarrierError {
    /// The container failed CRC-32C or header validation.
    ContainerIntegrity(String),
    /// The manifest failed relational SHACL validation.
    ManifestViolation(Vec<GeometryConstraintViolation>),
    /// The provenance sidecar was missing, malformed, or failed the digest/licence gate.
    ProvenanceFailed(String),
    /// The attached verifiable credential failed signature/expiry or issuer grounding checks.
    CredentialInvalid(VcError),
    /// The VC payload was malformed (could not decode signature + credential bytes).
    CredentialMalformed,
    /// The container has no `QuantizedMesh` section.
    NoMesh,
}

impl std::fmt::Display for BarrierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerIntegrity(e) => write!(f, "Barrier: Container integrity: {e}"),
            Self::ManifestViolation(vs) => {
                write!(f, "Barrier: SHACL manifest violations: {vs:?}")
            }
            Self::ProvenanceFailed(e) => write!(f, "Barrier: Provenance gate failed: {e}"),
            Self::CredentialInvalid(e) => write!(f, "Barrier: VC invalid: {e}"),
            Self::CredentialMalformed => write!(f, "Barrier: VC payload malformed"),
            Self::NoMesh => write!(f, "Barrier: No QuantizedMesh section found"),
        }
    }
}
impl std::error::Error for BarrierError {}

/// The validate-before-render barrier.
///
/// 1. Relational SHACL validation.
/// 2. `.10d` whole-file CRC verification.
/// 3. Provenance extraction and `validate_provenance` (source_digest and licence checks).
/// 4. If a VC is present, decodes it and verifies the signature using `verify_grounded`.
/// 5. Finally decodes and returns the `Mesh`.
pub fn validate_before_render(
    container_bytes: &[u8],
    manifest: &GeometryManifestFacts,
    config: &GeometryAssetConfiguration,
    index: &QuinIndex,
    now: u32,
    key_resolver: impl Fn(u64) -> Option<VerifyingKey>,
) -> Result<Mesh, BarrierError> {
    // 1. Relational SHACL validation
    let violations = validate_geometry_manifest(manifest, config);
    if !violations.is_empty() {
        return Err(BarrierError::ManifestViolation(violations));
    }

    // 2. Container Integrity
    let mut bytes_mut = container_bytes.to_vec();
    verify_whole_file_crc32c(&mut bytes_mut)
        .map_err(|e| BarrierError::ContainerIntegrity(e.to_string()))?;

    let header = Container10dHeader::parse(&bytes_mut)
        .map_err(|e| BarrierError::ContainerIntegrity(e.to_string()))?;

    let descs = parse_section_table(&bytes_mut, &header)
        .map_err(|e| BarrierError::ContainerIntegrity(format!("{e:?}")))?;

    let mut mesh = None;
    let mut provenance = None;

    for desc in descs.iter() {
        let st = SectionType::from_u8(desc.section_type);
        if let Some(st) = st {
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            let payload = &bytes_mut[off..off + len];

            if st == SectionType::QuantizedMesh {
                mesh = Some(payload);
            } else if st == SectionType::ProvenanceSidecar {
                provenance = Some(payload);
            }
        }
    }

    // 3. Provenance Extraction & Validation
    let prov_payload = provenance
        .ok_or_else(|| BarrierError::ProvenanceFailed("No provenance sidecar found".to_string()))?;

    let view = decode_provenance_section(prov_payload)
        .map_err(|e| BarrierError::ProvenanceFailed(e.to_string()))?;

    validate_provenance(&view).map_err(|e| BarrierError::ProvenanceFailed(e.to_string()))?;

    // 4. VC Verification
    if let Some(vc_bytes) = view.vc() {
        if vc_bytes.len() < 64 {
            return Err(BarrierError::CredentialMalformed);
        }
        let (sig_bytes, cred_bytes) = vc_bytes.split_at(64);
        let signature = Signature::from_bytes(sig_bytes.try_into().unwrap());
        let credential =
            decode_credential(cred_bytes).map_err(|_| BarrierError::CredentialMalformed)?;

        let issuer_key = key_resolver(credential.issuer).ok_or_else(|| {
            BarrierError::CredentialInvalid(VcError::InvalidSignature) // Key not found
        })?;

        verify_grounded(&credential, &issuer_key, &signature, now, index)
            .map_err(BarrierError::CredentialInvalid)?;
    }

    // 5. Decode Mesh
    let mesh_payload = mesh.ok_or(BarrierError::NoMesh)?;
    decode_mesh_section(mesh_payload).map_err(|e| BarrierError::ContainerIntegrity(e.to_string()))
}
