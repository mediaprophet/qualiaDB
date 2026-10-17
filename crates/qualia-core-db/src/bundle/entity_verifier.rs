//! Automated Ontological Entity Verification Engine (.10d & .hmc).
//!
//! Provides zero-heap and bounded validation for both CLI (native multi-threading)
//! and WebAssembly (browser/VibeScript), allowing autonomous software agents to
//! verify thousands of species/artifacts against:
//! 1. Reverse-LOD layout (Section 20 SURF1 -> Section 19 IMPO1 -> Meshlets -> Hero)
//! 2. Continuous spectral reflectance parameters and 3-band acoustic absorption
//! 3. Cryptographic CRC-32C and SHA-256 digest integrity
//! 4. CBOR-LD Verifiable Credential manifests and W3C DIDs
//! 5. Ontological embodiment taxonomy and anti-commodification human rights gates.

use serde::{Deserialize, Serialize};

use super::cbor_ld_manifest::{
    AcousticAnchors, CborLdManifest, CBOR_LD_MANIFEST_ENTRY_KEY, CBOR_LD_MANIFEST_LEGACY_KEY,
};
use super::reader::BundleReader;
use crate::container_10d::header::{Container10dHeader, MAGIC_10D};
use crate::container_10d::impostor_section::decode_impostor_section;
use crate::container_10d::integrity::check_whole_file_crc32c;
use crate::container_10d::section::{parse_section_table, SectionType};
use crate::container_10d::surfel_section::decode_surfel_section;
use crate::render::compile_10d::decode_10d_mesh;
use crate::render::entity_compile_gates::{
    assert_biosphere_permissive_commons, assert_person_anti_commodification, OntologicalClass,
};

/// Verification summary for Section 20 (`SURF1`) hierarchical surfels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfelVerificationSummary {
    pub surfel_count: u32,
    pub stride: u32,
    pub sphere_center: [f32; 3],
    pub sphere_radius: f32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
}

/// Verification summary for Section 19 (`IMPO1`) octahedral impostor atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpostorVerificationSummary {
    pub grid_dimension: u16,
    pub view_resolution: u16,
    pub sphere_center: [f32; 3],
    pub sphere_radius: f32,
}

/// Verification summary for Section 17 (`EONT1`) entity affordance ontology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffordanceVerificationSummary {
    pub entity_did_hash: u64,
    pub taxon_iri_hash: u64,
    pub affordance_count: u32,
    pub energy_permill: u16,
    pub damage_permill: u16,
}

/// Comprehensive report on a single `.10d` living container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity10dVerification {
    pub valid: bool,
    pub header_version: u16,
    pub crc32c_valid: bool,
    pub section_count: usize,
    pub sections_found: Vec<String>,
    pub vertices: usize,
    pub triangles: usize,
    pub surfels: Option<SurfelVerificationSummary>,
    pub impostors: Option<ImpostorVerificationSummary>,
    pub affordances: Option<AffordanceVerificationSummary>,
    pub reverse_lod_valid: bool,
    pub min_bounds: [f32; 3],
    pub max_bounds: [f32; 3],
    pub violations: Vec<String>,
}

/// Verification summary for Humanitarian ICT & Permissive Commons terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonsObligationSummary {
    pub licence_class: String,
    pub production_cost_cents: u64,
    pub roi_cap_percent: u64,
    pub accumulated_pool_cents: u64,
    pub transaction_cost_per_mib_cents: u64,
    pub perpetual_fees_prohibited: bool,
    pub is_discharged: bool,
    pub outstanding_cents: u64,
}

/// Comprehensive report on a `.hmc` bundle containing CBOR-LD manifest and assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityBundleVerification {
    pub valid: bool,
    pub bundle_crc32c_valid: bool,
    pub entry_count: usize,
    pub entries: Vec<String>,
    pub manifest_present: bool,
    pub credential_id: Option<String>,
    pub issuer_did: Option<String>,
    pub subject_id: Option<String>,
    pub entity_type: Option<String>,
    pub acoustic_anchors: Option<AcousticAnchors>,
    pub anti_commodification_passed: bool,
    pub permissive_commons_passed: bool,
    pub commons_obligation: Option<CommonsObligationSummary>,
    pub geometry_digests_match: bool,
    pub contained_10d_entities: Vec<Entity10dVerification>,
    pub violations: Vec<String>,
}

