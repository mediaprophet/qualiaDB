//! Reverse-LOD Ontological Entity Compiler & Asset Packaging Engine.
//!
//! Replaces the industrial "digital twin" framing with an ontologically-grounded
//! representation system respecting the sacred boundary between the living biosphere
//! ("world of god") and manufactured property ("world of man"):
//!
//! - **Natural Person (`NaturalPerson`)**: Compiled as a sovereign **`Persona`** / **`Likeness`**.
//!   Anti-commodification protection prevents persons from being classified as commercial property or assets.
//! - **Darwinian Taxon (`DarwinianTaxon`)**: Compiled as a biological/botanical **`Specimen`**.
//!   Part of the ecological commons; not an enclosed corporate clone.
//! - **Manufactured Property (`ManufacturedProperty`)**: Compiled as a **`Construct`** / **`Artifact`**.
//!   Legitimate domain of engineering specifications, ownership, and maintenance regimes.
//! - **Liberal Arts Work (`LiberalArtsWork`)**: Compiled as a creative **`Expression`** / **`Lore`**.
//!
//! Enforces:
//! 1. **Reverse-LOD Order**: Sequences binary chunks for zero-latency OS `mmap` page-in:
//!    - Header & CBOR-LD Manifest (instant cryptographic verification)
//!    - Section 20: SURF1 Hierarchical Surfels (far-field macro view)
//!    - Section 19: IMPO1 Octahedral Atlases (mid-to-far field proxy)
//!    - Section 2: Decimated Meshlets (mid-range geometry)
//!    - Section 1: Hero Meshlets (close-up detailed geometry)
//! 2. **Deterministic Validation Gates**: Spectral energy plausibility, geometry confinement,
//!    ontological integrity, and anti-commodification human rights.
//! 3. **CBOR-LD Manifest**: W3C Verifiable Credential binding DIDs, affordances, and
//!    geometry digests.

use super::cbor_ld_manifest::{
    AcousticAnchors, CborLdManifest, CommonsObligation, GeometryLayout, SectionLayoutEntry,
    CBOR_LD_MANIFEST_ENTRY_KEY, CBOR_LD_MANIFEST_KIND,
};
use super::format::{align_up, BundleError, BUNDLE_ENTRY_ALIGN};
use super::writer::BundleWriter;
use crate::render::entity_compile_gates::{
    assert_biosphere_permissive_commons, assert_person_anti_commodification,
    validate_ontological_integrity, validate_script_determinism, OntologicalClass,
};

/// Input payload configuration for compiling an ontological entity bundle.
pub struct EntityCompileInput {
    pub credential_id: String,
    pub issuer_did: String,
    pub issuance_date: String,
    pub subject_id: String,
    pub entity_type: String, // "DarwinianTaxon", "NaturalPerson", "ManufacturedProperty", etc.
    pub procedural_seed: String,
    pub acoustic_anchors: AcousticAnchors,
    pub verification_method: String,
    pub proof_value: String,
    pub commons_obligation: Option<CommonsObligation>,

    // Reverse-LOD Section Payloads
    pub surf1_bytes: Vec<u8>,
    pub impo1_bytes: Vec<u8>,
    pub mesh2_decimated_bytes: Option<Vec<u8>>,
    pub mesh1_hero_bytes: Vec<u8>,
}

// Backwards compatibility alias
pub type TwinCompileInput = EntityCompileInput;

/// Output of ontological entity compilation.
pub struct EntityCompileOutput {
    pub bundle_bytes: Vec<u8>,
    pub manifest: CborLdManifest,
    pub manifest_cbor: Vec<u8>,
    pub manifest_json: String,
    pub total_size: usize,
    pub embodiment_term: &'static str,
}

// Backwards compatibility alias
pub type TwinCompileOutput = EntityCompileOutput;

