//! CBOR-LD Self-Contained Verifiable Credential Manifest for `.hmc` Bundles.
//!
//! Bridges the Webizen decentralized identity layer (W3C Verifiable Credentials,
//! JSON-LD/CBOR-LD `@context`, `did:web:...`) with physical digital twin binary payloads.
//!
//! Binds the Tripartite Ontology (`DarwinianTaxon`, `ManufacturedProperty`), affordances
//! (procedural seeds, continuous acoustic anchors), and physical geometry layout offsets/digests
//! sequenced in Reverse-LOD order:
//! 1. Header & CBOR-LD Manifest
//! 2. Section 20: SURF1 Hierarchical Surfels (far-field macro view)
//! 3. Section 19: IMPO1 Octahedral Atlases (mid-to-far field proxy)
//! 4. Section 2: Decimated Meshlets (mid-range geometry)
//! 5. Section 1: Hero Meshlets (close-up detailed geometry)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::format::BundleError;

pub const CBOR_LD_CONTEXT_CREDENTIALS_V1: &str = "https://www.w3.org/2018/credentials/v1";
pub const CBOR_LD_CONTEXT_WEBCIVICS_V1: &str = "hmc://schemas/webcivics/v1";

pub const CBOR_LD_MANIFEST_ENTRY_KEY: &str = "manifest/entity_manifest.cborld";
pub const CBOR_LD_MANIFEST_LEGACY_KEY: &str = "manifest/twin_manifest.cborld";
pub const CBOR_LD_MANIFEST_KIND: &str = "cborld_manifest";

/// W3C Verifiable Credential manifest encoded as CBOR-LD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CborLdManifest {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    pub issuer: String,
    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,
    #[serde(rename = "credentialSubject")]
    pub credential_subject: CredentialSubject,
    pub proof: CredentialProof,
}

/// The credential subject binding physical affordances and geometry layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialSubject {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: String, // e.g. "DarwinianTaxon", "ManufacturedProperty"
    pub affordances: SubjectAffordances,
    pub geometry_layout: GeometryLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commons_obligation: Option<CommonsObligation>,
    /// ADR 0014 Payload catalogue for prose-overflow and multimodal commentary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payload_catalogue: Vec<PayloadCatalogueEntry>,
}

/// ADR 0014 Payload Catalogue Entry for prose-overflow, multi-lingual notes, or oral history audio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadCatalogueEntry {
    pub payload_key: String,
    pub mime_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub role: String,
    pub byte_length: u64,
    pub sha256_hex: String,
}

/// Humanitarian ICT & Permissive Commons terms for living biosphere & cognitive heritage.
///
/// ## Ontological Demarcation:
/// 1. **Commercial Rights / Manufactured Property (`ManufacturedProperty`)**:
///    Commercial vendors (e.g. Nike, industrial manufacturers, proprietary software authors)
///    retain full commercial freedom to set market prices, profit margins, and licensing terms.
///    Consumers have a right to fundamental bodily needs (shoes to walk, shelter, clothing),
///    but not an entitlement to a specific trademarked commercial design of shoe.
/// 2. **"World of God" / Biosphere / Knowledge for Life (`DarwinianTaxon`, `AbioticEnvironment`)**:
///    Pertains to living species, ecological heritage, and foundational knowledge necessary for
///    life, being, and freedom of thought. These CANNOT be enclosed behind perpetual tollbooths
///    or privatized royalties.
///    Instead, under Humanitarian ICT principles:
///    - **Finite Obligation Cost**: The limited, auditable labor and digitization cost to produce
///      the virtual entity may be recovered up to an ROI cap via an accumulated compensation pool.
///    - **Direct Carriage / Bandwidth Recovery**: Actual transaction cost per MiB for network
///      transport may be met at cost.
///    - **No Perpetual Fees**: Once production obligations are discharged, the asset remains
///      in pure permissive commons with zero perpetual tolls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonsObligation {
    /// Licencing framework: "SocialPermissiveCommons", "CC0-1.0", "CC-BY-4.0", etc.
    pub licence_class: String,
    /// Finite, audited cost of production in minor units (e.g. cents) - 0 if purely donated commons.
    #[serde(default)]
    pub production_cost_cents: u64,
    /// Legal ROI cap percentage (capped to prevent extractive rent).
    #[serde(default)]
    pub roi_cap_percent: u64,
    /// Cumulative compensation payments made towards discharging production cost.
    #[serde(default)]
    pub accumulated_pool_cents: u64,
    /// Allowed carrier/bandwidth transaction recovery cost in minor units per MiB.
    #[serde(default)]
    pub transaction_cost_per_mib_cents: u64,
    /// Prohibits perpetual fees or rent-seeking tolls in perpetuity.
    #[serde(default = "default_true")]
    pub perpetual_fees_prohibited: bool,
}

