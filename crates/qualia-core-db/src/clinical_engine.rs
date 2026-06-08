//! Clinical Decision Support Engine.
//!
//! Validated clinical risk scoring, pharmacological screening, and
//! FHIR Observation validation — all as pure-Rust, zero-FFI computations.
//!
//! Risk models:
//!   `qualia:computeRiskScore:framingham`  → `framingham_10yr_risk()`
//!   `qualia:computeRiskScore:cha2ds2`     → `cha2ds2_vasc_score()`
//!   `qualia:computeRiskScore:score2`      → `score2_risk()`
//!
//! Pharmacology:
//!   `qualia:evaluateDrugInteraction`      → `check_drug_interactions()`
//!   `qualia:checkContraindication`        → `check_contraindications()`
//!
//! Observation validation:
//!   `qualia:validateFhirObservation`      → `validate_fhir_observation()`
//!   `qualia:evaluateLongitudinalTrend`    → `longitudinal_trend()`
//!   `qualia:evaluateGeneExpression`       → `evaluate_gene_expression()`

// ─── Shared enumerations ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskCategory {
    Low,
    Moderate,
    High,
    VeryHigh,
}

// ─── Framingham 10-year CVD risk ──────────────────────────────────────────────
// Anderson et al. 1991 / Wilson et al. 1998 (ATP III sex-specific log-linear model)

#[derive(Debug, Clone)]
pub struct FraminghamInput {
    pub age: u8,
    pub sex_male: bool,
    pub total_cholesterol_mmol: f64,
    pub hdl_cholesterol_mmol: f64,
    pub systolic_bp: f64,
    /// True if currently on antihypertensive medication.
    pub bp_treated: bool,
    pub current_smoker: bool,
    pub diabetic: bool,
}

#[derive(Debug, Clone)]
pub struct FraminghamResult {
    /// Estimated 10-year absolute risk (0.0–1.0).
    pub risk_10yr: f64,
    pub category: RiskCategory,
    pub log_score: f64,
}

pub fn framingham_10yr_risk(input: &FraminghamInput) -> FraminghamResult {
    // Wilson 1998 coefficients (Table 2).
    let (b_age, b_tc, b_hdl, b_sbp_unt, b_sbp_trt, b_smoke, b_diab, baseline_surv, mean_sum) =
        if input.sex_male {
            (
                3.06117,
                1.12370,
                -0.93263,
                1.93303,
                1.99881,
                0.65451,
                0.57367,
                0.88936_f64,
                23.9802_f64,
            )
        } else {
            (
                2.32888,
                1.20904,
                -0.70833,
                2.76157,
                2.82263,
                0.52873,
                0.69154,
                0.95012_f64,
                26.1931_f64,
            )
        };

    let sbp_coeff = if input.bp_treated {
        b_sbp_trt
    } else {
        b_sbp_unt
    };

    let log_score = b_age * (input.age as f64).ln()
        + b_tc * input.total_cholesterol_mmol.ln()
        + b_hdl * input.hdl_cholesterol_mmol.ln()
        + sbp_coeff * input.systolic_bp.ln()
        + b_smoke * if input.current_smoker { 1.0 } else { 0.0 }
        + b_diab * if input.diabetic { 1.0 } else { 0.0 };

    let risk_10yr = (1.0 - baseline_surv.powf((log_score - mean_sum).exp())).clamp(0.0, 1.0);

    let category = if risk_10yr < 0.10 {
        RiskCategory::Low
    } else if risk_10yr <= 0.20 {
        RiskCategory::Moderate
    } else {
        RiskCategory::High
    };

    FraminghamResult {
        risk_10yr,
        category,
        log_score,
    }
}

// ─── CHA₂DS₂-VASc ────────────────────────────────────────────────────────────
// Lip GY 2010 / ESC 2020 guidelines for non-valvular atrial fibrillation

#[derive(Debug, Clone, Default)]
pub struct Cha2ds2VascInput {
    pub congestive_heart_failure: bool, // +1
    pub hypertension: bool,             // +1
    pub age_75_or_older: bool,          // +2
    pub diabetes: bool,                 // +1
    pub stroke_tia_history: bool,       // +2
    pub vascular_disease: bool,         // +1
    pub age_65_to_74: bool,             // +1
    pub sex_female: bool,               // +1
}

#[derive(Debug, Clone)]
pub struct Cha2ds2VascResult {
    pub score: u8,
    /// Annual stroke risk % from Lip 2010 cohort data.
    pub annual_stroke_risk_pct: f64,
    /// ESC 2020: men ≥2, women ≥3.
    pub anticoagulation_recommended: bool,
}

