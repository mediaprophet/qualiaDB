use super::*;

// ---------------------------------------------------------------------------
// Deterministic clinical calculators (the genuinely-computable subset).
//
// Every method in this block implements a PUBLISHED, deterministic formula that
// depends only on its numeric inputs — no trained model, no clinical dataset, no
// knowledge base. Each formula is cited by name in its doc comment. These are the
// only medical outputs this library is permitted to compute for real; anything
// requiring a validated model / curated data (diagnosis, imaging readouts, drug
// interaction/affinity prediction, learned prognosis) MUST fail closed with
// `MedicalError::NotImplemented` (see `analyze_data`, `process_image`,
// `screen_compounds`). Never fabricate a medical number.
//
// Inputs are validated (positive where physiologically required); invalid input
// returns `ValidationError` rather than a nonsensical number. Sex-dependent
// formulas reject `Gender::Other`/`Gender::Unknown` rather than silently picking
// a coefficient — the validated coefficient is defined only for male/female.
// ---------------------------------------------------------------------------

/// Summary of a numeric cohort, computed via `crate::solvers::statistics`.
#[derive(Debug, Clone, PartialEq)]
pub struct CohortSummary {
    /// Number of observations.
    pub n: usize,
    /// Arithmetic mean.
    pub mean: f64,
    /// Sample standard deviation (Bessel-corrected, n-1). `None` when n < 2.
    pub std_dev: Option<f64>,
    /// Median.
    pub median: f64,
    /// Minimum.
    pub min: f64,
    /// Maximum.
    pub max: f64,
}

impl MedicalComputingLibrary {
    // -- Anthropometry -----------------------------------------------------