fn default_true() -> bool {
    true
}

impl CommonsObligation {
    pub fn is_discharged(&self) -> bool {
        if self.production_cost_cents == 0 {
            return true;
        }
        let total_cost = crate::modalities::value_flow::commons_cost(
            self.production_cost_cents,
            self.roi_cap_percent,
            20,
        );
        crate::modalities::value_flow::is_commons_discharged(
            self.accumulated_pool_cents,
            total_cost,
        )
    }

    pub fn outstanding_obligation(&self) -> u64 {
        let total_cost = crate::modalities::value_flow::commons_cost(
            self.production_cost_cents,
            self.roi_cap_percent,
            20,
        );
        crate::modalities::value_flow::outstanding(self.accumulated_pool_cents, total_cost)
    }

    /// Automatically discharges the obligation when accumulated compensation meets
    /// the capped production cost, transitioning licence to SocialPermissiveCommons.
    pub fn discharge_to_commons(&mut self) -> bool {
        if self.is_discharged() {
            self.licence_class = "SocialPermissiveCommons".to_string();
            self.perpetual_fees_prohibited = true;
            true
        } else {
            false
        }
    }
}

/// Physical and procedural affordances attached to the entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectAffordances {
    pub procedural_seed: String,
    pub acoustic_anchors: AcousticAnchors,
}

/// 3-band continuous acoustic frequency anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticAnchors {
    pub low_hz: u32,  // 200 Hz
    pub mid_hz: u32,  // 1000 Hz
    pub high_hz: u32, // 8000 Hz
}

impl Default for AcousticAnchors {
    fn default() -> Self {
        Self {
            low_hz: 200,
            mid_hz: 1000,
            high_hz: 8000,
        }
    }
}

/// Reverse-LOD physical layout within the container or bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryLayout {
    pub surf1: SectionLayoutEntry,
    pub impo1: SectionLayoutEntry,
    pub mesh1: SectionLayoutEntry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh2: Option<SectionLayoutEntry>,
}

/// Byte offset, byte length, and SHA-256 cryptographic digest of a binary section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionLayoutEntry {
    pub offset: u64,
    pub length: u64,
    pub digest: String, // e.g. "sha256-..."
}

/// W3C Data Integrity proof (e.g. Ed25519Signature2020).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialProof {
    #[serde(rename = "type")]
    pub proof_type: String,
    pub created: String,
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,
    #[serde(rename = "proofValue")]
    pub proof_value: String,
}