pub fn cha2ds2_vasc_score(input: &Cha2ds2VascInput) -> Cha2ds2VascResult {
    let score = input.congestive_heart_failure as u8
        + input.hypertension as u8
        + if input.age_75_or_older { 2 } else { 0 }
        + input.diabetes as u8
        + if input.stroke_tia_history { 2 } else { 0 }
        + input.vascular_disease as u8
        + input.age_65_to_74 as u8
        + input.sex_female as u8;

    let annual_risk = match score {
        0 => 0.0,
        1 => 1.3,
        2 => 2.2,
        3 => 3.2,
        4 => 4.0,
        5 => 6.7,
        6 => 9.8,
        7 => 9.6,
        8 => 12.5,
        _ => 15.2,
    };

    let anticoagulation_recommended = if input.sex_female {
        score >= 3
    } else {
        score >= 2
    };

    Cha2ds2VascResult {
        score,
        annual_stroke_risk_pct: annual_risk,
        anticoagulation_recommended,
    }
}

// ─── SCORE2 ───────────────────────────────────────────────────────────────────
// ESC CVD Risk Collaboration / SCORE2 Working Group 2021

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score2Region {
    Low,
    Moderate,
    High,
    VeryHigh,
}

#[derive(Debug, Clone)]
pub struct Score2Input {
    pub age: u8,
    pub sex_male: bool,
    pub systolic_bp: f64,
    pub total_cholesterol_mmol: f64,
    pub hdl_cholesterol_mmol: f64,
    pub current_smoker: bool,
    pub risk_region: Score2Region,
}

#[derive(Debug, Clone)]
pub struct Score2Result {
    pub risk_10yr_pct: f64,
    pub category: RiskCategory,
}

pub fn score2_risk(input: &Score2Input) -> Score2Result {
    let non_hdl = input.total_cholesterol_mmol - input.hdl_cholesterol_mmol;
    let age_c = (input.age as f64 - 60.0) / 5.0;
    let sbp_c = (input.systolic_bp - 120.0) / 20.0;
    let chol_c = (non_hdl - 3.3) / 0.5;
    let smoke = input.current_smoker as u8 as f64;

    let (b_age, b_sbp, b_chol, b_smoke, baseline_surv) = if input.sex_male {
        (0.3742_f64, 0.2628, 0.1401, 0.5865, 0.9605_f64)
    } else {
        (0.4648_f64, 0.3131, 0.1002, 0.7742, 0.9776_f64)
    };

    let linear = b_age * age_c + b_sbp * sbp_c + b_chol * chol_c + b_smoke * smoke;
    let base_risk = 1.0 - baseline_surv.powf(linear.exp());

    let calibrated_pct = (base_risk
        * match input.risk_region {
            Score2Region::Low => 0.71,
            Score2Region::Moderate => 1.00,
            Score2Region::High => 1.56,
            Score2Region::VeryHigh => 2.27,
        }
        * 100.0)
        .clamp(0.0, 100.0);

    let category = if calibrated_pct < 5.0 {
        RiskCategory::Low
    } else if calibrated_pct <= 10.0 {
        RiskCategory::Moderate
    } else if calibrated_pct <= 20.0 {
        RiskCategory::High
    } else {
        RiskCategory::VeryHigh
    };

    Score2Result {
        risk_10yr_pct: calibrated_pct,
        category,
    }
}

// ─── Drug interaction screening ───────────────────────────────────────────────

/// NCI CTCAE-style severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InteractionSeverity {
    None = 0,
    Minor = 1,
    Moderate = 2,
    Major = 3,
    Contraindicated = 4,
}

#[derive(Debug, Clone)]
pub struct DrugInteraction {
    pub drug_a: u64,
    pub drug_b: u64,
    pub severity: InteractionSeverity,
    pub mechanism: &'static str,
}

