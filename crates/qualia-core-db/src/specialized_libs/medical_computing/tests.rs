
use super::*;

#[test]
fn test_medical_library_creation() {
    let mut library = MedicalComputingLibrary::new();
    assert!(library.initialize().is_ok());
}

#[test]
fn test_patient_record_creation() {
    let mut library = MedicalComputingLibrary::new();
    library.initialize().unwrap();

    let patient = Patient::new();
    let result = library.create_patient_record(patient).unwrap();

    assert_eq!(result.result.patient_id, "patient_1");
    assert_eq!(result.result.medical_record_number, "MRN001");
    // Honest: privacy is not measured by this scaffold, so no score is fabricated.
    assert!(result.privacy_score.is_none());
    assert!(result.compliance_status == ComplianceStatus::Compliant);
}

#[test]
fn test_clinical_analysis() {
    use std::collections::HashMap;
    let mut library = MedicalComputingLibrary::new();
    library.initialize().unwrap();

    // The patient-only path has no knowledge base → still fails closed (no fabricated
    // diagnosis/confidence) rather than inventing one.
    assert!(library
        .analyze_clinical_data("patient_1", ClinicalDataType::Diagnosis)
        .is_err());

    // REAL transparent naive-Bayes over an ILLUSTRATIVE, NON-AUTHORITATIVE caller KB.
    // unnorm(flu)=0.6*0.9*0.8=0.432 ; unnorm(cold)=0.4*0.2*0.6=0.048 ; sum=0.48
    // post(flu)=0.432/0.48=0.9 ; post(cold)=0.048/0.48=0.1
    let mut flu = HashMap::new();
    flu.insert("fever".to_string(), 0.9);
    flu.insert("cough".to_string(), 0.8);
    let mut cold = HashMap::new();
    cold.insert("fever".to_string(), 0.2);
    cold.insert("cough".to_string(), 0.6);
    let kb = DiagnosticKnowledgeBase {
        conditions: vec![
            ConditionModel {
                condition_id: "influenza_like".to_string(),
                prior: 0.6,
                likelihoods: flu,
            },
            ConditionModel {
                condition_id: "common_cold".to_string(),
                prior: 0.4,
                likelihoods: cold,
            },
        ],
        unlisted_finding_likelihood: 0.5,
    };
    let obs = vec!["fever".to_string(), "cough".to_string()];
    let out = library.analyze_differential(&obs, &kb).unwrap();
    let p = &out.result;
    // Honest label — explicitly NOT a diagnosis.
    assert!(p.epistemic_status.contains("NOT a diagnosis"));
    assert_eq!(p.ranked[0].condition_id, "influenza_like");
    approx(p.ranked[0].posterior, 0.9, 1e-9);
    approx(p.ranked[1].posterior, 0.1, 1e-9);
}

#[test]
fn test_medical_imaging() {
    let mut library = MedicalComputingLibrary::new();
    library.initialize().unwrap();

    // REAL DSP on a 4x4 step edge: left two cols = 0, right two cols = 100.
    let data: Vec<f64> = (0..16)
        .map(|i| if i % 4 >= 2 { 100.0 } else { 0.0 })
        .collect();
    let out = library
        .analyze_medical_image_grid(&data, 4, 4, 8, SegmentationThreshold::Fixed(50.0), None)
        .unwrap();
    let r = &out.result;
    // Honest label — signal-processing metrics only.
    assert!(r.epistemic_status.contains("NOT a diagnosis"));
    // Hand values: mean 50, population std 50 (half the pixels 0, half 100).
    approx(r.mean, 50.0, 1e-9);
    approx(r.std_dev, 50.0, 1e-9);
    // Fixed threshold 50 → foreground = the 8 bright pixels; region mean 100.
    assert_eq!(r.segmented_area, 8);
    approx(r.segmented_mean_intensity, 100.0, 1e-9);
    // Sobel magnitude peaks at the vertical edge (|G| = 400) and is 0 in flat columns.
    let peak = r.sobel_magnitude.iter().cloned().fold(f64::MIN, f64::max);
    approx(peak, 400.0, 1e-9);
    approx(r.sobel_magnitude[0], 0.0, 1e-9); // col 0 (flat)
    approx(r.sobel_magnitude[3], 0.0, 1e-9); // col 3 (flat)
}

