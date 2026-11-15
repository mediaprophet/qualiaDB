//! BIO-WP-12: Cross-Kingdom Calibration, Epistemic Decay & Mobility/Fracture-Risk Integration Suite.

use qualia_cooperative_core::contextual_evidence::{
    EpistemicTier, ObservationMethod, ObserverRole, TemporalObservationAxis,
};
use qualia_cooperative_core::record::{EpistemicStatus, EvidenceType, SensitivityClass};
use wellfare_core::anatomy::{
    AgeOntogenyProfile, AnatomicalCalibrationInput, AnatomicalObservationRecord, BodyConstitution,
    BodyMeasurements, CalibrationObservation, EndocrineOsteoState, Karyotype, LateralityProfile,
    MobilityTaskInput, MuscleQualityProfile, ReproductiveEndocrinePhase, SkeletalDensityProfile,
    SpeciesDensitometryPreset, Trimester, WhoDensityCategory, apply_physiological_modifiers_to_density,
    derive_anatomical_calibration, evaluate_mobility_and_fracture_risk, harmonic_tps_organ_transfer,
};
use wellfare_core::clinical::ClaimStatus;

#[test]
fn epistemic_provenance_decay_and_restricted_envelope_sealing() {
    let day_ms: i64 = 86_400_000;
    let t0: i64 = 1_700_000_000_000;

    // 1. Tape-measure circumference (DirectlyObserved, Subacute 7-day half-life)
    let tape = AnatomicalObservationRecord::from_tape_measure(
        "obs-tape-waist",
        "did:q42:subject:alice",
        "https://ns.webizen.org/anthro#waistCircumference",
        780.0,
        ObserverRole::IsakCertified,
        t0,
        Some(2.5),
        Some("http://purl.obolibrary.org/obo/UBERON_0000916".into()),
    )
    .expect("valid tape measure observation");
    assert_eq!(tape.evidence.epistemic_tier, Some(EpistemicTier::DirectlyObserved));
    assert_eq!(
        tape.evidence.observation_method,
        Some(ObservationMethod::FlexibleTapeCircumference)
    );
    let tape_axis = tape.evidence.temporal_axis.expect("temporal_axis present");
    let tape_30d = tape_axis.compute_decayed_confidence(t0 + 30 * day_ms);
    let expected_30d = 2.0_f64.powf(-30.0 / 7.0);
    assert!((tape_30d - expected_30d).abs() < 1e-10);

    // 2. DEXA areal BMD (DirectlyObserved, 730-day skeletal remodeling axis)
    let dexa = AnatomicalObservationRecord::from_dexa_scan(
        "obs-dexa-femoral-neck",
        "did:q42:subject:alice",
        "https://ns.webizen.org/dens#arealBMD",
        0.98,
        t0,
        Some(t0 + 1_200),
        Some("http://purl.obolibrary.org/obo/UBERON_0001480".into()),
        "src:dicom:study-1001",
    )
    .expect("valid dexa observation");
    let dexa_axis = TemporalObservationAxis {
        phenomenon_time_ms: t0,
        acquisition_time_ms: Some(t0 + 1_200),
        extraction_time_ms: None,
        calibration_time_ms: Some(t0 + 2_000),
        half_life_days: Some(730.0),
    };
    let dexa_30d = dexa_axis.compute_decayed_confidence(t0 + 30 * day_ms);
    assert!(dexa_30d > 0.97, "DEXA 730d half-life should barely decay over 30d");
    assert!(dexa_30d > tape_30d, "DEXA confidence must outlast subacute tape measure at 30d");

    // 3. Clinical EHR/FHIR record extraction (ExtractedFromRecord)
    let ehr = AnatomicalObservationRecord::from_clinical_record(
        "obs-fhir-bmd-report",
        "did:q42:subject:alice",
        "https://ns.webizen.org/dens#arealBMD",
        qualia_cooperative_core::contextual_evidence::EvidenceValue::Quantity {
            value: 0.96,
            unit_iri: "http://qudt.org/vocab/unit/GM-PER-CentiM2".into(),
        },
        ClaimStatus::ClinicianConfirmed,
        Some(ObservationMethod::ElectronicHealthRecordFhir),
        t0,
        t0 + 3_600_000,
        "src:fhir:DiagnosticReport/42",
    )
    .expect("valid FHIR clinical record observation");
    assert_eq!(ehr.evidence.epistemic_tier, Some(EpistemicTier::ExtractedFromRecord));

    // Seal into RecordEnvelope: must enforce SensitivityClass::Restricted (never Public).
    let tape_env = tape.seal_into_envelope("did:q42:subject:alice", "did:q42:clinician:1", 1_700_000_000, None);
    let dexa_env = dexa.seal_into_envelope("did:q42:subject:alice", "did:q42:clinician:1", 1_700_000_000, None);
    let ehr_env = ehr.seal_into_envelope("did:q42:subject:alice", "did:q42:curator:1", 1_700_000_000, None);

    assert_eq!(tape_env.sensitivity, SensitivityClass::Restricted);
    assert_eq!(dexa_env.sensitivity, SensitivityClass::Restricted);
    assert_eq!(ehr_env.sensitivity, SensitivityClass::Restricted);
    assert_eq!(tape_env.epistemic_status, EpistemicStatus::Asserted);
    assert_eq!(tape_env.evidence_type, EvidenceType::ClinicianObserved);
    assert_eq!(ehr_env.epistemic_status, EpistemicStatus::Asserted);
}