/// CYP450-based drug-drug interaction screening.
/// Drug identifiers are `q_hash(rxnorm_name)`.
pub fn check_drug_interactions(active_medications: &[u64]) -> Vec<DrugInteraction> {
    // (drug_a, drug_b, severity, mechanism)
    let pairs: &[(&str, &str, InteractionSeverity, &str)] = &[
        (
            "warfarin",
            "ibuprofen",
            InteractionSeverity::Major,
            "CYP2C9 inhibition + antiplatelet effect → major bleeding",
        ),
        (
            "warfarin",
            "naproxen",
            InteractionSeverity::Major,
            "CYP2C9 inhibition + antiplatelet effect → major bleeding",
        ),
        (
            "warfarin",
            "aspirin",
            InteractionSeverity::Moderate,
            "Additive antiplatelet: monitor INR closely",
        ),
        (
            "sertraline",
            "phenelzine",
            InteractionSeverity::Contraindicated,
            "Serotonin syndrome risk (SSRI + MAOI)",
        ),
        (
            "fluoxetine",
            "selegiline",
            InteractionSeverity::Contraindicated,
            "Serotonin syndrome risk (SSRI + MAO-B)",
        ),
        (
            "simvastatin",
            "clarithromycin",
            InteractionSeverity::Major,
            "CYP3A4 inhibition → rhabdomyolysis risk",
        ),
        (
            "atorvastatin",
            "clarithromycin",
            InteractionSeverity::Moderate,
            "CYP3A4 inhibition → myopathy risk",
        ),
        (
            "amiodarone",
            "ciprofloxacin",
            InteractionSeverity::Major,
            "Additive QT prolongation → TdP risk",
        ),
        (
            "methadone",
            "azithromycin",
            InteractionSeverity::Major,
            "Additive QT prolongation → TdP risk",
        ),
        (
            "lisinopril",
            "spironolactone",
            InteractionSeverity::Moderate,
            "Hyperkalaemia risk (ACEi + K-sparing diuretic)",
        ),
        (
            "ramipril",
            "spironolactone",
            InteractionSeverity::Moderate,
            "Hyperkalaemia risk (ACEi + K-sparing diuretic)",
        ),
        (
            "metformin",
            "iohexol",
            InteractionSeverity::Major,
            "Lactic acidosis risk — hold metformin 48h before iodinated contrast",
        ),
        (
            "lithium",
            "ibuprofen",
            InteractionSeverity::Major,
            "NSAIDs reduce renal lithium clearance → toxicity",
        ),
        (
            "lithium",
            "diclofenac",
            InteractionSeverity::Major,
            "NSAIDs reduce renal lithium clearance → toxicity",
        ),
        (
            "methotrexate",
            "trimethoprim",
            InteractionSeverity::Major,
            "Additive folate antagonism → pancytopenia",
        ),
        (
            "digoxin",
            "amiodarone",
            InteractionSeverity::Major,
            "Amiodarone increases digoxin levels → toxicity",
        ),
        (
            "clopidogrel",
            "omeprazole",
            InteractionSeverity::Moderate,
            "CYP2C19 inhibition reduces clopidogrel activation",
        ),
        (
            "tramadol",
            "sertraline",
            InteractionSeverity::Moderate,
            "Serotonin syndrome + seizure threshold lowering",
        ),
    ];

    let mut found = Vec::new();
    for (i, &a) in active_medications.iter().enumerate() {
        for &b in &active_medications[i + 1..] {
            for &(na, nb, sev, mech) in pairs {
                let ha = crate::q_hash(na);
                let hb = crate::q_hash(nb);
                if (a == ha && b == hb) || (a == hb && b == ha) {
                    found.push(DrugInteraction {
                        drug_a: a,
                        drug_b: b,
                        severity: sev,
                        mechanism: mech,
                    });
                }
            }
        }
    }
    found
}

// ─── Contraindication checking ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContraindicationResult {
    pub drug: u64,
    pub condition_snomed: u64,
    pub severity: InteractionSeverity,
    pub reason: &'static str,
}