#[test]
fn test_compound_screening() {
    let mut library = MedicalComputingLibrary::new();
    library.initialize().unwrap();

    // Ethanol: small, drug-like → passes Lipinski (0 violations).
    let mut ethanol = Compound::new();
    ethanol.compound_id = "eth".to_string();
    ethanol.chemical_structure = "CCO".to_string();
    // A 40-carbon alkane: MW > 500 → violates Lipinski (flagged).
    let mut alkane = Compound::new();
    alkane.compound_id = "alk".to_string();
    alkane.chemical_structure = "C".repeat(40);

    let target = DrugTarget::new();
    // Rank by Tanimoto similarity to an ethanol query structure.
    let out = library
        .screen_compounds(vec![ethanol, alkane], target, Some("CCO"))
        .unwrap();
    let p = &out.result;
    // Honest label — rule-based filter + similarity, NOT an affinity/efficacy prediction.
    assert!(p.epistemic_status.contains("NOT a"));

    // Ethanol is identical to the query → Tanimoto 1.0 → ranked first.
    assert_eq!(p.ranked[0].compound_id, "eth");
    approx(p.ranked[0].tanimoto_to_query, 1.0, 1e-12);

    let eth = p.ranked.iter().find(|r| r.compound_id == "eth").unwrap();
    let alk = p.ranked.iter().find(|r| r.compound_id == "alk").unwrap();
    // Descriptors came from the real parsed structure, not a fallback.
    assert!(eth.descriptors_from_structure);
    // Ethanol passes the rule-of-five; the big alkane is flagged (MW > 500).
    assert!(eth.passes_lipinski);
    assert_eq!(eth.lipinski_violations, 0);
    assert!(alk.molecular_weight > 500.0);
    assert!(alk.lipinski_violations >= 1);
}

#[test]
fn test_compliance_check() {
    let mut library = MedicalComputingLibrary::new();
    library.initialize().unwrap();

    let result = library.check_compliance(ComplianceType::HIPAA).unwrap();

    assert_eq!(result.result.report_type, ComplianceType::HIPAA);
    // Honest: no compliance assessment is performed, so no score is fabricated.
    assert!(result.result.compliance_score.is_none());
    assert!(result.compliance_status == ComplianceStatus::Compliant);
}

#[test]
fn test_performance_metrics() {
    let library = MedicalComputingLibrary::new();
    let metrics = library.get_performance_stats();

    assert_eq!(metrics.total_patients, 0);
    assert_eq!(metrics.average_processing_time, 0.0);
    // Honest: this scaffold measures none of these, so they are not fabricated.
    assert!(metrics.privacy_score.is_none());
    assert!(metrics.compliance_score.is_none());
    assert!(metrics.data_quality.is_none());
}

#[test]
fn test_patient_listing() {
    let library = MedicalComputingLibrary::new();
    let patients = library.list_patients();
    assert_eq!(patients.len(), 0);
}

#[test]
fn test_patient_info() {
    let library = MedicalComputingLibrary::new();
    let info = library.get_patient_info("patient_1");
    assert!(info.is_none());
}

// -----------------------------------------------------------------
// Deterministic clinical calculators — known-value (textbook) tests.
// -----------------------------------------------------------------

fn lib() -> MedicalComputingLibrary {
    MedicalComputingLibrary::new()
}

fn approx(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "expected {b}, got {a} (tol {tol})");
}

