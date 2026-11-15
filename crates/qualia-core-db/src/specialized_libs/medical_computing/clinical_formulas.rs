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
                     male/female; sex Other/Unknown has no validated coefficient"
                        .to_string(),
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
                 must be positive"
                    .to_string(),
            ));
        }
        let sex_factor = match sex {
            Gender::Female => 0.85,
            Gender::Male => 1.0,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "creatinine_clearance_cockcroft_gault: sex factor is defined only for \
                     male/female"
                        .to_string(),
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
    pub fn weight_based_dose(&self, dose_per_kg: f64, weight_kg: f64) -> Result<f64, MedicalError> {
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
    /// `adjusted_dose = normal_dose × Q`. `fraction_renally_excreted` (Fe) ∈ \[0,1\]
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
                 crcl_normal positive"
                    .to_string(),
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
                 concentration_per_ml must be positive"
                    .to_string(),
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
    pub fn half_life_from_rate_constant(&self, rate_constant: f64) -> Result<f64, MedicalError> {
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

    // -- Musculoskeletal densitometry & bone-muscle crosstalk (BIO-WP-03) --

    /// DEXA T-score relative to young-adult peak bone mass reference.
    pub fn bmd_t_score(
        &self,
        measured_bmd_g_cm2: f64,
        young_adult_mean_g_cm2: f64,
        young_adult_sd_g_cm2: f64,
    ) -> Result<f64, MedicalError> {
        bmd_t_score(
            measured_bmd_g_cm2,
            young_adult_mean_g_cm2,
            young_adult_sd_g_cm2,
        )
    }

    /// DEXA Z-score relative to age/sex/cohort-matched reference.
    pub fn bmd_z_score(
        &self,
        measured_bmd_g_cm2: f64,
        age_matched_mean_g_cm2: f64,
        age_matched_sd_g_cm2: f64,
    ) -> Result<f64, MedicalError> {
        bmd_z_score(
            measured_bmd_g_cm2,
            age_matched_mean_g_cm2,
            age_matched_sd_g_cm2,
        )
    }

    /// WHO (1994) diagnostic classification from DEXA T-score and fragility fracture history.
    pub fn who_osteoporosis_category(
        &self,
        t_score: f64,
        has_fragility_fracture: bool,
    ) -> Result<WhoOsteoporosisCategory, MedicalError> {
        who_osteoporosis_category(t_score, has_fragility_fracture)
    }

    /// Carter & Hayes (1977) / Zysset (2003) elastic modulus in GPa with mineralization
    /// modifier `mu_min` and trabecular fabric eigenvalue `m_i`.
    pub fn carter_hayes_elastic_modulus_gpa(
        &self,
        apparent_density_g_cm3: f64,
        strain_rate_per_s: f64,
        mu_min: f64,
        fabric_eigenvalue_m_i: f64,
    ) -> Result<f64, MedicalError> {
        carter_hayes_elastic_modulus_gpa(
            apparent_density_g_cm3,
            strain_rate_per_s,
            mu_min,
            fabric_eigenvalue_m_i,
        )
    }

    /// Gibson & Ashby (1997) open-cell trabecular yield strength in MPa.
    pub fn gibson_ashby_yield_strength_mpa(
        &self,
        apparent_density_g_cm3: f64,
        solid_density_g_cm3: f64,
        solid_yield_strength_mpa: f64,
        mu_min: f64,
    ) -> Result<f64, MedicalError> {
        gibson_ashby_yield_strength_mpa(
            apparent_density_g_cm3,
            solid_density_g_cm3,
            solid_yield_strength_mpa,
            mu_min,
        )
    }

    /// Effective muscle Physiological Cross-Sectional Area (PCSA) in cm².
    pub fn effective_muscle_pcsa_cm2(
        &self,
        muscle_volume_cm3: f64,
        optimal_fiber_length_cm: f64,
        pennation_angle_rad: f64,
    ) -> Result<f64, MedicalError> {
        effective_muscle_pcsa_cm2(
            muscle_volume_cm3,
            optimal_fiber_length_cm,
            pennation_angle_rad,
        )
    }

    /// Pennate muscle peak isometric force in Newtons, incorporating CT radiodensity
    /// (`hu_muscle`) and inter/intramuscular adipose (`imat_fraction`) myosteatosis penalties.
    pub fn pennate_muscle_max_force_n(
        &self,
        pcsa_cm2: f64,
        specific_tension_n_cm2: f64,
        pennation_angle_rad: f64,
        hu_muscle: f64,
        imat_fraction: f64,
        neural_activation_factor: f64,
    ) -> Result<f64, MedicalError> {
        pennate_muscle_max_force_n(
            pcsa_cm2,
            specific_tension_n_cm2,
            pennation_angle_rad,
            hu_muscle,
            imat_fraction,
            neural_activation_factor,
        )
    }

    /// Joint Relative Effort Ratio: `RER = |tau_required| / |tau_max|`.
    pub fn relative_effort_ratio(
        &self,
        required_torque_nm: f64,
        max_torque_nm: f64,
    ) -> Result<f64, MedicalError> {
        relative_effort_ratio(required_torque_nm, max_torque_nm)
    }

    /// Hof (2005) dynamic Margin of Stability (m) between Base of Support and XCoM.
    pub fn hof_margin_of_stability_m(
        &self,
        com_pos_m: f64,
        com_vel_m_s: f64,
        pendulum_length_m: f64,
        bos_boundary_m: f64,
        gravity_m_s2: f64,
    ) -> Result<f64, MedicalError> {
        hof_margin_of_stability_m(
            com_pos_m,
            com_vel_m_s,
            pendulum_length_m,
            bos_boundary_m,
            gravity_m_s2,
        )
    }

    /// Skeletal structural Factor of Risk: `Phi = |sigma_applied| / sigma_yield`.
    pub fn bone_fracture_risk_index(
        &self,
        applied_stress_mpa: f64,
        yield_strength_mpa: f64,
    ) -> Result<f64, MedicalError> {
        bone_fracture_risk_index(applied_stress_mpa, yield_strength_mpa)
    }

    /// Gestational/ovulatory relaxin ligament stiffness modifier in `[0.65, 1.0]`.
    pub fn relaxin_ligament_stiffness_modifier(
        &self,
        relaxin_ng_ml: f64,
    ) -> Result<f64, MedicalError> {
        relaxin_ligament_stiffness_modifier(relaxin_ng_ml)
    }

    /// Ontogenetic & endocrine trabecular BMD multiplier relative to young-adult PBM.
    pub fn age_trabecular_bmd_multiplier(
        &self,
        age_years: f64,
        sex: Gender,
        postmenopausal: bool,
    ) -> Result<f64, MedicalError> {
        age_trabecular_bmd_multiplier(age_years, sex, postmenopausal)
    }
}

// ---------------------------------------------------------------------------
// Pure, zero-heap musculoskeletal densitometry & bone-muscle crosstalk
// formulas (BIO-WP-03).
// ---------------------------------------------------------------------------

/// WHO (1994) diagnostic classification for areal bone mineral density (DEXA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhoOsteoporosisCategory {
    /// T-score >= -1.0.
    Normal,
    /// -2.5 < T-score < -1.0 (low bone mass).
    Osteopenia,
    /// T-score <= -2.5 without fragility fracture.
    Osteoporosis,
    /// T-score <= -2.5 with one or more fragility fractures.
    SevereOsteoporosis,
}

/// DEXA T-score: `(measured_bmd - young_adult_mean) / young_adult_sd` (Kanis et al., WHO 1994).
pub fn bmd_t_score(
    measured_bmd_g_cm2: f64,
    young_adult_mean_g_cm2: f64,
    young_adult_sd_g_cm2: f64,
) -> Result<f64, MedicalError> {
    if !measured_bmd_g_cm2.is_finite()
        || !young_adult_mean_g_cm2.is_finite()
        || !young_adult_sd_g_cm2.is_finite()
        || measured_bmd_g_cm2 < 0.0
        || young_adult_mean_g_cm2 <= 0.0
        || young_adult_sd_g_cm2 <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "bmd_t_score: require finite measured_bmd >= 0, young_adult_mean > 0, young_adult_sd > 0"
                .to_string(),
        ));
    }
    Ok((measured_bmd_g_cm2 - young_adult_mean_g_cm2) / young_adult_sd_g_cm2)
}