/// Checks a single medication (by q_hash name) against active conditions (SNOMED CT q_hashes).
pub fn check_contraindications(
    drug_hash: u64,
    condition_hashes: &[u64],
) -> Vec<ContraindicationResult> {
    // (drug_name, snomed_name_as_hashed, severity, reason)
    let table: &[(&str, &str, InteractionSeverity, &str)] = &[
        (
            "metformin",
            "709044004",
            InteractionSeverity::Contraindicated,
            "CKD stage 4/5 (eGFR < 30): lactic acidosis risk",
        ),
        (
            "nsaid",
            "709044004",
            InteractionSeverity::Major,
            "CKD: NSAIDs worsen renal haemodynamics",
        ),
        (
            "ibuprofen",
            "709044004",
            InteractionSeverity::Major,
            "CKD: NSAIDs worsen renal haemodynamics",
        ),
        (
            "naproxen",
            "709044004",
            InteractionSeverity::Major,
            "CKD: NSAIDs worsen renal haemodynamics",
        ),
        (
            "atenolol",
            "195967001",
            InteractionSeverity::Contraindicated,
            "Asthma: non-selective beta-blockers precipitate bronchospasm",
        ),
        (
            "propranolol",
            "195967001",
            InteractionSeverity::Contraindicated,
            "Asthma: non-selective beta-blockers precipitate bronchospasm",
        ),
        (
            "metoprolol",
            "195967001",
            InteractionSeverity::Moderate,
            "Asthma: cardioselective beta-blocker — use with caution",
        ),
        (
            "lithium",
            "709044004",
            InteractionSeverity::Contraindicated,
            "CKD: lithium is renally cleared — nephrotoxicity risk",
        ),
        (
            "clozapine",
            "84989004",
            InteractionSeverity::Major,
            "Seizure disorder: clozapine lowers seizure threshold",
        ),
        (
            "tramadol",
            "84989004",
            InteractionSeverity::Major,
            "Seizure disorder: tramadol lowers seizure threshold",
        ),
        (
            "warfarin",
            "713078009",
            InteractionSeverity::Major,
            "Haemorrhagic stroke history: anticoagulation risk",
        ),
        (
            "amiodarone",
            "49436004",
            InteractionSeverity::Major,
            "Pulmonary disease: amiodarone pulmonary toxicity risk",
        ),
        (
            "thalidomide",
            "77386006",
            InteractionSeverity::Contraindicated,
            "Pregnancy: teratogen (Category X)",
        ),
        (
            "isotretinoin",
            "77386006",
            InteractionSeverity::Contraindicated,
            "Pregnancy: teratogen (Category X)",
        ),
        (
            "methotrexate",
            "77386006",
            InteractionSeverity::Contraindicated,
            "Pregnancy: teratogen — folate antagonist",
        ),
        (
            "sildenafil",
            "194828000",
            InteractionSeverity::Contraindicated,
            "Angina on nitrates: severe hypotension risk",
        ),
    ];

    condition_hashes
        .iter()
        .filter_map(|&cond| {
            table
                .iter()
                .find(|&&(dn, sn, _, _)| {
                    crate::q_hash(dn) == drug_hash && crate::q_hash(sn) == cond
                })
                .map(|&(_, sn, sev, reason)| ContraindicationResult {
                    drug: drug_hash,
                    condition_snomed: crate::q_hash(sn),
                    severity: sev,
                    reason,
                })
        })
        .collect()
}

// ─── FHIR Observation validation ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FhirObservation {
    /// LOINC code string, e.g. "4548-4".
    pub loinc_code: String,
    pub value: f64,
    /// UCUM unit string, e.g. "%", "mmol/L".
    pub unit_ucum: String,
    pub reference_low: Option<f64>,
    pub reference_high: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    Normal,
    Low,
    High,
    CriticalLow,
    CriticalHigh,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FhirValidationResult {
    pub is_valid: bool,
    pub status: ObservationStatus,
    /// HL7 interpretation code: N / L / H / LL / HH / U.
    pub interpretation_code: &'static str,
}