    /// Body Mass Index (Quetelet index): `BMI = weight_kg / height_m²` (kg/m²).
    pub fn bmi(&self, weight_kg: f64, height_m: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_m > 0.0) {
            return Err(MedicalError::ValidationError(
                "bmi: weight_kg and height_m must be positive".to_string(),
            ));
        }
        Ok(weight_kg / (height_m * height_m))
    }

    /// Body Surface Area, Mosteller formula (1987):
    /// `BSA (m²) = sqrt(height_cm × weight_kg / 3600)`.
    pub fn bsa_mosteller(&self, weight_kg: f64, height_cm: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "bsa_mosteller: weight_kg and height_cm must be positive".to_string(),
            ));
        }
        Ok((height_cm * weight_kg / 3600.0).sqrt())
    }

    /// Body Surface Area, Du Bois & Du Bois formula (1916):
    /// `BSA (m²) = 0.007184 × weight_kg^0.425 × height_cm^0.725`.
    pub fn bsa_du_bois(&self, weight_kg: f64, height_cm: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "bsa_du_bois: weight_kg and height_cm must be positive".to_string(),
            ));
        }
        Ok(0.007184 * weight_kg.powf(0.425) * height_cm.powf(0.725))
    }

    /// Ideal Body Weight, Devine formula (1974). In kg:
    /// male   = 50.0  + 2.3 × (height_inches − 60);
    /// female = 45.5  + 2.3 × (height_inches − 60);  height_inches = height_cm / 2.54.
    /// Only defined for male/female (rejects Other/Unknown).
    pub fn ideal_body_weight_devine(
        &self,
        height_cm: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "ideal_body_weight_devine: height_cm must be positive".to_string(),
            ));
        }
        let base = match sex {
            Gender::Male => 50.0,
            Gender::Female => 45.5,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "ideal_body_weight_devine: Devine coefficients are defined only for \
                     male/female; sex Other/Unknown has no validated coefficient".to_string(),
                ))
            }
        };
        let height_inches = height_cm / 2.54;
        Ok(base + 2.3 * (height_inches - 60.0))
    }

    // -- Renal function ----------------------------------------------------

    /// Estimated GFR, CKD-EPI 2021 creatinine equation (race-free), mL/min/1.73 m²:
    /// `eGFR = 142 × min(Scr/κ,1)^α × max(Scr/κ,1)^−1.200 × 0.9938^age × (1.012 if female)`
    /// with κ = 0.7 (female)/0.9 (male), α = −0.241 (female)/−0.302 (male).
    /// `scr_mg_dl` = serum creatinine in mg/dL. Only defined for male/female.
    pub fn egfr_ckd_epi_2021(
        &self,
        scr_mg_dl: f64,
        age_years: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "egfr_ckd_epi_2021: scr_mg_dl and age_years must be positive".to_string(),
            ));
        }
        let (kappa, alpha, female_factor) = match sex {
            Gender::Female => (0.7_f64, -0.241_f64, 1.012_f64),
            Gender::Male => (0.9_f64, -0.302_f64, 1.0_f64),
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "egfr_ckd_epi_2021: CKD-EPI coefficients are defined only for male/female"
                        .to_string(),
                ))
            }
        };
        let ratio = scr_mg_dl / kappa;
        let egfr = 142.0
            * ratio.min(1.0).powf(alpha)
            * ratio.max(1.0).powf(-1.200)
            * 0.9938_f64.powf(age_years)
            * female_factor;
        Ok(egfr)
    }

    /// Estimated GFR, MDRD 4-variable equation (IDMS-traceable, 2006 coefficient
    /// 175), mL/min/1.73 m²:
    /// `eGFR = 175 × Scr^−1.154 × age^−0.203 × (0.742 if female) × (1.212 if Black)`.
    /// `scr_mg_dl` = serum creatinine in mg/dL. Only defined for male/female.
    pub fn egfr_mdrd(
        &self,
        scr_mg_dl: f64,
        age_years: f64,
        sex: Gender,
        is_black: bool,
    ) -> Result<f64, MedicalError> {
        if !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "egfr_mdrd: scr_mg_dl and age_years must be positive".to_string(),
            ));
        }
        let sex_factor = match sex {
            Gender::Female => 0.742,
            Gender::Male => 1.0,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "egfr_mdrd: MDRD sex factor is defined only for male/female".to_string(),
                ))
            }
        };
        let race_factor = if is_black { 1.212 } else { 1.0 };
        Ok(175.0 * scr_mg_dl.powf(-1.154) * age_years.powf(-0.203) * sex_factor * race_factor)
    }

    /// Creatinine clearance, Cockcroft-Gault equation (1976), mL/min:
    /// `CrCl = ((140 − age) × weight_kg × (0.85 if female)) / (72 × Scr_mg/dL)`.
    /// Only defined for male/female.
    pub fn creatinine_clearance_cockcroft_gault(
        &self,
        age_years: f64,
        weight_kg: f64,
        scr_mg_dl: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "creatinine_clearance_cockcroft_gault: age_years, weight_kg and scr_mg_dl \
                 must be positive".to_string(),
            ));
        }
        let sex_factor = match sex {
            Gender::Female => 0.85,
            Gender::Male => 1.0,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "creatinine_clearance_cockcroft_gault: sex factor is defined only for \
                     male/female".to_string(),
                ))
            }
        };
        Ok(((140.0 - age_years) * weight_kg * sex_factor) / (72.0 * scr_mg_dl))
    }

    // -- Hemodynamics & acid-base -----------------------------------------

    /// Mean Arterial Pressure (standard estimate): `MAP = (SBP + 2·DBP) / 3` (mmHg).
    pub fn mean_arterial_pressure(
        &self,
        systolic: f64,
        diastolic: f64,
    ) -> Result<f64, MedicalError> {
        if !(systolic > 0.0) || !(diastolic > 0.0) || diastolic > systolic {
            return Err(MedicalError::ValidationError(
                "mean_arterial_pressure: require systolic >= diastolic > 0".to_string(),
            ));
        }
        Ok((systolic + 2.0 * diastolic) / 3.0)
    }

    /// Serum anion gap: `AG = Na − (Cl + HCO3)` (mEq/L). (Potassium excluded, the
    /// common convention.)
    pub fn anion_gap(&self, na: f64, cl: f64, hco3: f64) -> f64 {
        na - (cl + hco3)
    }

    /// Albumin-corrected calcium (Payne 1973):
    /// `corrected = measured_ca_mg_dl + 0.8 × (4.0 − albumin_g_dl)` (mg/dL).
    pub fn corrected_calcium(
        &self,
        measured_ca_mg_dl: f64,
        albumin_g_dl: f64,
    ) -> Result<f64, MedicalError> {
        if !(measured_ca_mg_dl >= 0.0) || !(albumin_g_dl >= 0.0) {
            return Err(MedicalError::ValidationError(
                "corrected_calcium: measured_ca_mg_dl and albumin_g_dl must be non-negative"
                    .to_string(),
            ));
        }
        Ok(measured_ca_mg_dl + 0.8 * (4.0 - albumin_g_dl))
    }

    /// Winter's formula — expected PaCO₂ compensation for metabolic acidosis:
    /// `expected pCO2 = 1.5 × HCO3 + 8` (mmHg, ±2). Returns the point estimate.
    pub fn winters_expected_pco2(&self, hco3: f64) -> Result<f64, MedicalError> {
        if !(hco3 >= 0.0) {
            return Err(MedicalError::ValidationError(
                "winters_expected_pco2: hco3 must be non-negative".to_string(),
            ));
        }
        Ok(1.5 * hco3 + 8.0)
    }

    // -- Risk scores (pure published point sums) --------------------------

    /// CHA₂DS₂-VASc stroke-risk score (Lip 2010) as its deterministic point sum
    /// (0–9). This is the arithmetic score itself, NOT a risk/probability estimate
    /// (mapping the score to an annual stroke rate needs the validated cohort table,
    /// which is not shipped): CHF/LV dysfunction (1), hypertension (1),
    /// age ≥75 (2) or 65–74 (1), diabetes (1), prior stroke/TIA/thromboembolism (2),
    /// vascular disease (1), female sex (1).
    pub fn cha2ds2_vasc_score(
        &self,
        congestive_heart_failure: bool,
        hypertension: bool,
        age_years: u32,
        diabetes: bool,
        prior_stroke_tia_or_thromboembolism: bool,
        vascular_disease: bool,
        sex: Gender,
    ) -> u8 {
        let mut score: u8 = 0;
        if congestive_heart_failure {
            score += 1;
        }
        if hypertension {
            score += 1;
        }
        if age_years >= 75 {
            score += 2;
        } else if age_years >= 65 {
            score += 1;
        }
        if diabetes {
            score += 1;
        }
        if prior_stroke_tia_or_thromboembolism {
            score += 2;
        }
        if vascular_disease {
            score += 1;
        }
        if matches!(sex, Gender::Female) {
            score += 1;
        }
        score
    }

    // -- Drug dosing math --------------------------------------------------

    /// Weight-based dose: `dose = dose_per_kg × weight_kg` (units follow dose_per_kg).
    pub fn weight_based_dose(
        &self,
        dose_per_kg: f64,
        weight_kg: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose_per_kg >= 0.0) || !(weight_kg > 0.0) {
            return Err(MedicalError::ValidationError(
                "weight_based_dose: dose_per_kg must be non-negative and weight_kg positive"
                    .to_string(),
            ));
        }
        Ok(dose_per_kg * weight_kg)
    }

    /// Renal dose adjustment, Giusti-Hayton method (1973):
    /// `Q = 1 − Fe × (1 − CrCl_patient / CrCl_normal)`;
    /// `adjusted_dose = normal_dose × Q`. `fraction_renally_excreted` (Fe) ∈ [0,1]
    /// is the fraction of drug eliminated unchanged by the kidney.
    pub fn giusti_hayton_adjusted_dose(
        &self,
        normal_dose: f64,
        fraction_renally_excreted: f64,
        crcl_patient: f64,
        crcl_normal: f64,
    ) -> Result<f64, MedicalError> {
        if !(normal_dose >= 0.0) || !(crcl_patient >= 0.0) || !(crcl_normal > 0.0) {
            return Err(MedicalError::ValidationError(
                "giusti_hayton_adjusted_dose: doses/clearances must be non-negative and \
                 crcl_normal positive".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&fraction_renally_excreted) {
            return Err(MedicalError::ValidationError(
                "giusti_hayton_adjusted_dose: fraction_renally_excreted must be in [0,1]"
                    .to_string(),
            ));
        }
        let q = 1.0 - fraction_renally_excreted * (1.0 - crcl_patient / crcl_normal);
        Ok(normal_dose * q)
    }

    /// Convert mass to amount of substance: `mmol = mg / molar_mass_g_per_mol`.
    pub fn mg_to_mmol(&self, mg: f64, molar_mass_g_per_mol: f64) -> Result<f64, MedicalError> {
        if !(molar_mass_g_per_mol > 0.0) {
            return Err(MedicalError::ValidationError(
                "mg_to_mmol: molar_mass_g_per_mol must be positive".to_string(),
            ));
        }
        Ok(mg / molar_mass_g_per_mol)
    }

    /// Convert amount of substance to mass: `mg = mmol × molar_mass_g_per_mol`.
    pub fn mmol_to_mg(&self, mmol: f64, molar_mass_g_per_mol: f64) -> Result<f64, MedicalError> {
        if !(molar_mass_g_per_mol > 0.0) {
            return Err(MedicalError::ValidationError(
                "mmol_to_mg: molar_mass_g_per_mol must be positive".to_string(),
            ));
        }
        Ok(mmol * molar_mass_g_per_mol)
    }

    /// Continuous infusion rate (mL/hr) for a weight-based dose:
    /// `rate = (dose_per_kg_per_min × weight_kg × 60) / concentration_per_ml`.
    /// Units of `dose_per_kg_per_min` and `concentration_per_ml` must match
    /// (e.g. µg/kg/min with µg/mL).
    pub fn infusion_rate_ml_per_hr(
        &self,
        dose_per_kg_per_min: f64,
        weight_kg: f64,
        concentration_per_ml: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose_per_kg_per_min >= 0.0) || !(weight_kg > 0.0) || !(concentration_per_ml > 0.0) {
            return Err(MedicalError::ValidationError(
                "infusion_rate_ml_per_hr: dose/weight non-negative-or-positive and \
                 concentration_per_ml must be positive".to_string(),
            ));
        }
        Ok((dose_per_kg_per_min * weight_kg * 60.0) / concentration_per_ml)
    }

    // -- First-order pharmacokinetics -------------------------------------

    /// First-order elimination rate constant from half-life: `k = ln(2) / t½`.
    pub fn elimination_rate_constant(&self, half_life: f64) -> Result<f64, MedicalError> {
        if !(half_life > 0.0) {
            return Err(MedicalError::ValidationError(
                "elimination_rate_constant: half_life must be positive".to_string(),
            ));
        }
        Ok(std::f64::consts::LN_2 / half_life)
    }

    /// Half-life from first-order rate constant: `t½ = ln(2) / k`.
    pub fn half_life_from_rate_constant(
        &self,
        rate_constant: f64,
    ) -> Result<f64, MedicalError> {
        if !(rate_constant > 0.0) {
            return Err(MedicalError::ValidationError(
                "half_life_from_rate_constant: rate_constant must be positive".to_string(),
            ));
        }
        Ok(std::f64::consts::LN_2 / rate_constant)
    }

    /// Drug clearance from first-order PK: `CL = k × Vd`.
    pub fn clearance(
        &self,
        rate_constant: f64,
        volume_of_distribution: f64,
    ) -> Result<f64, MedicalError> {
        if !(rate_constant >= 0.0) || !(volume_of_distribution >= 0.0) {
            return Err(MedicalError::ValidationError(
                "clearance: rate_constant and volume_of_distribution must be non-negative"
                    .to_string(),
            ));
        }
        Ok(rate_constant * volume_of_distribution)
    }

    /// Apparent volume of distribution: `Vd = dose / C0`.
    pub fn volume_of_distribution(
        &self,
        dose: f64,
        initial_concentration: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose >= 0.0) || !(initial_concentration > 0.0) {
            return Err(MedicalError::ValidationError(
                "volume_of_distribution: dose non-negative and initial_concentration positive"
                    .to_string(),
            ));
        }
        Ok(dose / initial_concentration)
    }

    /// Steady-state concentration under continuous infusion:
    /// `Css = infusion_rate / clearance`.
    pub fn steady_state_concentration(
        &self,
        infusion_rate: f64,
        clearance: f64,
    ) -> Result<f64, MedicalError> {
        if !(infusion_rate >= 0.0) || !(clearance > 0.0) {
            return Err(MedicalError::ValidationError(
                "steady_state_concentration: infusion_rate non-negative and clearance positive"
                    .to_string(),
            ));
        }
        Ok(infusion_rate / clearance)
    }

    // -- Statistics (delegated) -------------------------------------------

    /// Summarise a numeric cohort (e.g. a series of lab values). This is the only
    /// statistical work here and it DELEGATES to `crate::solvers::statistics`
    /// (`descriptive::mean`, `descriptive::std_dev`, `descriptive::median_sorted`).
    pub fn summarize_cohort(&self, values: &[f64]) -> Result<CohortSummary, MedicalError> {
        use crate::solvers::statistics::descriptive;
        if values.is_empty() {
            return Err(MedicalError::ValidationError(
                "summarize_cohort: values must be non-empty".to_string(),
            ));
        }
        let mean = descriptive::mean(values).ok_or_else(|| {
            MedicalError::DataError("summarize_cohort: mean undefined".to_string())
        })?;
        // Sample std-dev is undefined for n < 2 — report None rather than NaN.
        let std_dev = if values.len() >= 2 {
            descriptive::std_dev(values, true)
        } else {
            None
        };
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = descriptive::median_sorted(&sorted).ok_or_else(|| {
            MedicalError::DataError("summarize_cohort: median undefined".to_string())
        })?;
        Ok(CohortSummary {
            n: values.len(),
            mean,
            std_dev,
            median,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        })
    }
}