// Backwards compatibility aliases
pub type DigitalTwin10dVerification = Entity10dVerification;
pub type DigitalTwinBundleVerification = EntityBundleVerification;

/// Verify an in-memory `.10d` container payload.
pub fn verify_entity_10d(bytes: &[u8]) -> Entity10dVerification {
    let mut violations = Vec::new();
    let mut sections_found = Vec::new();
    let mut surfels = None;
    let mut impostors = None;
    let mut affordances = None;
    let mut vertices = 0;
    let mut triangles = 0;
    let mut min_bounds = [0.0; 3];
    let mut max_bounds = [0.0; 3];

    if bytes.len() < 64 {
        violations.push(format!(
            "payload too short: {} bytes < 64 byte header",
            bytes.len()
        ));
        return Entity10dVerification {
            valid: false,
            header_version: 0,
            crc32c_valid: false,
            section_count: 0,
            sections_found,
            vertices,
            triangles,
            surfels,
            impostors,
            affordances,
            reverse_lod_valid: false,
            min_bounds,
            max_bounds,
            violations,
        };
    }

    // 1. Verify Header
    let header = match Container10dHeader::parse(&bytes[..64]) {
        Ok(h) => h,
        Err(e) => {
            violations.push(format!("header parse error: {e:?}"));
            return Entity10dVerification {
                valid: false,
                header_version: 0,
                crc32c_valid: false,
                section_count: 0,
                sections_found,
                vertices,
                triangles,
                surfels,
                impostors,
                affordances,
                reverse_lod_valid: false,
                min_bounds,
                max_bounds,
                violations,
            };
        }
    };

    if header.magic != MAGIC_10D {
        violations.push(format!(
            "invalid magic: expected {:?}, got {:?}",
            MAGIC_10D, header.magic
        ));
    }

    // 2. Whole-file CRC-32C verification
    let crc32c_valid = match check_whole_file_crc32c(bytes) {
        Ok(_) => true,
        Err(e) => {
            // Only report violation if CRC check actually failed on populated bytes
            if header.header_crc32c != 0 {
                violations.push(format!("whole-file crc32c mismatch: {e:?}"));
                false
            } else {
                true
            }
        }
    };

    // 3. Section Table Inspection
    let mut surfel_offset = None;
    let mut impostor_offset = None;
    let mut mesh_offset = None;

    if header.section_count > 0 {
        match parse_section_table(bytes, &header) {
            Ok(table) => {
                for desc in table {
                    let sec_type = SectionType::from_u8(desc.section_type);
                    let name = format!("{:?} (type {})", sec_type, desc.section_type);
                    sections_found.push(name);

                    let start = desc.byte_offset as usize;
                    let end = start + desc.byte_length as usize;

                    if end > bytes.len() {
                        violations.push(format!(
                            "section {:?} out of bounds: [{}..{}] vs file len {}",
                            sec_type,
                            start,
                            end,
                            bytes.len()
                        ));
                        continue;
                    }

                    let sec_bytes = &bytes[start..end];

                    match sec_type {
                        Some(SectionType::HierarchicalSurfels) => {
                            surfel_offset = Some(start);
                            match decode_surfel_section(sec_bytes) {
                                Ok((h, _records)) => {
                                    surfels = Some(SurfelVerificationSummary {
                                        surfel_count: h.surfel_count,
                                        stride: h.stride,
                                        sphere_center: h.sphere_center,
                                        sphere_radius: h.sphere_radius,
                                        aabb_min: h.aabb_min,
                                        aabb_max: h.aabb_max,
                                    });
                                }
                                Err(e) => {
                                    violations.push(format!("surfel section decode error: {e}"))
                                }
                            }
                        }
                        Some(SectionType::ImpostorAtlas) => {
                            impostor_offset = Some(start);
                            match decode_impostor_section(sec_bytes) {
                                Ok((h, _atlas)) => {
                                    impostors = Some(ImpostorVerificationSummary {
                                        grid_dimension: h.grid_dimension,
                                        view_resolution: h.view_resolution,
                                        sphere_center: h.sphere_center,
                                        sphere_radius: h.sphere_radius,
                                    });
                                }
                                Err(e) => {
                                    violations.push(format!("impostor section decode error: {e}"))
                                }
                            }
                        }
                        Some(SectionType::EntityOntology) => {
                            if sec_bytes.len() >= 32 {
                                use crate::container_10d::entity_ontology_section::EntityOntologyMiniHeader;
                                let mini: &EntityOntologyMiniHeader =
                                    bytemuck::from_bytes(&sec_bytes[..32]);
                                affordances = Some(AffordanceVerificationSummary {
                                    entity_did_hash: mini.entity_did_hash,
                                    taxon_iri_hash: mini.taxon_iri_hash,
                                    affordance_count: mini.affordance_count,
                                    energy_permill: mini.energy_permill,
                                    damage_permill: mini.damage_permill,
                                });
                            }
                        }
                        Some(SectionType::QuantizedMesh) => {
                            mesh_offset = Some(start);
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                violations.push(format!("section table parse error: {e:?}"));
            }
        }
    }

    // 4. Mesh Decoding (if mesh present)
    if let Ok(mesh) = decode_10d_mesh(bytes) {
        vertices = mesh.vertex_count();
        triangles = mesh.triangle_count();
        min_bounds = mesh.min;
        max_bounds = mesh.max;
    }

    // 5. Reverse-LOD Order Check:
    // When multiple tiers are present in a single package,
    // far-field surfels should precede mid-field impostor, which precedes close-up mesh
    let mut reverse_lod_valid = true;
    if let (Some(s_off), Some(i_off)) = (surfel_offset, impostor_offset) {
        if s_off > i_off {
            violations.push(format!(
                "Reverse-LOD layout violation: surfels (offset {s_off}) appear after impostors (offset {i_off})"
            ));
            reverse_lod_valid = false;
        }
    }
    if let (Some(i_off), Some(m_off)) = (impostor_offset, mesh_offset) {
        if i_off > m_off {
            violations.push(format!(
                "Reverse-LOD layout violation: impostors (offset {i_off}) appear after mesh (offset {m_off})"
            ));
            reverse_lod_valid = false;
        }
    }

    let valid = violations.is_empty();

    Entity10dVerification {
        valid,
        header_version: header.version,
        crc32c_valid,
        section_count: header.section_count as usize,
        sections_found,
        vertices,
        triangles,
        surfels,
        impostors,
        affordances,
        reverse_lod_valid,
        min_bounds,
        max_bounds,
        violations,
    }
}

/// Verify an in-memory `.hmc` bundle with embedded CBOR-LD manifest.
pub fn verify_entity_bundle(bytes: &[u8]) -> EntityBundleVerification {
    let mut violations = Vec::new();
    let mut entries = Vec::new();
    let mut manifest_present = false;
    let mut credential_id = None;
    let mut issuer_did = None;
    let mut subject_id = None;
    let mut entity_type = None;
    let mut acoustic_anchors = None;
    let mut anti_commodification_passed = true;
    let mut permissive_commons_passed = true;
    let mut commons_obligation = None;
    let mut geometry_digests_match = true;
    let mut contained_10d_entities = Vec::new();

    let reader = match BundleReader::parse(bytes) {
        Ok(r) => r,
        Err(e) => {
            violations.push(format!("bundle header parse failed: {e:?}"));
            return EntityBundleVerification {
                valid: false,
                bundle_crc32c_valid: false,
                entry_count: 0,
                entries,
                manifest_present: false,
                credential_id,
                issuer_did,
                subject_id,
                entity_type,
                acoustic_anchors,
                anti_commodification_passed: false,
                permissive_commons_passed: false,
                commons_obligation: None,
                geometry_digests_match: false,
                contained_10d_entities,
                violations,
            };
        }
    };

    for entry in reader.entries() {
        entries.push(entry.key.clone());
        if !reader.verify_entry(&entry.key) {
            violations.push(format!("entry {} failed sha256 checksum", entry.key));
        }

        // Verify interior .10d models
        if entry.key.ends_with(".10d") {
            if let Some(slice) = reader.get(&entry.key) {
                let ent_rep = verify_entity_10d(slice);
                if !ent_rep.valid {
                    violations.push(format!("embedded entity {} failed validation", entry.key));
                }
                contained_10d_entities.push(ent_rep);
            }
        }
    }

    // Inspect CBOR-LD manifest
    let manifest_bytes = reader
        .get(CBOR_LD_MANIFEST_ENTRY_KEY)
        .or_else(|| reader.get(CBOR_LD_MANIFEST_LEGACY_KEY));

    if let Some(m_bytes) = manifest_bytes {
        manifest_present = true;
        match ciborium::from_reader::<CborLdManifest, _>(m_bytes) {
            Ok(manifest) => {
                credential_id = Some(manifest.id.clone());
                issuer_did = Some(manifest.issuer.clone());
                subject_id = Some(manifest.credential_subject.id.clone());
                entity_type = Some(manifest.credential_subject.entity_type.clone());
                acoustic_anchors = Some(
                    manifest
                        .credential_subject
                        .affordances
                        .acoustic_anchors
                        .clone(),
                );

                // Anti-commodification gate
                let ent_class =
                    OntologicalClass::from_str_name(&manifest.credential_subject.entity_type)
                        .unwrap_or(OntologicalClass::DarwinianTaxon);

                let is_asset_claim = manifest
                    .credential_subject
                    .entity_type
                    .eq_ignore_ascii_case("manufacturedproperty")
                    || manifest
                        .credential_subject
                        .entity_type
                        .eq_ignore_ascii_case("asset");

                if let Err(e) = assert_person_anti_commodification(ent_class, is_asset_claim) {
                    anti_commodification_passed = false;
                    violations.push(format!("anti-commodification gate failure: {e:?}"));
                }

                // Humanitarian ICT & Permissive Commons gate
                if let Some(ref cob) = manifest.credential_subject.commons_obligation {
                    commons_obligation = Some(CommonsObligationSummary {
                        licence_class: cob.licence_class.clone(),
                        production_cost_cents: cob.production_cost_cents,
                        roi_cap_percent: cob.roi_cap_percent,
                        accumulated_pool_cents: cob.accumulated_pool_cents,
                        transaction_cost_per_mib_cents: cob.transaction_cost_per_mib_cents,
                        perpetual_fees_prohibited: cob.perpetual_fees_prohibited,
                        is_discharged: cob.is_discharged(),
                        outstanding_cents: cob.outstanding_obligation(),
                    });

                    if let Err(e) = assert_biosphere_permissive_commons(
                        ent_class,
                        &cob.licence_class,
                        !cob.perpetual_fees_prohibited,
                    ) {
                        permissive_commons_passed = false;
                        violations.push(format!("permissive commons gate failure: {e:?}"));
                    }
                } else if matches!(
                    ent_class,
                    OntologicalClass::DarwinianTaxon | OntologicalClass::AbioticEnvironment
                ) {
                    permissive_commons_passed = false;
                    violations.push("permissive commons gate failure: biosphere entities ('world of god') require an explicit Permissive Commons / SPC declaration".into());
                }

                // Verify geometry digests against the bundle
                match manifest.verify_geometry_digests(bytes) {
                    Ok(matches) => {
                        geometry_digests_match = matches;
                        if !matches {
                            violations.push(
                                "manifest declared geometry digests do not match binary payload"
                                    .into(),
                            );
                        }
                    }
                    Err(e) => {
                        geometry_digests_match = false;
                        violations.push(format!("geometry digest verification error: {e:?}"));
                    }
                }
            }
            Err(e) => {
                violations.push(format!("failed to deserialize CBOR-LD manifest: {e}"));
            }
        }
    }

    let valid = violations.is_empty();

    EntityBundleVerification {
        valid,
        bundle_crc32c_valid: true,
        entry_count: entries.len(),
        entries,
        manifest_present,
        credential_id,
        issuer_did,
        subject_id,
        entity_type,
        acoustic_anchors,
        anti_commodification_passed,
        permissive_commons_passed,
        commons_obligation,
        geometry_digests_match,
        contained_10d_entities,
        violations,
    }
}

// Backwards compatibility functions
/// Alias for [`verify_entity_10d`] — verifies a `.10d` digital-twin
/// container (kept for the original "digital twin" naming).
pub fn verify_digital_twin_10d(bytes: &[u8]) -> Entity10dVerification {
    verify_entity_10d(bytes)
}

/// Alias for [`verify_entity_bundle`] — verifies a `.hmc` entity bundle.
pub fn verify_digital_twin_bundle(bytes: &[u8]) -> EntityBundleVerification {
    verify_entity_bundle(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_truncated_10d_fails_cleanly() {
        let rep = verify_entity_10d(&[0u8; 10]);
        assert!(!rep.valid);
        assert!(!rep.violations.is_empty());
    }

    #[test]
    fn test_verify_empty_bundle_fails_cleanly() {
        let rep = verify_entity_bundle(&[0u8; 16]);
        assert!(!rep.valid);
        assert!(!rep.violations.is_empty());
    }

    #[test]
    fn test_verify_bundle_permissive_commons_pass_and_discharge_summary() {
        use crate::bundle::cbor_ld_manifest::CommonsObligation;
        use crate::bundle::entity_compiler::{compile_entity, EntityCompileInput};

        let input = EntityCompileInput {
            credential_id: "urn:uuid:spc-test-01".to_string(),
            issuer_did: "did:web:trustfactory.org:compiler".to_string(),
            issuance_date: "2026-10-13T08:46:00Z".to_string(),
            subject_id: "urn:qualia:taxon:eucalyptus_verified".to_string(),
            entity_type: "DarwinianTaxon".to_string(),
            procedural_seed: "hmc://seeds/bark".to_string(),
            acoustic_anchors: AcousticAnchors::default(),
            verification_method: "did:web:trustfactory.org:compiler#key-1".to_string(),
            proof_value: "zProof".to_string(),
            commons_obligation: Some(CommonsObligation {
                licence_class: "SocialPermissiveCommons".to_string(),
                production_cost_cents: 10_000, // $100.00
                roi_cap_percent: 20,
                accumulated_pool_cents: 12_000, // $120.00 (discharged)
                transaction_cost_per_mib_cents: 1, // $0.01/MiB
                perpetual_fees_prohibited: true,
            }),
            surf1_bytes: vec![0x14; 512],
            impo1_bytes: vec![0x13; 2048],
            mesh2_decimated_bytes: None,
            mesh1_hero_bytes: vec![0x01; 16384],
        };

        let output = compile_entity(input).expect("compilation should succeed");
        let rep = verify_entity_bundle(&output.bundle_bytes);

        assert!(rep.valid);
        assert!(rep.permissive_commons_passed);
        assert!(rep.anti_commodification_passed);
        assert!(rep.geometry_digests_match);
        let cob = rep
            .commons_obligation
            .expect("commons obligation must be populated");
        assert_eq!(cob.licence_class, "SocialPermissiveCommons");
        assert_eq!(cob.production_cost_cents, 10_000);
        assert_eq!(cob.accumulated_pool_cents, 12_000);
        assert!(cob.is_discharged);
        assert_eq!(cob.outstanding_cents, 0);
        assert_eq!(cob.transaction_cost_per_mib_cents, 1);
        assert!(cob.perpetual_fees_prohibited);
    }
}