pub fn validate_fhir_observation(obs: &FhirObservation) -> FhirValidationResult {
    // (loinc_code, low, high, critical_low, critical_high)
    const RANGES: &[(&str, f64, f64, f64, f64)] = &[
        ("4548-4", 4.0, 5.7, 2.0, 15.0),       // HbA1c %
        ("1558-6", 3.9, 5.5, 2.2, 25.0),       // Fasting glucose mmol/L
        ("2093-3", 0.0, 5.17, 0.0, 15.0),      // Total cholesterol mmol/L
        ("2089-1", 0.0, 3.36, 0.0, 12.0),      // LDL mmol/L
        ("2085-9", 1.0, 3.0, 0.4, 5.0),        // HDL mmol/L
        ("2571-8", 0.0, 1.7, 0.0, 10.0),       // Triglycerides mmol/L
        ("62238-1", 60.0, 200.0, 5.0, 200.0),  // eGFR mL/min/1.73m²
        ("9318-7", 0.0, 3.0, 0.0, 30.0),       // uACR mg/mmol
        ("38483-4", 45.0, 90.0, 10.0, 1000.0), // Creatinine µmol/L
        ("1742-6", 0.0, 41.0, 0.0, 1000.0),    // ALT U/L
        ("1920-8", 0.0, 40.0, 0.0, 1000.0),    // AST U/L
        ("718-7", 120.0, 170.0, 60.0, 200.0),  // Haemoglobin g/L
        ("6690-2", 4.0, 11.0, 1.5, 30.0),      // WBC ×10⁹/L
        ("30522-7", 0.0, 5.0, 0.0, 50.0),      // CRP mg/L
        ("3016-3", 0.4, 4.0, 0.01, 100.0),     // TSH mIU/L
        ("3024-7", 9.0, 19.0, 0.0, 50.0),      // Free T4 pmol/L
        ("2143-6", 0.0, 500.0, 0.0, 2000.0),   // Cortisol AM nmol/L
        ("1989-3", 50.0, 250.0, 10.0, 400.0),  // Vitamin D nmol/L
        ("20448-7", 2.6, 24.9, 0.0, 200.0),    // Fasting insulin pmol/L
        ("8480-6", 90.0, 140.0, 70.0, 220.0),  // Systolic BP mmHg
        ("8462-4", 60.0, 90.0, 40.0, 140.0),   // Diastolic BP mmHg
        ("8867-4", 50.0, 100.0, 30.0, 200.0),  // Resting HR bpm
        ("80404-7", 20.0, 200.0, 0.0, 500.0),  // HRV RMSSD ms
        ("59408-5", 95.0, 100.0, 85.0, 100.0), // SpO2 %
        ("44261-6", 0.0, 4.0, 0.0, 27.0),      // PHQ-9 score
        ("69737-5", 0.0, 4.0, 0.0, 21.0),      // GAD-7 score
        ("93832-4", 70.0, 100.0, 40.0, 100.0), // Sleep efficiency %
    ];

    let range = RANGES
        .iter()
        .find(|(code, ..)| *code == obs.loinc_code.as_str());

    let status = if let Some(&(_, low, high, crit_low, crit_high)) = range {
        let v = obs.value;
        if v < crit_low {
            ObservationStatus::CriticalLow
        } else if v > crit_high {
            ObservationStatus::CriticalHigh
        } else if v < low {
            ObservationStatus::Low
        } else if v > high {
            ObservationStatus::High
        } else {
            ObservationStatus::Normal
        }
    } else if let (Some(low), Some(high)) = (obs.reference_low, obs.reference_high) {
        let v = obs.value;
        if v < low * 0.5 {
            ObservationStatus::CriticalLow
        } else if v > high * 2.0 {
            ObservationStatus::CriticalHigh
        } else if v < low {
            ObservationStatus::Low
        } else if v > high {
            ObservationStatus::High
        } else {
            ObservationStatus::Normal
        }
    } else {
        ObservationStatus::Unknown
    };

    let interp = match status {
        ObservationStatus::Normal => "N",
        ObservationStatus::Low => "L",
        ObservationStatus::High => "H",
        ObservationStatus::CriticalLow => "LL",
        ObservationStatus::CriticalHigh => "HH",
        ObservationStatus::Unknown => "U",
    };

    FhirValidationResult {
        is_valid: status != ObservationStatus::Unknown,
        status,
        interpretation_code: interp,
    }
}

// ─── Longitudinal trend analysis ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TimePoint {
    /// Unix timestamp in seconds.
    pub timestamp_s: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Improving,
    Worsening,
    Stable,
    Insufficient,
}

#[derive(Debug, Clone)]
pub struct TrendResult {
    /// Ordinary least-squares slope in biomarker units per day.
    pub slope_per_day: f64,
    /// Coefficient of determination.
    pub r_squared: f64,
    /// Predicted value `forecast_days` after the last observation.
    pub forecast: f64,
    pub direction: TrendDirection,
}

/// OLS linear regression over a biomarker time series.
/// `improvement_direction`: `1` = rising is good (e.g. eGFR), `-1` = falling is good (e.g. BP, HbA1c).
pub fn longitudinal_trend(
    series: &[TimePoint],
    forecast_days: f64,
    improvement_direction: i8,
) -> TrendResult {
    if series.len() < 2 {
        let v = series.first().map(|p| p.value).unwrap_or(0.0);
        return TrendResult {
            slope_per_day: 0.0,
            r_squared: 0.0,
            forecast: v,
            direction: TrendDirection::Insufficient,
        };
    }

    let t0 = series[0].timestamp_s as f64;
    let xs: Vec<f64> = series
        .iter()
        .map(|p| (p.timestamp_s as f64 - t0) / 86400.0)
        .collect();
    let ys: Vec<f64> = series.iter().map(|p| p.value).collect();
    let n = xs.len() as f64;

    let sx: f64 = xs.iter().sum();
    let sy: f64 = ys.iter().sum();
    let sxx: f64 = xs.iter().map(|x| x * x).sum();
    let sxy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
    let denom = n * sxx - sx * sx;

    if denom.abs() < 1e-10 {
        return TrendResult {
            slope_per_day: 0.0,
            r_squared: 1.0,
            forecast: ys[0],
            direction: TrendDirection::Stable,
        };
    }

    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    let y_mean = sy / n;

    let ss_res: f64 = ys
        .iter()
        .zip(xs.iter())
        .map(|(y, x)| (y - (intercept + slope * x)).powi(2))
        .sum();
    let ss_tot: f64 = ys.iter().map(|y| (y - y_mean).powi(2)).sum();
    let r_squared = if ss_tot < 1e-10 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    let last_x = xs.last().copied().unwrap_or(0.0);
    let forecast = intercept + slope * (last_x + forecast_days);

    let direction = if slope.abs() < 0.001 {
        TrendDirection::Stable
    } else if (slope > 0.0 && improvement_direction > 0)
        || (slope < 0.0 && improvement_direction < 0)
    {
        TrendDirection::Improving
    } else {
        TrendDirection::Worsening
    };

    TrendResult {
        slope_per_day: slope,
        r_squared,
        forecast,
        direction,
    }
}

