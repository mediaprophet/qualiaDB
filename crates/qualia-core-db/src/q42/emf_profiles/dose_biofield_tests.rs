use super::super::field_envelope::{Family, Limits as StoreLimits, StructuralView};
use super::super::field_semantics::NodeState;
use super::*;

fn test_clock() -> FieldClock {
    FieldClock {
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 1000,
        ticks_per_second_den: 1,
    }
}

fn test_frame() -> FieldFrame {
    FieldFrame {
        origin_m: [0.0, 0.0, 0.0],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        grid: None,
    }
}

fn test_prov() -> FieldProvenance<'static> {
    FieldProvenance {
        role: 2,
        algorithm: Some("transport-dose-biofield"),
        algorithm_version: Some("1.0"),
        lineage: Some("test-e06"),
    }
}

fn test_cal(unit: &'static str, validity: [i64; 2]) -> CalibrationSpec<'static> {
    CalibrationSpec {
        instrument_identifier: "detector-array-0",
        content_digest: [0x67; 32],
        quantity_unit: unit,
        scale: 1.0,
        offset: 0.0,
        uncertainty: Some((0.03, "k=1_combined")),
        validity_ticks: Some(validity),
    }
}

#[test]
fn dose_record_crc_validity_flags_zero_on_absent_and_photon_vs_particle_physics() {
    let rec = DoseTransportRecord {
        position_m: [0.1, 0.2, 0.3],
        tick: 50,
        photon_energy_j: Some(kev_to_joules(100.0).unwrap()),
        linear_attenuation_per_m: Some(17.1),
        mass_attenuation_m2_per_kg: Some(0.0171),
        photon_fluence_per_m2: Some(1.0e12),
        absorbed_dose_gy: Some(absorbed_dose_gy(0.05, 2.0).unwrap()),
        equivalent_dose_sv: None,
        linear_energy_transfer_j_per_m: None,
        uncertainty: Some(0.001),
        material_entry: 5,
        calibration_ref: 6,
        model_ref: 7,
        validity_flags: DOSE_VALID_PHOTON_ENERGY
            | DOSE_VALID_LINEAR_ATTEN
            | DOSE_VALID_MASS_ATTEN
            | DOSE_VALID_FLUENCE
            | DOSE_VALID_ABSORBED_GY
            | DOSE_VALID_UNCERTAINTY,
    };
    let mut bytes = [0u8; 128];
    rec.encode(true, &mut bytes).unwrap();
    let parsed = DoseTransportRecord::parse(&bytes, true).unwrap().unwrap();
    assert_eq!(parsed, rec);

    // Non-zero bytes (including -0.0) in an absent lane (equivalent_dose_sv offset 72..80) reject Reserved!
    let mut bad_absent = bytes;
    bad_absent[72..80].copy_from_slice(&(-0.0f64).to_le_bytes());
    let new_crc = crate::container_10d::crc32c::crc32c(&bad_absent[..124]);
    bad_absent[124..128].copy_from_slice(&new_crc.to_le_bytes());
    assert_eq!(DoseTransportRecord::parse(&bad_absent, true), Err(Error::Reserved));

    // Bit-flip breaks CRC.
    let mut bad_crc = bytes;
    bad_crc[10] ^= 0x80;
    assert_eq!(DoseTransportRecord::parse(&bad_crc, true), Err(Error::Reserved));

    // Photon attenuation vs charged particle rejection.
    let mu = linear_from_mass_attenuation(0.0171, 1000.0).unwrap();
    assert!((mu - 17.1).abs() < 1.0e-12);
    let flu = photon_uncollided_fluence(TransportSpecies::Photon, 1.0e12, mu, 0.05).unwrap();
    assert!((flu - 1.0e12 * (-0.855f64).exp()).abs() / flu < 1.0e-12);
    assert_eq!(
        photon_uncollided_fluence(TransportSpecies::Proton, 1.0e12, mu, 0.05),
        Err(Error::UnsupportedProfile)
    );
    assert!((equivalent_dose_sv(0.025, 20.0).unwrap() - 0.5).abs() < 1.0e-12);
}

#[test]
fn dose_0x0106_envelope_roundtrip_sv_weighting_and_charged_particle_rejects() {
    let dose_rec = DoseTransportRecord {
        position_m: [0.0, 0.0, 1.0],
        tick: 15,
        photon_energy_j: Some(kev_to_joules(662.0).unwrap()),
        linear_attenuation_per_m: Some(8.57),
        mass_attenuation_m2_per_kg: Some(0.00857),
        photon_fluence_per_m2: Some(5.0e11),
        absorbed_dose_gy: Some(0.12),
        equivalent_dose_sv: Some(equivalent_dose_sv(0.12, 1.0).unwrap()),
        linear_energy_transfer_j_per_m: None,
        uncertainty: Some(0.004),
        material_entry: 3,
        calibration_ref: 6,
        model_ref: 7,
        validity_flags: DOSE_VALID_PHOTON_ENERGY
            | DOSE_VALID_LINEAR_ATTEN
            | DOSE_VALID_MASS_ATTEN
            | DOSE_VALID_FLUENCE
            | DOSE_VALID_ABSORBED_GY
            | DOSE_VALID_EQUIVALENT_SV
            | DOSE_VALID_UNCERTAINTY,
    };
    let mut payload = [0u8; 128];
    encode_dose_records(&[dose_rec], None, &mut payload).unwrap();

    let spec = ExtendedModeSpec::Dose {
        evidence: ModelEvidenceSpec {
            model_identifier_version: "geant4-mc-em-v11",
            domain_applicability: "photon_tissue_kerma_0.01_10MeV",
            parameter_source_digest: [0x19; 32],
            numerical_convention: "icrp103_si",
            epistemic_class: "model_estimate",
            uncertainty: Some("3% MonteCarlo 1sigma"),
            external_reference: Some("ICRP-103"),
            qualification_receipt_id: None,
            particle_species: Some("photon"),
            integration_ticks: Some([10, 20]),
            weighting_derating_model: Some("ICRP-103 w_R=1 photon"),
        },
        calibration: test_cal("Gy", [10, 25]),
        material: Some(MaterialSpec {
            container_digest: [0xaa; 32],
            table_digest: [0xbb; 32],
            table_epoch: 1,
            table_schema_version: "v1",
            material_ids: &[3],
        }),
    };

    for mode_id in [0x0106u16, 6u16] {
        let mut mb = [0u8; 4096];
        let mut st = [0u8; 8192];
        let mut out = [0u8; 8192];
        let mut md = [NodeState::default(); 32];
        let mut ed = [NodeState::default(); 8];
        let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
        let bytes = write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            mode_id,
            Some(5),
            15,
            1,
            None,
            1,
            &payload,
            &spec,
            &mut mb,
            &mut st,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap();
        let view = StructuralView::open(bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 65536).unwrap();
        let chunk = reader.chunk(1).unwrap();
        assert_eq!(chunk.dose(0).unwrap().unwrap(), dose_rec);
        assert!(chunk.biofield(0).is_err());
    }

    // 1. Equivalent dose Sv without weighting_derating_model fails Descriptor.
    let spec_no_wr = ExtendedModeSpec::Dose {
        evidence: ModelEvidenceSpec {
            model_identifier_version: "geant4-mc-em-v11",
            domain_applicability: "photon",
            parameter_source_digest: [0x19; 32],
            numerical_convention: "icrp103_si",
            epistemic_class: "model_estimate",
            uncertainty: Some("3%"),
            external_reference: None,
            qualification_receipt_id: None,
            particle_species: Some("photon"),
            integration_ticks: Some([10, 20]),
            weighting_derating_model: None,
        },
        calibration: test_cal("Gy", [10, 25]),
        material: None,
    };
    let mut mb = [0u8; 4096];
    let mut st = [0u8; 8192];
    let mut out = [0u8; 8192];
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    assert_eq!(
        write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            0x0106,
            Some(5),
            15,
            1,
            None,
            1,
            &payload,
            &spec_no_wr,
            &mut mb,
            &mut st,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap_err(),
        Error::Descriptor
    );

    // 2. Charged particle ("proton") carrying Beer-Lambert attenuation fails UnsupportedProfile.
    let spec_proton = ExtendedModeSpec::Dose {
        evidence: ModelEvidenceSpec {
            model_identifier_version: "geant4-hadron-v11",
            domain_applicability: "proton",
            parameter_source_digest: [0x19; 32],
            numerical_convention: "si",
            epistemic_class: "model_estimate",
            uncertainty: Some("3%"),
            external_reference: None,
            qualification_receipt_id: None,
            particle_species: Some("proton"),
            integration_ticks: Some([10, 20]),
            weighting_derating_model: Some("ICRP-103 w_R=2"),
        },
        calibration: test_cal("Gy", [10, 25]),
        material: None,
    };
    assert_eq!(
        write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            0x0106,
            Some(5),
            15,
            1,
            None,
            1,
            &payload,
            &spec_proton,
            &mut mb,
            &mut st,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap_err(),
        Error::UnsupportedProfile
    );
}

#[test]
fn biofield_dipole_potential_epistemic_isolation_and_0x0107_roundtrip() {
    // 1. Quasi-static point current dipole analytical check and singularity rejection.
    let phi = dipole_potential_infinite_homogeneous(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0e-7],
        [0.0, 0.0, 0.02],
        0.33,
    )
    .unwrap();
    let expected = (1.0e-7 * 0.02) / (4.0 * core::f64::consts::PI * 0.33 * 0.02f64.powi(3));
    assert!((phi - expected).abs() / expected < 1.0e-12);

    assert_eq!(
        dipole_potential_infinite_homogeneous([1.0, 2.0, 3.0], [1.0, 0.0, 0.0], [1.0, 2.0, 3.0], 0.33),
        Err(Error::Value)
    );

    // 2. Biofield 0x0107 and candidate 7 roundtrip with consent receipt digest & epistemic check.
    let bio_rec = BiofieldRecord {
        position_m: [0.01, -0.02, 0.05],
        current_dipole_moment_a_m: [1.0e-8, -2.0e-8, 3.0e-8],
        extracellular_potential_v: Some(phi),
        tick: 12,
        carrier_hz: Some(40.0),
        uncertainty: Some(1.0e-6),
        anatomy_metadata_ref: 8,
        model_ref: 7,
        calibration_ref: 6,
        consent_provenance_ref: 9,
        flags: BIO_FLAG_POTENTIAL | BIO_FLAG_CARRIER | BIO_FLAG_UNCERTAINTY,
    };
    let mut payload = [0u8; 128];
    encode_biofield_records(&[bio_rec], None, &mut payload).unwrap();
    assert_eq!(BiofieldRecord::parse(&payload, true).unwrap().unwrap(), bio_rec);

    let spec = ExtendedModeSpec::Biofield {
        bio: BioContextSpec {
            anatomical_ontology_version: "FMA-5.0",
            anatomical_identifier: "FMA:50801",
            observation_type: "current_dipole",
            consent_receipt_digest: Some([0x7c; 32]),
        },
        evidence: ModelEvidenceSpec {
            model_identifier_version: "meg-forward-homog-v1",
            domain_applicability: "quasi_static_cortical_dipole",
            parameter_source_digest: [0x22; 32],
            numerical_convention: "si_dipole_a_m",
            epistemic_class: "empirical",
            uncertainty: Some("1e-6 V 1sigma"),
            external_reference: None,
            qualification_receipt_id: Some("irb-2026-01"),
            particle_species: None,
            integration_ticks: Some([0, 50]),
            weighting_derating_model: None,
        },
        calibration: test_cal("A m", [0, 50]),
    };

    for mode_id in [0x0107u16, 7u16] {
        let mut mb = [0u8; 4096];
        let mut st = [0u8; 8192];
        let mut out = [0u8; 8192];
        let mut md = [NodeState::default(); 32];
        let mut ed = [NodeState::default(); 8];
        let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
        let written = write_extended_envelope(
            test_clock(),
            test_frame(),
            test_prov(),
            mode_id,
            Some(1),
            12,
            1,
            None,
            1,
            &payload,
            &spec,
            &mut mb,
            &mut st,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap();
        let view = StructuralView::open(written, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 65536).unwrap();
        let chunk = reader.chunk(1).unwrap();
        assert_eq!(chunk.biofield(0).unwrap().unwrap(), bio_rec);

        // Verify empirical join succeeds on admitted empirical Schema 134 block (id=7).
        let model_blk = view.blocks().find(|b| b.id() == 7).unwrap();
        assert_eq!(verify_empirical_join_allowed(model_blk).unwrap(), super::schemas::EpistemicClass::Empirical);
    }

    // 3. Speculative model ("hypothesis") still stores cleanly if valid, but fails empirical join check!
    let spec_hyp = ExtendedModeSpec::Biofield {
        bio: BioContextSpec {
            anatomical_ontology_version: "FMA-5.0",
            anatomical_identifier: "FMA:50801",
            observation_type: "model_construct",
            consent_receipt_digest: Some([0x7c; 32]),
        },
        evidence: ModelEvidenceSpec {
            model_identifier_version: "hyp-dipole-v1",
            domain_applicability: "exploratory_hypothesis",
            parameter_source_digest: [0x22; 32],
            numerical_convention: "si_dipole_a_m",
            epistemic_class: "hypothesis",
            uncertainty: Some("model_bound"),
            external_reference: None,
            qualification_receipt_id: None,
            particle_species: None,
            integration_ticks: Some([0, 50]),
            weighting_derating_model: None,
        },
        calibration: test_cal("A m", [0, 50]),
    };
    let mut mb = [0u8; 4096];
    let mut st = [0u8; 8192];
    let mut out = [0u8; 8192];
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    let hyp_bytes = write_extended_envelope(
        test_clock(),
        test_frame(),
        test_prov(),
        0x0107,
        Some(1),
        12,
        1,
        None,
        1,
        &payload,
        &spec_hyp,
        &mut mb,
        &mut st,
        &mut out,
        &mut scratch,
        FieldWriteLimits::default(),
    )
    .unwrap();
    let hyp_view = StructuralView::open(hyp_bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let hyp_blk = hyp_view.blocks().find(|b| b.id() == 7).unwrap();
    assert_eq!(verify_empirical_join_allowed(hyp_blk), Err(Error::UnsupportedProfile));
}