/// DEXA Z-score: `(measured_bmd - age_matched_mean) / age_matched_sd` (ISCD).
pub fn bmd_z_score(
    measured_bmd_g_cm2: f64,
    age_matched_mean_g_cm2: f64,
    age_matched_sd_g_cm2: f64,
) -> Result<f64, MedicalError> {
    if !measured_bmd_g_cm2.is_finite()
        || !age_matched_mean_g_cm2.is_finite()
        || !age_matched_sd_g_cm2.is_finite()
        || measured_bmd_g_cm2 < 0.0
        || age_matched_mean_g_cm2 <= 0.0
        || age_matched_sd_g_cm2 <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "bmd_z_score: require finite measured_bmd >= 0, age_matched_mean > 0, age_matched_sd > 0"
                .to_string(),
        ));
    }
    Ok((measured_bmd_g_cm2 - age_matched_mean_g_cm2) / age_matched_sd_g_cm2)
}

/// WHO (1994) diagnostic classification from DEXA T-score and fragility fracture history.
pub fn who_osteoporosis_category(
    t_score: f64,
    has_fragility_fracture: bool,
) -> Result<WhoOsteoporosisCategory, MedicalError> {
    if !t_score.is_finite() {
        return Err(MedicalError::ValidationError(
            "who_osteoporosis_category: t_score must be finite".to_string(),
        ));
    }
    if t_score >= -1.0 {
        Ok(WhoOsteoporosisCategory::Normal)
    } else if t_score > -2.5 {
        Ok(WhoOsteoporosisCategory::Osteopenia)
    } else if has_fragility_fracture {
        Ok(WhoOsteoporosisCategory::SevereOsteoporosis)
    } else {
        Ok(WhoOsteoporosisCategory::Osteoporosis)
    }
}