// ─── Gene expression evaluation ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionDirection {
    Upregulated,
    Downregulated,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct GeneExpressionResult {
    pub gene_id: u64,
    pub fold_change: f64,
    pub log2_fold_change: f64,
    pub is_significant: bool,
    pub direction: ExpressionDirection,
}

/// Evaluates normalised expression (RPKM/TPM) against a fold-change threshold.
pub fn evaluate_gene_expression(
    gene_id: u64,
    baseline: f64,
    treatment: f64,
    fc_threshold: f64,
) -> GeneExpressionResult {
    let fold_change = if baseline > 1e-9 {
        treatment / baseline
    } else {
        f64::INFINITY
    };
    let log2_fc = if fold_change.is_finite() {
        fold_change.log2()
    } else {
        f64::INFINITY
    };
    let is_significant = fold_change >= fc_threshold
        || (fold_change.is_finite() && fold_change <= 1.0 / fc_threshold);

    let direction = if !is_significant {
        ExpressionDirection::Unchanged
    } else if fold_change >= 1.0 {
        ExpressionDirection::Upregulated
    } else {
        ExpressionDirection::Downregulated
    };

    GeneExpressionResult {
        gene_id,
        fold_change,
        log2_fold_change: log2_fc,
        is_significant,
        direction,
    }
}

// ─── Renal Function Estimation ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RenalInput {
    pub age: u8,
    pub sex_male: bool,
    pub weight_kg: f64,
    /// Serum creatinine in mg/dL
    pub serum_creatinine: f64,
}

/// Computes Creatinine Clearance (CrCl) via Cockcroft-Gault equation.
pub fn cockcroft_gault_crcl(input: &RenalInput) -> f64 {
    let mut crcl = ((140.0 - input.age as f64) * input.weight_kg) / (72.0 * input.serum_creatinine);
    if !input.sex_male {
        crcl *= 0.85;
    }
    crcl
}

/// Computes eGFR using the 2021 CKD-EPI equation (creatinine, without race).
pub fn ckd_epi_egfr(input: &RenalInput) -> f64 {
    let k = if input.sex_male { 0.9 } else { 0.7 };
    let a = if input.sex_male { -0.302 } else { -0.241 };

    let scr_k = input.serum_creatinine / k;
    let min_val = scr_k.min(1.0);
    let max_val = scr_k.max(1.0);

    let mut egfr =
        142.0 * min_val.powf(a) * max_val.powf(-1.200) * 0.9938_f64.powf(input.age as f64);
    if !input.sex_male {
        egfr *= 1.012;
    }
    egfr
}

// ─── Pharmacokinetics (PK) ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PkOneCompartmentInput {
    pub dose_mg: f64,
    /// Volume of distribution (L)
    pub volume_distribution_l: f64,
    /// Clearance (L/hr)
    pub clearance_l_hr: f64,
    /// Time since dose (hours)
    pub time_hr: f64,
}

#[derive(Debug, Clone)]
pub struct PkResult {
    /// Concentration at time t (mg/L)
    pub concentration: f64,
    /// Half-life (hours)
    pub half_life_hr: f64,
}

/// Predicts drug concentration using a 1-compartment IV bolus model.
pub fn one_compartment_pk_model(input: &PkOneCompartmentInput) -> PkResult {
    let k_el = input.clearance_l_hr / input.volume_distribution_l;
    let c0 = input.dose_mg / input.volume_distribution_l;
    let concentration = c0 * (-k_el * input.time_hr).exp();
    let half_life_hr = 0.693147 / k_el;

    PkResult {
        concentration,
        half_life_hr,
    }
}