#[test]
fn test_bmi_known_value() {
    // 70 kg / (1.75 m)^2 = 22.857...
    approx(lib().bmi(70.0, 1.75).unwrap(), 22.857, 1e-3);
    assert!(lib().bmi(70.0, 0.0).is_err());
}

#[test]
fn test_bsa_mosteller_known_value() {
    // sqrt(180*75/3600) = sqrt(3.75) = 1.93649
    approx(lib().bsa_mosteller(75.0, 180.0).unwrap(), 1.93649, 1e-4);
}

#[test]
fn test_bsa_du_bois_known_value() {
    // 0.007184 * 70^0.425 * 170^0.725 ≈ 1.8090 m²
    approx(lib().bsa_du_bois(70.0, 170.0).unwrap(), 1.8090, 1e-3);
}

#[test]
fn test_ideal_body_weight_devine() {
    // Male, 175 cm: 68.898 in; 50 + 2.3*(8.898) = 70.465 kg
    approx(
        lib().ideal_body_weight_devine(175.0, Gender::Male).unwrap(),
        70.465,
        1e-2,
    );
    // Female base 45.5 → 65.965 kg
    approx(
        lib()
            .ideal_body_weight_devine(175.0, Gender::Female)
            .unwrap(),
        65.965,
        1e-2,
    );
    // Sex Other/Unknown must fail closed (no validated coefficient).
    assert!(lib()
        .ideal_body_weight_devine(175.0, Gender::Unknown)
        .is_err());
}

#[test]
fn test_egfr_ckd_epi_2021_known_value() {
    // Male, Scr 1.0 mg/dL, age 50 → ≈ 91.7 mL/min/1.73m²
    approx(
        lib().egfr_ckd_epi_2021(1.0, 50.0, Gender::Male).unwrap(),
        91.70,
        0.2,
    );
    assert!(lib().egfr_ckd_epi_2021(1.0, 50.0, Gender::Other).is_err());
}

#[test]
fn test_egfr_mdrd_known_value() {
    // Scr 1.0, age 50, male, non-black: 175 * 50^-0.203 = 79.10
    approx(
        lib().egfr_mdrd(1.0, 50.0, Gender::Male, false).unwrap(),
        79.10,
        0.1,
    );
    // Black race factor 1.212 applied.
    approx(
        lib().egfr_mdrd(1.0, 50.0, Gender::Male, true).unwrap(),
        79.10 * 1.212,
        0.2,
    );
}

#[test]
fn test_cockcroft_gault_known_value() {
    // age 60, 72 kg, Scr 1.0, male: (140-60)*72/(72*1) = 80 mL/min
    approx(
        lib()
            .creatinine_clearance_cockcroft_gault(60.0, 72.0, 1.0, Gender::Male)
            .unwrap(),
        80.0,
        1e-9,
    );
    // Female 0.85 factor → 68 mL/min
    approx(
        lib()
            .creatinine_clearance_cockcroft_gault(60.0, 72.0, 1.0, Gender::Female)
            .unwrap(),
        68.0,
        1e-9,
    );
}

#[test]
fn test_mean_arterial_pressure_known_value() {
    // 120/80 → (120 + 160)/3 = 93.333
    approx(
        lib().mean_arterial_pressure(120.0, 80.0).unwrap(),
        93.333,
        1e-3,
    );
    assert!(lib().mean_arterial_pressure(80.0, 120.0).is_err());
}

#[test]
fn test_anion_gap_known_value() {
    // Na 140, Cl 100, HCO3 24 → 16
    approx(lib().anion_gap(140.0, 100.0, 24.0), 16.0, 1e-9);
}

#[test]
fn test_corrected_calcium_known_value() {
    // measured 8.0 mg/dL, albumin 2.0 → 8.0 + 0.8*2.0 = 9.6
    approx(lib().corrected_calcium(8.0, 2.0).unwrap(), 9.6, 1e-9);
}