impl CborLdManifest {
    /// Construct a new Verifiable Credential manifest with default context and types.
    pub fn new(
        credential_id: impl Into<String>,
        issuer_did: impl Into<String>,
        issuance_date: impl Into<String>,
        subject_id: impl Into<String>,
        entity_type: impl Into<String>,
        procedural_seed: impl Into<String>,
        acoustic_anchors: AcousticAnchors,
        geometry_layout: GeometryLayout,
        verification_method: impl Into<String>,
        proof_value: impl Into<String>,
    ) -> Self {
        let created = issuance_date.into();
        Self {
            context: vec![
                CBOR_LD_CONTEXT_CREDENTIALS_V1.to_string(),
                CBOR_LD_CONTEXT_WEBCIVICS_V1.to_string(),
            ],
            id: credential_id.into(),
            credential_type: vec![
                "VerifiableCredential".to_string(),
                "QualiaEntity".to_string(),
            ],
            issuer: issuer_did.into(),
            issuance_date: created.clone(),
            credential_subject: CredentialSubject {
                id: subject_id.into(),
                entity_type: entity_type.into(),
                affordances: SubjectAffordances {
                    procedural_seed: procedural_seed.into(),
                    acoustic_anchors,
                },
                geometry_layout,
                commons_obligation: None,
                payload_catalogue: Vec::new(),
            },
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".to_string(),
                created,
                verification_method: verification_method.into(),
                proof_purpose: "assertionMethod".to_string(),
                proof_value: proof_value.into(),
            },
        }
    }

    /// Construct a verified botanical/ecological Specimen manifest (DarwinianTaxon).
    pub fn new_specimen(
        credential_id: impl Into<String>,
        issuer_did: impl Into<String>,
        issuance_date: impl Into<String>,
        taxon_id: impl Into<String>,
        procedural_seed: impl Into<String>,
        acoustic_anchors: AcousticAnchors,
        geometry_layout: GeometryLayout,
        verification_method: impl Into<String>,
        proof_value: impl Into<String>,
    ) -> Self {
        let mut manifest = Self::new(
            credential_id,
            issuer_did,
            issuance_date,
            taxon_id,
            "DarwinianTaxon",
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        );
        manifest.credential_type = vec!["VerifiableCredential".into(), "TaxonSpecimen".into()];
        manifest.credential_subject.commons_obligation = Some(CommonsObligation {
            licence_class: "SocialPermissiveCommons".into(),
            production_cost_cents: 0,
            roi_cap_percent: 20,
            accumulated_pool_cents: 0,
            transaction_cost_per_mib_cents: 0,
            perpetual_fees_prohibited: true,
        });
        manifest
    }

    /// Construct a sovereign Human Persona / Likeness manifest (NaturalPerson).
    /// Inalienable: cannot be commodified or assigned as property.
    pub fn new_persona(
        credential_id: impl Into<String>,
        human_did: impl Into<String>,
        issuance_date: impl Into<String>,
        subject_did: impl Into<String>,
        procedural_seed: impl Into<String>,
        acoustic_anchors: AcousticAnchors,
        geometry_layout: GeometryLayout,
        verification_method: impl Into<String>,
        proof_value: impl Into<String>,
    ) -> Self {
        let mut manifest = Self::new(
            credential_id,
            human_did,
            issuance_date,
            subject_did,
            "NaturalPerson",
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        );
        manifest.credential_type = vec![
            "VerifiableCredential".into(),
            "HumanPersona".into(),
            "SovereignLikeness".into(),
        ];
        manifest
    }

    /// Construct a Manufactured Construct manifest (ManufacturedProperty).
    pub fn new_construct(
        credential_id: impl Into<String>,
        issuer_did: impl Into<String>,
        issuance_date: impl Into<String>,
        property_id: impl Into<String>,
        procedural_seed: impl Into<String>,
        acoustic_anchors: AcousticAnchors,
        geometry_layout: GeometryLayout,
        verification_method: impl Into<String>,
        proof_value: impl Into<String>,
    ) -> Self {
        let mut manifest = Self::new(
            credential_id,
            issuer_did,
            issuance_date,
            property_id,
            "ManufacturedProperty",
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        );
        manifest.credential_type = vec![
            "VerifiableCredential".into(),
            "ManufacturedConstruct".into(),
        ];
        manifest
    }

    /// Serialise the manifest into binary CBOR-LD.
    pub fn encode_to_cbor(&self) -> Result<Vec<u8>, BundleError> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| BundleError::Cbor(e.to_string()))?;
        Ok(buf)
    }

    /// Deserialise a manifest from binary CBOR-LD bytes.
    pub fn decode_from_cbor(bytes: &[u8]) -> Result<Self, BundleError> {
        ciborium::from_reader(bytes).map_err(|e| BundleError::Cbor(e.to_string()))
    }

    /// Serialise the manifest into standard JSON-LD representation.
    pub fn encode_to_json(&self) -> Result<String, BundleError> {
        serde_json::to_string_pretty(self).map_err(|e| BundleError::Cbor(e.to_string()))
    }

    /// Deserialise the manifest from JSON-LD string.
    pub fn decode_from_json(json: &str) -> Result<Self, BundleError> {
        serde_json::from_str(json).map_err(|e| BundleError::Cbor(e.to_string()))
    }

    /// Compute SHA-256 digest string in the format "sha256-<hex>".
    pub fn compute_sha256_digest(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let mut hex_str = String::with_capacity(64);
        for byte in hash {
            use std::fmt::Write;
            let _ = write!(hex_str, "{:02x}", byte);
        }
        format!("sha256-{}", hex_str)
    }

    /// Verify that physical byte slices match the declared offsets, lengths, and digests.
    pub fn verify_geometry_digests(&self, bundle_bytes: &[u8]) -> Result<bool, BundleError> {
        let entries = [
            &self.credential_subject.geometry_layout.surf1,
            &self.credential_subject.geometry_layout.impo1,
            &self.credential_subject.geometry_layout.mesh1,
        ];

        for entry in entries {
            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            if end > bundle_bytes.len() {
                return Err(BundleError::EntryOutOfBounds {
                    key: "geometry_layout".to_string(),
                    offset: entry.offset,
                    length: entry.length,
                });
            }
            let actual = Self::compute_sha256_digest(&bundle_bytes[start..end]);
            if actual != entry.digest {
                return Ok(false);
            }
        }

        if let Some(ref m2) = self.credential_subject.geometry_layout.mesh2 {
            let start = m2.offset as usize;
            let end = start + m2.length as usize;
            if end > bundle_bytes.len() {
                return Err(BundleError::EntryOutOfBounds {
                    key: "mesh2".to_string(),
                    offset: m2.offset,
                    length: m2.length,
                });
            }
            let actual = Self::compute_sha256_digest(&bundle_bytes[start..end]);
            if actual != m2.digest {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbor_ld_manifest_roundtrips_json_and_cbor() {
        let layout = GeometryLayout {
            surf1: SectionLayoutEntry {
                offset: 2048,
                length: 32768,
                digest: "sha256-1111".to_string(),
            },
            impo1: SectionLayoutEntry {
                offset: 34816,
                length: 524288,
                digest: "sha256-2222".to_string(),
            },
            mesh1: SectionLayoutEntry {
                offset: 559104,
                length: 14500000,
                digest: "sha256-3333".to_string(),
            },
            mesh2: None,
        };

        let manifest = CborLdManifest::new(
            "urn:uuid:3f8a42e0-2026-eucalyptus",
            "did:web:trustfactory.org:compiler",
            "2026-10-13T08:46:00Z",
            "urn:qualia:taxon:eucalyptus_regnans_01",
            "DarwinianTaxon",
            "hmc://seeds/voronoi_bark",
            AcousticAnchors::default(),
            layout,
            "did:web:trustfactory.org:compiler#key-1",
            "z58DfakeSignatureValue2026",
        );

        // JSON roundtrip
        let json_str = manifest.encode_to_json().expect("json encode");
        assert!(json_str.contains("@context"));
        assert!(json_str.contains("DarwinianTaxon"));
        assert!(json_str.contains("did:web:trustfactory.org:compiler"));
        let json_dec = CborLdManifest::decode_from_json(&json_str).expect("json decode");
        assert_eq!(json_dec, manifest);

        // CBOR roundtrip
        let cbor_bytes = manifest.encode_to_cbor().expect("cbor encode");
        let cbor_dec = CborLdManifest::decode_from_cbor(&cbor_bytes).expect("cbor decode");
        assert_eq!(cbor_dec, manifest);
    }

    #[test]
    fn typed_constructors_bind_expected_credentials() {
        let dummy_layout = GeometryLayout {
            surf1: SectionLayoutEntry {
                offset: 0,
                length: 10,
                digest: CborLdManifest::compute_sha256_digest(&[1; 10]),
            },
            impo1: SectionLayoutEntry {
                offset: 16,
                length: 10,
                digest: CborLdManifest::compute_sha256_digest(&[2; 10]),
            },
            mesh1: SectionLayoutEntry {
                offset: 32,
                length: 10,
                digest: CborLdManifest::compute_sha256_digest(&[3; 10]),
            },
            mesh2: None,
        };

        let specimen = CborLdManifest::new_specimen(
            "urn:uuid:specimen-01",
            "did:web:trustfactory.org:compiler",
            "2026-10-13T08:46:00Z",
            "urn:qualia:taxon:eucalyptus",
            "hmc://seeds/bark",
            AcousticAnchors::default(),
            dummy_layout.clone(),
            "did:web:trustfactory.org:compiler#key-1",
            "zProof",
        );
        assert_eq!(specimen.credential_subject.entity_type, "DarwinianTaxon");
        assert!(specimen
            .credential_type
            .contains(&"TaxonSpecimen".to_string()));

        let persona = CborLdManifest::new_persona(
            "urn:uuid:persona-01",
            "did:key:citizen",
            "2026-10-13T08:46:00Z",
            "did:key:citizen",
            "hmc://seeds/face",
            AcousticAnchors::default(),
            dummy_layout.clone(),
            "did:key:citizen#key-1",
            "zProof",
        );
        assert_eq!(persona.credential_subject.entity_type, "NaturalPerson");
        assert!(persona
            .credential_type
            .contains(&"HumanPersona".to_string()));

        let construct = CborLdManifest::new_construct(
            "urn:uuid:construct-01",
            "did:web:trustfactory.org:compiler",
            "2026-10-13T08:46:00Z",
            "urn:qualia:artifact:pump",
            "hmc://seeds/pump",
            AcousticAnchors::default(),
            dummy_layout,
            "did:web:trustfactory.org:compiler#key-1",
            "zProof",
        );
        assert_eq!(
            construct.credential_subject.entity_type,
            "ManufacturedProperty"
        );
        assert!(construct
            .credential_type
            .contains(&"ManufacturedConstruct".to_string()));
    }
}