// ─── SOFA Score (Sequential Organ Failure Assessment) ────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SofaInput {
    pub pao2_fio2_ratio: f64, // mmHg
    pub platelets_10_9_l: f64,
    pub bilirubin_mg_dl: f64,
    pub map_mmhg: f64,            // Mean arterial pressure
    pub dopamine_dose: f64,       // ug/kg/min
    pub epinephrine_dose: f64,    // ug/kg/min
    pub norepinephrine_dose: f64, // ug/kg/min
    pub glasgow_coma_scale: u8,
    pub creatinine_mg_dl: f64,
    pub urine_output_ml_d: f64,
}

/// Evaluates acute sepsis morbidity via the SOFA score (0-24).
pub fn sofa_score(input: &SofaInput) -> u8 {
    let mut score = 0;

    // Respiration
    if input.pao2_fio2_ratio > 0.0 {
        if input.pao2_fio2_ratio < 100.0 {
            score += 4;
        } else if input.pao2_fio2_ratio < 200.0 {
            score += 3;
        } else if input.pao2_fio2_ratio < 300.0 {
            score += 2;
        } else if input.pao2_fio2_ratio < 400.0 {
            score += 1;
        }
    }

    // Coagulation (Platelets)
    if input.platelets_10_9_l > 0.0 {
        if input.platelets_10_9_l < 20.0 {
            score += 4;
        } else if input.platelets_10_9_l < 50.0 {
            score += 3;
        } else if input.platelets_10_9_l < 100.0 {
            score += 2;
        } else if input.platelets_10_9_l < 150.0 {
            score += 1;
        }
    }

    // Liver (Bilirubin)
    if input.bilirubin_mg_dl >= 12.0 {
        score += 4;
    } else if input.bilirubin_mg_dl >= 6.0 {
        score += 3;
    } else if input.bilirubin_mg_dl >= 2.0 {
        score += 2;
    } else if input.bilirubin_mg_dl >= 1.2 {
        score += 1;
    }

    // Cardiovascular
    if input.dopamine_dose > 15.0 || input.epinephrine_dose > 0.1 || input.norepinephrine_dose > 0.1
    {
        score += 4;
    } else if input.dopamine_dose > 5.0
        || (input.epinephrine_dose > 0.0 && input.epinephrine_dose <= 0.1)
        || (input.norepinephrine_dose > 0.0 && input.norepinephrine_dose <= 0.1)
    {
        score += 3;
    } else if input.dopamine_dose > 0.0 {
        score += 2;
    } else if input.map_mmhg > 0.0 && input.map_mmhg < 70.0 {
        score += 1;
    }

    // Central Nervous System (GCS)
    if input.glasgow_coma_scale > 0 {
        if input.glasgow_coma_scale < 6 {
            score += 4;
        } else if input.glasgow_coma_scale <= 9 {
            score += 3;
        } else if input.glasgow_coma_scale <= 12 {
            score += 2;
        } else if input.glasgow_coma_scale <= 14 {
            score += 1;
        }
    }

    // Renal
    if input.creatinine_mg_dl >= 5.0
        || (input.urine_output_ml_d < 200.0 && input.urine_output_ml_d > 0.0)
    {
        score += 4;
    } else if input.creatinine_mg_dl >= 3.5
        || (input.urine_output_ml_d < 500.0 && input.urine_output_ml_d > 0.0)
    {
        score += 3;
    } else if input.creatinine_mg_dl >= 2.0 {
        score += 2;
    } else if input.creatinine_mg_dl >= 1.2 {
        score += 1;
    }

    score
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framingham_high_risk_male() {
        let r = framingham_10yr_risk(&FraminghamInput {
            age: 60,
            sex_male: true,
            total_cholesterol_mmol: 6.5,
            hdl_cholesterol_mmol: 0.9,
            systolic_bp: 162.0,
            bp_treated: false,
            current_smoker: true,
            diabetic: true,
        });
        assert!(r.risk_10yr > 0.20, "Expected >20% got {:.2}", r.risk_10yr);
        assert_eq!(r.category, RiskCategory::High);
    }

    #[test]
    fn framingham_low_risk_female() {
        let r = framingham_10yr_risk(&FraminghamInput {
            age: 40,
            sex_male: false,
            total_cholesterol_mmol: 4.5,
            hdl_cholesterol_mmol: 1.8,
            systolic_bp: 115.0,
            bp_treated: false,
            current_smoker: false,
            diabetic: false,
        });
        assert!(r.risk_10yr < 0.10, "Expected <10% got {:.2}", r.risk_10yr);
        assert_eq!(r.category, RiskCategory::Low);
    }

    #[test]
    fn cha2ds2_max_score() {
        let r = cha2ds2_vasc_score(&Cha2ds2VascInput {
            congestive_heart_failure: true,
            hypertension: true,
            age_75_or_older: true,
            diabetes: true,
            stroke_tia_history: true,
            vascular_disease: true,
            age_65_to_74: false,
            sex_female: true,
        });
        assert_eq!(r.score, 9);
        assert!(r.anticoagulation_recommended);
    }

    #[test]
    fn cha2ds2_zero_male() {
        let r = cha2ds2_vasc_score(&Cha2ds2VascInput::default());
        assert_eq!(r.score, 0);
        assert!(!r.anticoagulation_recommended);
    }

    #[test]
    fn drug_interaction_warfarin_ibuprofen() {
        let meds = vec![crate::q_hash("warfarin"), crate::q_hash("ibuprofen")];
        let found = check_drug_interactions(&meds);
        assert!(!found.is_empty());
        assert_eq!(found[0].severity, InteractionSeverity::Major);
    }

    #[test]
    fn drug_interaction_no_false_positive() {
        let meds = vec![crate::q_hash("paracetamol"), crate::q_hash("lactulose")];
        let found = check_drug_interactions(&meds);
        assert!(found.is_empty());
    }

    #[test]
    fn fhir_hba1c_normal() {
        let r = validate_fhir_observation(&FhirObservation {
            loinc_code: "4548-4".into(),
            value: 5.2,
            unit_ucum: "%".into(),
            reference_low: None,
            reference_high: None,
        });
        assert_eq!(r.status, ObservationStatus::Normal);
        assert_eq!(r.interpretation_code, "N");
    }

    #[test]
    fn fhir_hba1c_high() {
        let r = validate_fhir_observation(&FhirObservation {
            loinc_code: "4548-4".into(),
            value: 8.5,
            unit_ucum: "%".into(),
            reference_low: None,
            reference_high: None,
        });
        assert_eq!(r.status, ObservationStatus::High);
        assert_eq!(r.interpretation_code, "H");
    }

    #[test]
    fn trend_worsening_bp() {
        let series = vec![
            TimePoint {
                timestamp_s: 0,
                value: 120.0,
            },
            TimePoint {
                timestamp_s: 86400,
                value: 125.0,
            },
            TimePoint {
                timestamp_s: 172800,
                value: 130.0,
            },
            TimePoint {
                timestamp_s: 259200,
                value: 135.0,
            },
        ];
        let r = longitudinal_trend(&series, 7.0, -1);
        assert!(r.slope_per_day > 4.0);
        assert_eq!(r.direction, TrendDirection::Worsening);
        assert!(r.r_squared > 0.99);
    }

    #[test]
    fn gene_expression_upregulated() {
        let r = evaluate_gene_expression(0xDEAD, 100.0, 350.0, 2.0);
        assert!(r.is_significant);
        assert_eq!(r.direction, ExpressionDirection::Upregulated);
        assert!((r.log2_fold_change - 1.807).abs() < 0.01);
    }

    #[test]
    fn test_cockcroft_gault() {
        let input = RenalInput {
            age: 60,
            sex_male: true,
            weight_kg: 80.0,
            serum_creatinine: 1.2,
        };
        let crcl = cockcroft_gault_crcl(&input);
        assert!((crcl - 74.07).abs() < 0.1);
    }

    #[test]
    fn test_ckd_epi() {
        let input = RenalInput {
            age: 60,
            sex_male: false,
            weight_kg: 70.0,
            serum_creatinine: 1.2,
        };
        let egfr = ckd_epi_egfr(&input);
        assert!((egfr - 51.5).abs() < 1.0);
    }

    #[test]
    fn test_one_compartment_pk() {
        let input = PkOneCompartmentInput {
            dose_mg: 1000.0,
            volume_distribution_l: 50.0,
            clearance_l_hr: 5.0,
            time_hr: 10.0,
        };
        let pk = one_compartment_pk_model(&input);
        assert!((pk.concentration - 7.35).abs() < 0.1);
        assert!((pk.half_life_hr - 6.93).abs() < 0.1);
    }

    #[test]
    fn test_sofa_score() {
        let input = SofaInput {
            pao2_fio2_ratio: 250.0, // 2 points
            platelets_10_9_l: 80.0, // 2 points
            bilirubin_mg_dl: 3.0,   // 2 points
            map_mmhg: 65.0,         // 1 point
            dopamine_dose: 0.0,
            epinephrine_dose: 0.0,
            norepinephrine_dose: 0.0,
            glasgow_coma_scale: 13, // 1 point
            creatinine_mg_dl: 2.5,  // 2 points
            urine_output_ml_d: 0.0,
        };
        let score = sofa_score(&input);
        assert_eq!(score, 10);
    }
}