#[test]
fn test_winters_expected_pco2_known_value() {
    // HCO3 12 → 1.5*12 + 8 = 26
    approx(lib().winters_expected_pco2(12.0).unwrap(), 26.0, 1e-9);
}

#[test]
fn test_cha2ds2_vasc_point_sum() {
    // 70 y/o female with hypertension + diabetes:
    // age 65-74 (1) + female (1) + HTN (1) + DM (1) = 4
    let score = lib().cha2ds2_vasc_score(false, true, 70, true, false, false, Gender::Female);
    assert_eq!(score, 4);
    // Max case: CHF+HTN+age>=75(2)+DM+stroke(2)+vascular+female = 9
    let max = lib().cha2ds2_vasc_score(true, true, 80, true, true, true, Gender::Female);
    assert_eq!(max, 9);
}

#[test]
fn test_weight_based_dose() {
    // 5 mg/kg * 70 kg = 350 mg
    approx(lib().weight_based_dose(5.0, 70.0).unwrap(), 350.0, 1e-9);
}

#[test]
fn test_giusti_hayton_adjusted_dose() {
    // Fe 0.5, CrCl 30/120, normal 500: Q = 1 - 0.5*(1-0.25) = 0.625 → 312.5
    approx(
        lib()
            .giusti_hayton_adjusted_dose(500.0, 0.5, 30.0, 120.0)
            .unwrap(),
        312.5,
        1e-9,
    );
    assert!(lib()
        .giusti_hayton_adjusted_dose(500.0, 1.5, 30.0, 120.0)
        .is_err());
}

#[test]
fn test_mg_mmol_roundtrip() {
    // Calcium MW 40.08: 100 mg -> 2.4950 mmol -> back to 100 mg
    let mmol = lib().mg_to_mmol(100.0, 40.08).unwrap();
    approx(mmol, 2.49501, 1e-4);
    approx(lib().mmol_to_mg(mmol, 40.08).unwrap(), 100.0, 1e-9);
}

#[test]
fn test_infusion_rate() {
    // 5 µg/kg/min, 70 kg, 1600 µg/mL: (5*70*60)/1600 = 13.125 mL/hr
    approx(
        lib().infusion_rate_ml_per_hr(5.0, 70.0, 1600.0).unwrap(),
        13.125,
        1e-6,
    );
}

#[test]
fn test_pharmacokinetics() {
    // k = ln2/4 = 0.17329 /h
    approx(
        lib().elimination_rate_constant(4.0).unwrap(),
        0.173287,
        1e-5,
    );
    // t½ from k roundtrips
    let k = lib().elimination_rate_constant(4.0).unwrap();
    approx(lib().half_life_from_rate_constant(k).unwrap(), 4.0, 1e-9);
    // CL = k * Vd
    approx(lib().clearance(0.1, 50.0).unwrap(), 5.0, 1e-9);
    // Vd = dose / C0
    approx(
        lib().volume_of_distribution(500.0, 10.0).unwrap(),
        50.0,
        1e-9,
    );
    // Css = R0 / CL
    approx(
        lib().steady_state_concentration(100.0, 10.0).unwrap(),
        10.0,
        1e-9,
    );
}

#[test]
fn test_summarize_cohort_delegates_to_statistics() {
    // [2,4,4,4,5,5,7,9]: mean 5, sample std 2.138..., median 4.5
    let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let s = lib().summarize_cohort(&v).unwrap();
    assert_eq!(s.n, 8);
    approx(s.mean, 5.0, 1e-9);
    approx(s.std_dev.unwrap(), 2.13809, 1e-4);
    approx(s.median, 4.5, 1e-9);
    approx(s.min, 2.0, 1e-9);
    approx(s.max, 9.0, 1e-9);
    assert!(lib().summarize_cohort(&[]).is_err());
    // n<2 → std_dev None, not NaN.
    assert!(lib().summarize_cohort(&[3.0]).unwrap().std_dev.is_none());
}