/// Carter & Hayes (1977) / Zysset (2003) bone elastic modulus in GPa:
/// `E = 3.790 * mu_min * rho_app^3 * m_i^2 * strain_rate^0.06`.
pub fn carter_hayes_elastic_modulus_gpa(
    apparent_density_g_cm3: f64,
    strain_rate_per_s: f64,
    mu_min: f64,
    fabric_eigenvalue_m_i: f64,
) -> Result<f64, MedicalError> {
    if !apparent_density_g_cm3.is_finite()
        || !strain_rate_per_s.is_finite()
        || !mu_min.is_finite()
        || !fabric_eigenvalue_m_i.is_finite()
        || apparent_density_g_cm3 < 0.0
        || strain_rate_per_s <= 0.0
        || mu_min < 0.0
        || fabric_eigenvalue_m_i <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "carter_hayes_elastic_modulus_gpa: require finite apparent_density >= 0, mu_min >= 0, strain_rate > 0, fabric_eigenvalue > 0"
                .to_string(),
        ));
    }
    Ok(3.790
        * mu_min
        * apparent_density_g_cm3.powi(3)
        * fabric_eigenvalue_m_i.powi(2)
        * strain_rate_per_s.powf(0.06))
}

/// Gibson & Ashby (1997) open-cell trabecular plastic/yield strength in MPa:
/// `sigma_yield = 0.3 * sigma_solid * mu_min * (rho_app / rho_solid)^1.5`.
pub fn gibson_ashby_yield_strength_mpa(
    apparent_density_g_cm3: f64,
    solid_density_g_cm3: f64,
    solid_yield_strength_mpa: f64,
    mu_min: f64,
) -> Result<f64, MedicalError> {
    if !apparent_density_g_cm3.is_finite()
        || !solid_density_g_cm3.is_finite()
        || !solid_yield_strength_mpa.is_finite()
        || !mu_min.is_finite()
        || apparent_density_g_cm3 < 0.0
        || solid_density_g_cm3 <= 0.0
        || solid_yield_strength_mpa <= 0.0
        || mu_min < 0.0
    {
        return Err(MedicalError::ValidationError(
            "gibson_ashby_yield_strength_mpa: require finite apparent_density >= 0, mu_min >= 0, solid_density > 0, solid_yield_strength > 0"
                .to_string(),
        ));
    }
    let rel_density = apparent_density_g_cm3 / solid_density_g_cm3;
    Ok(0.3 * solid_yield_strength_mpa * mu_min * rel_density.powf(1.5))
}