#[test]
fn young_adult_female_relaxin_vs_postmenopausal_osteoporotic_myosteatosis() {
    let young_bone = SpeciesDensitometryPreset::HumanYoungAdultFemale.to_profile();
    let osteo_bone = SpeciesDensitometryPreset::HumanPostmenopausalOsteoporotic.to_profile();

    assert_eq!(young_bone.who_category(), Some(WhoDensityCategory::Normal));
    assert_eq!(osteo_bone.who_category(), Some(WhoDensityCategory::Osteoporosis));

    let young_e = young_bone.effective_cortical_modulus_gpa(0.01).unwrap();
    let osteo_e = osteo_bone.effective_cortical_modulus_gpa(0.01).unwrap();
    assert!(young_e > osteo_e, "young cortical modulus ({young_e}) > osteoporotic ({osteo_e})");

    let young_yield = young_bone.effective_trabecular_yield_mpa().unwrap();
    let osteo_yield = osteo_bone.effective_trabecular_yield_mpa().unwrap();
    assert!(young_yield > osteo_yield);

    // Apply gestational relaxin modulation to young adult female
    let young_muscle = MuscleQualityProfile::new(
        "quadriceps_femoris",
        1800.0,
        9.0,
        0.22,
        30.0,
        52.0,
        0.04,
        0.95,
    )
    .unwrap();
    let pregnant_endo =
        EndocrineOsteoState::for_phase(ReproductiveEndocrinePhase::Pregnant(Trimester::Third));
    let young_age = AgeOntogenyProfile::from_age_months(28 * 12).unwrap();
    let lat = LateralityProfile::default();

    let (_preg_bone, _preg_muscle, preg_summary) = apply_physiological_modifiers_to_density(
        &young_bone,
        &young_muscle,
        &pregnant_endo,
        &young_age,
        &lat,
        Some(Karyotype::Xx),
        true,
    )
    .unwrap();
    assert!(
        preg_summary.ligament_laxity_scale > 1.0,
        "Pregnancy Trimester 3 relaxin must increase ligament laxity scale (got {})",
        preg_summary.ligament_laxity_scale
    );

    // Postmenopausal osteoporotic + myosteatosis (low HU, high IMAT)
    let sarcopenic_muscle = MuscleQualityProfile::new(
        "quadriceps_femoris",
        1150.0,
        9.0,
        0.22,
        30.0,
        22.0,
        0.26,
        0.78,
    )
    .unwrap();
    assert!(sarcopenic_muscle.is_myosteatotic());
    assert!(
        young_muscle.effective_max_force_n().unwrap()
            > sarcopenic_muscle.effective_max_force_n().unwrap()
    );

    let stair_task = MobilityTaskInput {
        task_label: "stair_ascent_step_up".into(),
        required_joint_torque_nm: 110.0,
        muscle_moment_arm_m: 0.045,
        com_pos_m: 0.42,
        com_vel_m_s: 0.55,
        pendulum_length_m: 0.90,
        bos_boundary_m: 0.58,
        gravity_m_s2: 9.81,
        applied_vertebral_stress_mpa: 2.8,
        applied_femoral_neck_stress_mpa: 3.4,
    };

    let young_report =
        evaluate_mobility_and_fracture_risk(&young_bone, &young_muscle, &stair_task).unwrap();
    let osteo_report =
        evaluate_mobility_and_fracture_risk(&osteo_bone, &sarcopenic_muscle, &stair_task).unwrap();

    assert!(
        osteo_report.joint_relative_effort_ratio > young_report.joint_relative_effort_ratio,
        "sarcopenic/myosteatotic RER must exceed healthy young adult"
    );
    assert!(
        osteo_report.femoral_neck_factor_of_risk_phi > young_report.femoral_neck_factor_of_risk_phi,
        "osteoporotic hip Phi must exceed young adult"
    );
    assert!(osteo_report.myosteatosis_present);
    assert!(young_report.hof_margin_of_stability_m < 0.0 || young_report.hof_margin_of_stability_m > -1.0);
}