/// Compile and seal an ontological entity bundle with validation gates and Reverse-LOD layout.
pub fn compile_entity(input: EntityCompileInput) -> Result<EntityCompileOutput, BundleError> {
    // ── 1. Gate Validations & Anti-Commodification Check ──
    let ontological_class = OntologicalClass::from_str_name(&input.entity_type)
        .unwrap_or(OntologicalClass::DarwinianTaxon);

    // Assert that a NaturalPerson is never compiled as manufactured property
    assert_person_anti_commodification(
        ontological_class,
        input.entity_type.eq_ignore_ascii_case("manufacturedproperty")
            || input.entity_type.eq_ignore_ascii_case("asset"),
    )
    .map_err(|e| BundleError::Cbor(format!("anti-commodification gate failure: {e:?}")))?;

    // Assert that living biosphere entities are governed by Permissive Commons with no perpetual fees
    let licence_class = input
        .commons_obligation
        .as_ref()
        .map(|c| c.licence_class.as_str())
        .unwrap_or("SocialPermissiveCommons");
    let has_perpetual_fees = input
        .commons_obligation
        .as_ref()
        .map(|c| !c.perpetual_fees_prohibited)
        .unwrap_or(false);
    assert_biosphere_permissive_commons(ontological_class, licence_class, has_perpetual_fees)
        .map_err(|e| BundleError::Cbor(format!("permissive commons gate failure: {e:?}")))?;

    // Validate ontological integrity
    validate_ontological_integrity(ontological_class as u8, Some(1.0))
        .map_err(|e| BundleError::Cbor(format!("gate 3 failure: {e:?}")))?;

    // Validate script determinism on procedural seed
    validate_script_determinism(250_000, false)
        .map_err(|e| BundleError::Cbor(format!("gate 4 failure: {e:?}")))?;

    // ── 2. Calculate Offsets & Reverse-LOD Layout ──
    let surf1_len = input.surf1_bytes.len() as u64;
    let impo1_len = input.impo1_bytes.len() as u64;
    let mesh1_len = input.mesh1_hero_bytes.len() as u64;
    let mesh2_len = input.mesh2_decimated_bytes.as_ref().map(|b| b.len() as u64);

    let surf1_digest = CborLdManifest::compute_sha256_digest(&input.surf1_bytes);
    let impo1_digest = CborLdManifest::compute_sha256_digest(&input.impo1_bytes);
    let mesh1_digest = CborLdManifest::compute_sha256_digest(&input.mesh1_hero_bytes);
    let mesh2_digest = input
        .mesh2_decimated_bytes
        .as_ref()
        .map(|b| CborLdManifest::compute_sha256_digest(b));

    // Construct preliminary manifest to determine CBOR byte size
    let dummy_layout = GeometryLayout {
        surf1: SectionLayoutEntry {
            offset: 0,
            length: surf1_len,
            digest: surf1_digest.clone(),
        },
        impo1: SectionLayoutEntry {
            offset: 0,
            length: impo1_len,
            digest: impo1_digest.clone(),
        },
        mesh1: SectionLayoutEntry {
            offset: 0,
            length: mesh1_len,
            digest: mesh1_digest.clone(),
        },
        mesh2: mesh2_len.map(|len| SectionLayoutEntry {
            offset: 0,
            length: len,
            digest: mesh2_digest.clone().unwrap_or_default(),
        }),
    };

    let mut temp_manifest = build_typed_manifest(
        ontological_class,
        &input.credential_id,
        &input.issuer_did,
        &input.issuance_date,
        &input.subject_id,
        &input.procedural_seed,
        input.acoustic_anchors.clone(),
        dummy_layout,
        &input.verification_method,
        &input.proof_value,
    );
    if let Some(ref cob) = input.commons_obligation {
        temp_manifest.credential_subject.commons_obligation = Some(cob.clone());
    }
    let temp_cbor = temp_manifest.encode_to_cbor()?;

    // Compute exact offsets
    let mut current_offset = align_up(super::format::BUNDLE_HEADER_SIZE, BUNDLE_ENTRY_ALIGN) as u64;
    let _manifest_offset = current_offset;
    current_offset += align_up(temp_cbor.len(), BUNDLE_ENTRY_ALIGN) as u64;

    let surf1_offset = current_offset;
    current_offset += align_up(surf1_len as usize, BUNDLE_ENTRY_ALIGN) as u64;

    let impo1_offset = current_offset;
    current_offset += align_up(impo1_len as usize, BUNDLE_ENTRY_ALIGN) as u64;

    let mesh2_offset = if let Some(m2_len) = mesh2_len {
        let off = current_offset;
        current_offset += align_up(m2_len as usize, BUNDLE_ENTRY_ALIGN) as u64;
        Some(off)
    } else {
        None
    };

    let mesh1_offset = current_offset;

    // Build final manifest with exact physical offsets
    let final_layout = GeometryLayout {
        surf1: SectionLayoutEntry {
            offset: surf1_offset,
            length: surf1_len,
            digest: surf1_digest,
        },
        impo1: SectionLayoutEntry {
            offset: impo1_offset,
            length: impo1_len,
            digest: impo1_digest,
        },
        mesh1: SectionLayoutEntry {
            offset: mesh1_offset,
            length: mesh1_len,
            digest: mesh1_digest,
        },
        mesh2: mesh2_offset.map(|off| SectionLayoutEntry {
            offset: off,
            length: mesh2_len.unwrap_or(0),
            digest: mesh2_digest.unwrap_or_default(),
        }),
    };

    let mut final_manifest = build_typed_manifest(
        ontological_class,
        &input.credential_id,
        &input.issuer_did,
        &input.issuance_date,
        &input.subject_id,
        &input.procedural_seed,
        input.acoustic_anchors,
        final_layout,
        &input.verification_method,
        &input.proof_value,
    );
    if let Some(ref cob) = input.commons_obligation {
        final_manifest.credential_subject.commons_obligation = Some(cob.clone());
    }

    let manifest_cbor = final_manifest.encode_to_cbor()?;
    let manifest_json = final_manifest.encode_to_json()?;

    // ── 3. Assemble Bundle in Reverse-LOD Order ──
    let mut writer = BundleWriter::new();

    // 1. Manifest
    writer.add_file(
        CBOR_LD_MANIFEST_ENTRY_KEY,
        CBOR_LD_MANIFEST_KIND,
        manifest_cbor.clone(),
        None,
    )?;

    // 2. Section 20: SURF1 Hierarchical Surfels
    writer.add_file(
        "sections/section20_surf1.bin",
        "surf1",
        input.surf1_bytes,
        None,
    )?;

    // 3. Section 19: IMPO1 Octahedral Atlases
    writer.add_file(
        "sections/section19_impo1.bin",
        "impo1",
        input.impo1_bytes,
        None,
    )?;

    // 4. Section 2: Decimated Meshlets (optional)
    if let Some(mesh2) = input.mesh2_decimated_bytes {
        writer.add_file(
            "sections/section02_mesh2_decimated.bin",
            "mesh2",
            mesh2,
            None,
        )?;
    }

    // 5. Section 1: Hero Meshlets
    writer.add_file(
        "sections/section01_mesh1_hero.bin",
        "mesh1",
        input.mesh1_hero_bytes,
        None,
    )?;

    let bundle_bytes = writer.build()?;
    let total_size = bundle_bytes.len();

    Ok(EntityCompileOutput {
        bundle_bytes,
        manifest: final_manifest,
        manifest_cbor,
        manifest_json,
        total_size,
        embodiment_term: ontological_class.embodiment_term(),
    })
}