/// Muscle Physiological Cross-Sectional Area in cm²:
/// `PCSA = (V_muscle * cos(pennation_angle_rad)) / l_opt`.
pub fn effective_muscle_pcsa_cm2(
    muscle_volume_cm3: f64,
    optimal_fiber_length_cm: f64,
    pennation_angle_rad: f64,
) -> Result<f64, MedicalError> {
    if !muscle_volume_cm3.is_finite()
        || !optimal_fiber_length_cm.is_finite()
        || !pennation_angle_rad.is_finite()
        || muscle_volume_cm3 < 0.0
        || optimal_fiber_length_cm <= 0.0
        || !(0.0..std::f64::consts::FRAC_PI_2).contains(&pennation_angle_rad)
    {
        return Err(MedicalError::ValidationError(
            "effective_muscle_pcsa_cm2: require finite volume >= 0, optimal_fiber_length > 0, pennation_angle in [0, pi/2)"
                .to_string(),
        ));
    }
    Ok((muscle_volume_cm3 * pennation_angle_rad.cos()) / optimal_fiber_length_cm)
}

/// Pennate muscle peak isometric force in Newtons incorporating Goodpaster CT
/// radiodensity (`hu_muscle`) and IMAT myosteatosis penalty:
/// `F_max = PCSA * sigma_0 * cos(alpha) * (1 - imat_fraction) * phi_hu(hu_muscle) * phi_neural`,
/// where `phi_hu = clamp((hu_muscle + 20) / 60, 0, 1)` (1.0 for healthy >= +40 HU, degraded below +40 HU).
pub fn pennate_muscle_max_force_n(
    pcsa_cm2: f64,
    specific_tension_n_cm2: f64,
    pennation_angle_rad: f64,
    hu_muscle: f64,
    imat_fraction: f64,
    neural_activation_factor: f64,
) -> Result<f64, MedicalError> {
    if !pcsa_cm2.is_finite()
        || !specific_tension_n_cm2.is_finite()
        || !pennation_angle_rad.is_finite()
        || !hu_muscle.is_finite()
        || !imat_fraction.is_finite()
        || !neural_activation_factor.is_finite()
        || pcsa_cm2 < 0.0
        || specific_tension_n_cm2 <= 0.0
        || !(0.0..std::f64::consts::FRAC_PI_2).contains(&pennation_angle_rad)
        || !(-1000.0..=3000.0).contains(&hu_muscle)
        || !(0.0..=1.0).contains(&imat_fraction)
        || !(0.0..=1.0).contains(&neural_activation_factor)
    {
        return Err(MedicalError::ValidationError(
            "pennate_muscle_max_force_n: invalid biomechanical or myosteatosis parameter"
                .to_string(),
        ));
    }
    let hu_quality = ((hu_muscle + 20.0) / 60.0).clamp(0.0, 1.0);
    let myo_quality = (1.0 - imat_fraction) * hu_quality * neural_activation_factor;
    Ok(pcsa_cm2 * specific_tension_n_cm2 * pennation_angle_rad.cos() * myo_quality)
}

/// Joint Relative Effort Ratio: `RER = |tau_required| / |tau_max|`.
pub fn relative_effort_ratio(
    required_torque_nm: f64,
    max_torque_nm: f64,
) -> Result<f64, MedicalError> {
    if !required_torque_nm.is_finite()
        || !max_torque_nm.is_finite()
        || max_torque_nm.abs() <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "relative_effort_ratio: require finite torques and |max_torque_nm| > 0".to_string(),
        ));
    }
    Ok(required_torque_nm.abs() / max_torque_nm.abs())
}