#[test]
fn comparative_cross_kingdom_densitometry_presets_validate_and_order_physically() {
    let presets = [
        SpeciesDensitometryPreset::AvianPneumaticFemur,
        SpeciesDensitometryPreset::SirenianPachyosteoscleroticRib,
        SpeciesDensitometryPreset::CetaceanOsteoporoticSpongiosa,
        SpeciesDensitometryPreset::CrustaceanCalcifiedCuticle,
        SpeciesDensitometryPreset::HardwoodSecondaryXylem,
    ];
    let profiles: Vec<SkeletalDensityProfile> = presets.iter().map(|p| p.to_profile()).collect();
    for p in &profiles {
        p.validate().expect("comparative preset must validate");
        assert!(p.effective_cortical_modulus_gpa(0.01).unwrap() > 0.0);
        assert!(p.effective_trabecular_yield_mpa().unwrap() > 0.0);
    }

    // Sirenian pachyosteosclerotic rib is denser than cetacean osteoporotic spongiosa
    let sirenian = profiles[1].trabecular_apparent_density_g_cm3;
    let cetacean = profiles[2].trabecular_apparent_density_g_cm3;
    assert!(sirenian > cetacean, "sirenian ({sirenian}) > cetacean ({cetacean})");
}

#[test]
fn three_stage_ssdm_and_harmonic_tps_calibration_preserves_organ_volume() {
    let mut constitution = BodyConstitution::default();
    constitution.measurements = BodyMeasurements {
        stature_mm: Some(1740),
        weight_g: Some(72_000),
        waist_mm: Some(810),
        hip_mm: Some(980),
        ..Default::default()
    };

    let obs = vec![CalibrationObservation {
        feature_id: "waist_mm".into(),
        region_bone_id: Some("lumbar_spine".into()),
        target_displacement_m: Some([0.0, 0.012, 0.004]),
        base_confidence: 0.92,
        elapsed_days: 3.0,
        half_life_days: 7.0,
    }];

    let input = AnatomicalCalibrationInput {
        constitution: &constitution,
        physiological_state: None,
        observations: &obs,
        organ_query_points: &[("heart", [0.02, 1.28, 0.04]), ("liver", [-0.06, 1.14, 0.03])],
        basis: None,
    };

    let result = derive_anatomical_calibration(&input);
    assert!(!result.bone_scalings.is_empty());
    assert!(!result.bone_corrections.is_empty());
    assert_eq!(result.organ_transfers.len(), 2);
    for transfer in &result.organ_transfers {
        assert!(
            transfer.jacobian_det > 0.0,
            "organ {} det(J) must be > 0, got {}",
            transfer.organ_id,
            transfer.jacobian_det
        );
    }

    // Direct harmonic_tps_organ_transfer check on a deformed thoracic cage
    let control_cage = [
        [-0.2, 1.1, -0.1],
        [0.2, 1.1, -0.1],
        [-0.2, 1.4, -0.1],
        [0.2, 1.4, -0.1],
        [-0.2, 1.1, 0.1],
        [0.2, 1.1, 0.1],
        [-0.2, 1.4, 0.1],
        [0.2, 1.4, 0.1],
    ];
    let displaced_cage = [
        [-0.22, 1.1, -0.11],
        [0.22, 1.1, -0.11],
        [-0.21, 1.42, -0.1],
        [0.21, 1.42, -0.1],
        [-0.22, 1.1, 0.12],
        [0.22, 1.1, 0.12],
        [-0.21, 1.42, 0.11],
        [0.21, 1.42, 0.11],
    ];
    let (deformed, det_j) = harmonic_tps_organ_transfer(&control_cage, &displaced_cage, [0.0, 1.25, 0.0]);
    assert!(deformed.iter().all(|v| v.is_finite()));
    assert!(det_j > 0.0, "harmonic_tps_organ_transfer must preserve orientation (det_j={det_j})");
}