// Backwards compatibility alias
pub use compile_entity as compile_digital_twin;

fn build_typed_manifest(
    ontological_class: OntologicalClass,
    credential_id: &str,
    issuer_did: &str,
    issuance_date: &str,
    subject_id: &str,
    procedural_seed: &str,
    acoustic_anchors: AcousticAnchors,
    geometry_layout: GeometryLayout,
    verification_method: &str,
    proof_value: &str,
) -> CborLdManifest {
    match ontological_class {
        OntologicalClass::DarwinianTaxon => CborLdManifest::new_specimen(
            credential_id,
            issuer_did,
            issuance_date,
            subject_id,
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        ),
        OntologicalClass::NaturalPerson => CborLdManifest::new_persona(
            credential_id,
            issuer_did,
            issuance_date,
            subject_id,
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        ),
        OntologicalClass::ManufacturedProperty => CborLdManifest::new_construct(
            credential_id,
            issuer_did,
            issuance_date,
            subject_id,
            procedural_seed,
            acoustic_anchors,
            geometry_layout,
            verification_method,
            proof_value,
        ),
        OntologicalClass::LiberalArtsWork => {
            let mut m = CborLdManifest::new(
                credential_id,
                issuer_did,
                issuance_date,
                subject_id,
                "LiberalArtsWork",
                procedural_seed,
                acoustic_anchors,
                geometry_layout,
                verification_method,
                proof_value,
            );
            m.credential_type = vec!["VerifiableCredential".into(), "CulturalExpression".into()];
            m
        }
        OntologicalClass::AbioticEnvironment => {
            let mut m = CborLdManifest::new(
                credential_id,
                issuer_did,
                issuance_date,
                subject_id,
                "AbioticEnvironment",
                procedural_seed,
                acoustic_anchors,
                geometry_layout,
                verification_method,
                proof_value,
            );
            m.credential_type = vec!["VerifiableCredential".into(), "EnvironmentalFeature".into()];
            m.credential_subject.commons_obligation = Some(CommonsObligation {
                licence_class: "SocialPermissiveCommons".into(),
                production_cost_cents: 0,
                roi_cap_percent: 20,
                accumulated_pool_cents: 0,
                transaction_cost_per_mib_cents: 0,
                perpetual_fees_prohibited: true,
            });
            m
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxon_compilation_emits_specimen() {
        let input = EntityCompileInput {
            credential_id: "urn:uuid:eucalyptus-specimen-01".to_string(),
            issuer_did: "did:web:trustfactory.org:compiler".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:taxon:eucalyptus_regnans_01".to_string(),
            entity_type: "DarwinianTaxon".to_string(),
            procedural_seed: "hmc://seeds/voronoi_bark".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:trustfactory.org:compiler#key-1".to_string(),
            proof_value: "z58DValidProofValue2026".to_string(),
            commons_obligation: None,
            surf1_bytes: vec![0x14; 2048],
            impo1_bytes: vec![0x13; 8192],
            mesh2_decimated_bytes: Some(vec![0x02; 16384]),
            mesh1_hero_bytes: vec![0x01; 65536],
        };

        let output = compile_entity(input).expect("specimen compile must succeed");
        assert_eq!(output.embodiment_term, "Specimen");
        assert!(output.manifest.credential_type.contains(&"TaxonSpecimen".to_string()));
        assert!(output.manifest.credential_subject.commons_obligation.is_some());
    }

    #[test]
    fn test_natural_person_compilation_emits_persona_and_blocks_asset_commodification() {
        let input = EntityCompileInput {
            credential_id: "urn:uuid:citizen-persona-01".to_string(),
            issuer_did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            entity_type: "NaturalPerson".to_string(),
            procedural_seed: "hmc://seeds/citizen_face".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK#key-1".to_string(),
            proof_value: "z58DValidProofValue2026".to_string(),
            commons_obligation: None,
            surf1_bytes: vec![0x14; 1024],
            impo1_bytes: vec![0x13; 4096],
            mesh2_decimated_bytes: None,
            mesh1_hero_bytes: vec![0x01; 32768],
        };

        let output = compile_entity(input).expect("persona compile must succeed");
        assert_eq!(output.embodiment_term, "Persona");
        assert!(output.manifest.credential_type.contains(&"HumanPersona".to_string()));
    }

    #[test]
    fn test_manufactured_property_compilation_emits_construct() {
        let input = EntityCompileInput {
            credential_id: "urn:uuid:solar-pump-construct-01".to_string(),
            issuer_did: "did:web:trustfactory.org:compiler".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:artifact:solar_pump_400x".to_string(),
            entity_type: "ManufacturedProperty".to_string(),
            procedural_seed: "hmc://seeds/stress_fracture".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:trustfactory.org:compiler#key-1".to_string(),
            proof_value: "z58DValidProofValue2026".to_string(),
            commons_obligation: None,
            surf1_bytes: vec![0x14; 512],
            impo1_bytes: vec![0x13; 2048],
            mesh2_decimated_bytes: Some(vec![0x02; 4096]),
            mesh1_hero_bytes: vec![0x01; 16384],
        };

        let output = compile_entity(input).expect("construct compile must succeed");
        assert_eq!(output.embodiment_term, "Construct");
        assert!(output.manifest.credential_type.contains(&"EngineeredConstruct".to_string()));
    }

    #[test]
    fn test_biosphere_permissive_commons_compilation_enforcement() {
        // 1. Rejection of proprietary license on living species
        let proprietary_input = EntityCompileInput {
            credential_id: "urn:uuid:enclosure-attempt".to_string(),
            issuer_did: "did:web:bad-actor.com".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:taxon:eucalyptus_enclosed".to_string(),
            entity_type: "DarwinianTaxon".to_string(),
            procedural_seed: "hmc://seeds/bark".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:bad-actor.com#key-1".to_string(),
            proof_value: "zProof".to_string(),
            commons_obligation: Some(CommonsObligation {
                licence_class: "AllRightsReservedProprietary".to_string(),
                production_cost_cents: 100_000,
                roi_cap_percent: 200,
                accumulated_pool_cents: 0,
                transaction_cost_per_mib_cents: 50,
                perpetual_fees_prohibited: true,
            }),
            surf1_bytes: vec![0x14; 512],
            impo1_bytes: vec![0x13; 2048],
            mesh2_decimated_bytes: None,
            mesh1_hero_bytes: vec![0x01; 16384],
        };
        assert!(compile_entity(proprietary_input).is_err());

        // 2. Rejection of perpetual royalties on living species
        let perpetual_fees_input = EntityCompileInput {
            credential_id: "urn:uuid:perpetual-toll-attempt".to_string(),
            issuer_did: "did:web:bad-actor.com".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:taxon:eucalyptus_rent_toll".to_string(),
            entity_type: "DarwinianTaxon".to_string(),
            procedural_seed: "hmc://seeds/bark".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:bad-actor.com#key-1".to_string(),
            proof_value: "zProof".to_string(),
            commons_obligation: Some(CommonsObligation {
                licence_class: "SocialPermissiveCommons".to_string(),
                production_cost_cents: 50_000,
                roi_cap_percent: 20,
                accumulated_pool_cents: 0,
                transaction_cost_per_mib_cents: 5,
                perpetual_fees_prohibited: false, // Breach! Attempting perpetual royalties
            }),
            surf1_bytes: vec![0x14; 512],
            impo1_bytes: vec![0x13; 2048],
            mesh2_decimated_bytes: None,
            mesh1_hero_bytes: vec![0x01; 16384],
        };
        assert!(compile_entity(perpetual_fees_input).is_err());

        // 3. Permissive Commons with finite production obligation succeeds
        let valid_commons_input = EntityCompileInput {
            credential_id: "urn:uuid:valid-spc-specimen".to_string(),
            issuer_did: "did:web:trustfactory.org:compiler".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:taxon:eucalyptus_permissive".to_string(),
            entity_type: "DarwinianTaxon".to_string(),
            procedural_seed: "hmc://seeds/bark".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:trustfactory.org:compiler#key-1".to_string(),
            proof_value: "zProof".to_string(),
            commons_obligation: Some(CommonsObligation {
                licence_class: "SocialPermissiveCommons".to_string(),
                production_cost_cents: 25_000, // $250.00 finite digitization cost
                roi_cap_percent: 20,
                accumulated_pool_cents: 30_000, // fully discharged!
                transaction_cost_per_mib_cents: 2, // at-cost carrier transport
                perpetual_fees_prohibited: true,
            }),
            surf1_bytes: vec![0x14; 512],
            impo1_bytes: vec![0x13; 2048],
            mesh2_decimated_bytes: None,
            mesh1_hero_bytes: vec![0x01; 16384],
        };
        let out = compile_entity(valid_commons_input).expect("valid SPC must compile");
        let cob = out.manifest.credential_subject.commons_obligation.expect("must have cob");
        assert!(cob.is_discharged());
        assert_eq!(cob.outstanding_obligation(), 0);
    }
}