/// Hof, Gazendam & Sinke (2005) Margin of Stability (m):
/// `XCoM = com_pos + com_vel / sqrt(g / l_pendulum)`, `MoS = bos_boundary - XCoM`.
pub fn hof_margin_of_stability_m(
    com_pos_m: f64,
    com_vel_m_s: f64,
    pendulum_length_m: f64,
    bos_boundary_m: f64,
    gravity_m_s2: f64,
) -> Result<f64, MedicalError> {
    if !com_pos_m.is_finite()
        || !com_vel_m_s.is_finite()
        || !pendulum_length_m.is_finite()
        || !bos_boundary_m.is_finite()
        || !gravity_m_s2.is_finite()
        || pendulum_length_m <= 0.0
        || gravity_m_s2 <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "hof_margin_of_stability_m: require finite kinematics and positive pendulum_length_m, gravity_m_s2"
                .to_string(),
        ));
    }
    let omega0 = (gravity_m_s2 / pendulum_length_m).sqrt();
    let xcom = com_pos_m + com_vel_m_s / omega0;
    Ok(bos_boundary_m - xcom)
}

/// Skeletal Structural Factor of Risk: `Phi = |applied_stress_mpa| / yield_strength_mpa`.
/// Values `>= 1.0` indicate structural yield/fracture risk.
pub fn bone_fracture_risk_index(
    applied_stress_mpa: f64,
    yield_strength_mpa: f64,
) -> Result<f64, MedicalError> {
    if !applied_stress_mpa.is_finite()
        || !yield_strength_mpa.is_finite()
        || yield_strength_mpa <= 0.0
    {
        return Err(MedicalError::ValidationError(
            "bone_fracture_risk_index: require finite stresses and yield_strength_mpa > 0"
                .to_string(),
        ));
    }
    Ok(applied_stress_mpa.abs() / yield_strength_mpa)
}

/// Smooth relaxin-mediated ligamentous stiffness modifier in `[0.65, 1.0]`:
/// `1.0 - 0.35 * (relaxin_ng_ml / (relaxin_ng_ml + 0.5))`.
pub fn relaxin_ligament_stiffness_modifier(relaxin_ng_ml: f64) -> Result<f64, MedicalError> {
    if !relaxin_ng_ml.is_finite() || relaxin_ng_ml < 0.0 {
        return Err(MedicalError::ValidationError(
            "relaxin_ligament_stiffness_modifier: relaxin_ng_ml must be finite and non-negative"
                .to_string(),
        ));
    }
    Ok(1.0 - 0.35 * (relaxin_ng_ml / (relaxin_ng_ml + 0.5)))
}

/// Ontogenetic & endocrine trabecular BMD multiplier relative to young-adult Peak Bone Mass
/// (ages 25–35 = 1.0). Only defined for `Gender::Male` and `Gender::Female`.
pub fn age_trabecular_bmd_multiplier(
    age_years: f64,
    sex: Gender,
    postmenopausal: bool,
) -> Result<f64, MedicalError> {
    if !age_years.is_finite() || !(0.0..=130.0).contains(&age_years) {
        return Err(MedicalError::ValidationError(
            "age_trabecular_bmd_multiplier: age_years must be finite and in [0, 130]".to_string(),
        ));
    }
    match sex {
        Gender::Other | Gender::Unknown => {
            return Err(MedicalError::ValidationError(
                "age_trabecular_bmd_multiplier: validated coefficients are defined only for male/female"
                    .to_string(),
            ));
        }
        Gender::Male | Gender::Female => {}
    }

    if age_years < 25.0 {
        Ok(0.45 + 0.55 * (age_years / 25.0))
    } else if age_years <= 35.0 {
        Ok(1.0)
    } else {
        let years_past_peak = age_years - 35.0;
        let base_decline = 0.006 * years_past_peak;
        if matches!(sex, Gender::Female) && postmenopausal {
            let estrogen_withdrawal = 0.18 * (1.0 - (-0.10 * years_past_peak).exp());
            Ok((1.0 - base_decline - estrogen_withdrawal).max(0.35))
        } else {
            Ok((1.0 - base_decline).max(0.40))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) {
        assert!(
            (a - b).abs() <= tol,
            "expected {a} to be within {tol} of {b}"
        );
    }

    #[test]
    fn test_who_densitometry_and_aging_osteoporosis() {
        let lib = MedicalComputingLibrary::new();
        // Normal young adult female: measured 1.05, mean 1.00, sd 0.12 => T = +0.4167
        let t_norm = lib.bmd_t_score(1.05, 1.00, 0.12).unwrap();
        approx_eq(t_norm, 0.4166666667, 1e-6);
        assert_eq!(
            lib.who_osteoporosis_category(t_norm, false).unwrap(),
            WhoOsteoporosisCategory::Normal
        );

        // Osteopenic: measured 0.80 => T = -1.6667
        let t_penia = bmd_t_score(0.80, 1.00, 0.12).unwrap();
        assert_eq!(
            who_osteoporosis_category(t_penia, false).unwrap(),
            WhoOsteoporosisCategory::Osteopenia
        );

        // Osteoporosis without vs with fragility fracture
        let t_porosis = bmd_t_score(0.65, 1.00, 0.12).unwrap(); // -2.9167
        assert_eq!(
            who_osteoporosis_category(t_porosis, false).unwrap(),
            WhoOsteoporosisCategory::Osteoporosis
        );
        assert_eq!(
            who_osteoporosis_category(t_porosis, true).unwrap(),
            WhoOsteoporosisCategory::SevereOsteoporosis
        );

        // Z-score
        let z = lib.bmd_z_score(0.75, 0.85, 0.10).unwrap();
        approx_eq(z, -1.0, 1e-9);

        // Aging & postmenopausal trabecular BMD multiplier
        let pediatric = age_trabecular_bmd_multiplier(10.0, Gender::Female, false).unwrap();
        let pbm = age_trabecular_bmd_multiplier(30.0, Gender::Female, false).unwrap();
        let elderly_male = age_trabecular_bmd_multiplier(75.0, Gender::Male, false).unwrap();
        let elderly_postmeno =
            age_trabecular_bmd_multiplier(75.0, Gender::Female, true).unwrap();
        assert!(pediatric < pbm);
        approx_eq(pbm, 1.0, 1e-9);
        assert!(elderly_postmeno < elderly_male && elderly_male < pbm);
        assert!(age_trabecular_bmd_multiplier(40.0, Gender::Other, false).is_err());
    }

    #[test]
    fn test_myosteatosis_and_muscle_force_crosstalk() {
        let pcsa = effective_muscle_pcsa_cm2(400.0, 10.0, 0.25).unwrap();
        assert!(pcsa > 38.0 && pcsa < 40.0);

        // Healthy muscle: +50 HU, 3% IMAT, full neural activation
        let f_healthy = pennate_muscle_max_force_n(pcsa, 40.0, 0.25, 50.0, 0.03, 1.0).unwrap();
        // Myosteatotic muscle: +15 HU (< +30 HU), 22% IMAT, 85% neural activation
        let f_myo = pennate_muscle_max_force_n(pcsa, 40.0, 0.25, 15.0, 0.22, 0.85).unwrap();
        assert!(f_myo < 0.55 * f_healthy);

        // Lower max torque increases Relative Effort Ratio (RER)
        let rer_healthy = relative_effort_ratio(60.0, 120.0).unwrap();
        let rer_sarcopenic = relative_effort_ratio(60.0, 50.0).unwrap();
        approx_eq(rer_healthy, 0.5, 1e-9);
        assert!(rer_sarcopenic > 1.0);

        // Hof Margin of Stability: forward velocity reduces stability margin
        let mos_quiet = hof_margin_of_stability_m(0.05, 0.0, 1.0, 0.15, 9.81).unwrap();
        let mos_fast = hof_margin_of_stability_m(0.05, 0.5, 1.0, 0.15, 9.81).unwrap();
        approx_eq(mos_quiet, 0.10, 1e-9);
        assert!(mos_fast < 0.0);
    }

    #[test]
    fn test_pregnancy_relaxin_modulation() {
        let baseline = relaxin_ligament_stiffness_modifier(0.0).unwrap();
        let first_trimester = relaxin_ligament_stiffness_modifier(1.0).unwrap();
        let peak_surge = relaxin_ligament_stiffness_modifier(2.5).unwrap();

        approx_eq(baseline, 1.0, 1e-9);
        assert!(first_trimester < baseline && first_trimester > 0.70);
        assert!(peak_surge < first_trimester && peak_surge >= 0.65);
        assert!(relaxin_ligament_stiffness_modifier(-0.1).is_err());
    }

    #[test]
    fn test_comparative_avian_cetacean_sirenian_bone_densities() {
        // Sirenian (manatee/dugong) pachyosteosclerotic rib: rho_app ~ 2.0 g/cm^3
        let e_sirenian = carter_hayes_elastic_modulus_gpa(2.00, 0.01, 1.0, 1.0).unwrap();
        let y_sirenian = gibson_ashby_yield_strength_mpa(2.00, 2.05, 140.0, 1.0).unwrap();

        // Avian pneumatic femur cortex: hyper-mineralized (mu_min = 1.10), moderate apparent density
        let e_avian = carter_hayes_elastic_modulus_gpa(1.75, 0.01, 1.10, 1.15).unwrap();

        // Pelagic cetacean deep-diving osteoporotic spongiosa: low BV/TV (rho_app ~ 0.25 g/cm^3)
        let e_cetacean = carter_hayes_elastic_modulus_gpa(0.25, 0.01, 0.95, 1.0).unwrap();
        let y_cetacean = gibson_ashby_yield_strength_mpa(0.25, 1.85, 140.0, 0.95).unwrap();

        // Human osteomalacic trabecular bone (mineralization deficit mu_min = 0.55)
        let y_osteomalacia = gibson_ashby_yield_strength_mpa(0.30, 1.85, 140.0, 0.55).unwrap();
        let y_normal_trab = gibson_ashby_yield_strength_mpa(0.30, 1.85, 140.0, 1.0).unwrap();

        assert!(e_sirenian > e_avian && e_avian > e_cetacean);
        assert!(y_sirenian > y_cetacean);
        assert!(y_osteomalacia < y_normal_trab);

        // Fracture risk index: Phi >= 1.0 at or above yield
        let phi_safe = bone_fracture_risk_index(1.0, y_normal_trab).unwrap();
        let phi_fail = bone_fracture_risk_index(y_osteomalacia * 1.1, y_osteomalacia).unwrap();
        assert!(phi_safe < 1.0);
        assert!(phi_fail > 1.0);
    }

    #[test]
    fn test_fail_closed_validation_edge_cases() {
        assert!(bmd_t_score(f64::NAN, 1.0, 0.1).is_err());
        assert!(bmd_t_score(-0.1, 1.0, 0.1).is_err());
        assert!(bmd_z_score(1.0, 1.0, 0.0).is_err());
        assert!(who_osteoporosis_category(f64::INFINITY, false).is_err());
        assert!(carter_hayes_elastic_modulus_gpa(1.0, 0.0, 1.0, 1.0).is_err());
        assert!(gibson_ashby_yield_strength_mpa(0.3, 0.0, 140.0, 1.0).is_err());
        assert!(effective_muscle_pcsa_cm2(100.0, 10.0, 1.6).is_err());
        assert!(pennate_muscle_max_force_n(20.0, 40.0, 0.2, 40.0, 1.2, 1.0).is_err());
        assert!(relative_effort_ratio(10.0, 0.0).is_err());
        assert!(hof_margin_of_stability_m(0.0, 0.0, 0.0, 0.1, 9.81).is_err());
        assert!(bone_fracture_risk_index(10.0, -5.0).is_err());
    }
}
