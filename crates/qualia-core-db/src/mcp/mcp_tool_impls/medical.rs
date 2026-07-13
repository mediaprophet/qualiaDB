use super::*;


pub fn medical_score(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::medical_computing::{
        ClinicalDataType, MedicalComputingLibrary, MedicalError, Patient,
    };

    let v = parse_tool_args(args)?;
    let score = json_str(&v, "score", "diagnosis");
    let patient_id = v
        .get("patient_id")
        .and_then(Value::as_str)
        .unwrap_or("mcp_patient")
        .to_string();

    let mut lib = MedicalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    // Real image DSP over a caller-provided intensity grid (signal-processing
    // metrics only — the result is explicitly NOT a diagnosis).
    // { "op":"image", "data":[...], "width":W, "height":H, "bins":B, "threshold":T? }
    if json_str(&v, "op", "") == "image" {
        use crate::specialized_libs::medical_computing::SegmentationThreshold;
        let data: Vec<f64> = v
            .get("data")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        let width = v.get("width").and_then(Value::as_u64).unwrap_or(0) as usize;
        let height = v.get("height").and_then(Value::as_u64).unwrap_or(0) as usize;
        let bins = v.get("bins").and_then(Value::as_u64).unwrap_or(16) as usize;
        let threshold = match v.get("threshold").and_then(Value::as_f64) {
            Some(t) => SegmentationThreshold::Fixed(t),
            None => SegmentationThreshold::Otsu,
        };
        let r = lib
            .analyze_medical_image_grid(&data, width, height, bins, threshold, None)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        let im = r.result;
        return Ok(json!({
            "op": "image", "epistemic_status": im.epistemic_status,
            "min": im.min, "max": im.max, "mean": im.mean, "std_dev": im.std_dev,
            "segmented_area": im.segmented_area, "segmented_mean_intensity": im.segmented_mean_intensity
        })
        .to_string());
    }

    // Transparent naive-Bayes differential over a CALLER-SUPPLIED knowledge base.
    // Returns a ranked epistemic PROPOSAL, explicitly NOT a diagnosis.
    // { "op":"differential", "findings":["fever",...],
    //   "knowledge_base":[{ "condition_id":"flu", "prior":0.6,
    //     "likelihoods":{"fever":0.9,"cough":0.8} }, ...],
    //   "unlisted_finding_likelihood": 0.5 }
    if json_str(&v, "op", "") == "differential" {
        use crate::specialized_libs::medical_computing::{
            ConditionModel, DiagnosticKnowledgeBase,
        };
        use std::collections::HashMap;
        let findings: Vec<String> = v
            .get("findings")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        let conditions: Vec<ConditionModel> = v
            .get("knowledge_base")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|c| {
                        let mut likelihoods = HashMap::new();
                        if let Some(m) = c.get("likelihoods").and_then(Value::as_object) {
                            for (k, val) in m {
                                if let Some(p) = val.as_f64() {
                                    likelihoods.insert(k.clone(), p);
                                }
                            }
                        }
                        Some(ConditionModel {
                            condition_id: c.get("condition_id").and_then(Value::as_str)?.to_string(),
                            prior: c.get("prior").and_then(Value::as_f64)?,
                            likelihoods,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let kb = DiagnosticKnowledgeBase {
            conditions,
            unlisted_finding_likelihood: json_f64(&v, "unlisted_finding_likelihood", 0.5),
        };
        let r = lib
            .analyze_differential(&findings, &kb)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        let prop = r.result;
        let ranked: Vec<Value> = prop
            .ranked
            .iter()
            .map(|c| json!({"condition_id": c.condition_id, "prior": c.prior, "posterior": c.posterior}))
            .collect();
        return Ok(json!({
            "op": "differential", "epistemic_status": prop.epistemic_status,
            "observed_findings": prop.observed_findings, "ranked": ranked
        })
        .to_string());
    }

    // Genuinely-computable clinical metrics (published deterministic formulas).
    // Selected by the `metric` field; each delegates to the real library method.
    if let Some(metric) = v.get("metric").and_then(Value::as_str) {
        use crate::specialized_libs::medical_computing::Gender;
        let g = |key: &str| -> Gender {
            match v.get(key).and_then(Value::as_str).unwrap_or("unknown") {
                "male" | "m" => Gender::Male,
                "female" | "f" => Gender::Female,
                "other" => Gender::Other,
                _ => Gender::Unknown,
            }
        };
        let f = |key: &str| json_f64(&v, key, 0.0);
        let val: f64 = match metric {
            "bmi" => lib.bmi(f("weight_kg"), f("height_m")),
            "bsa_mosteller" => lib.bsa_mosteller(f("weight_kg"), f("height_cm")),
            "bsa_du_bois" => lib.bsa_du_bois(f("weight_kg"), f("height_cm")),
            "ideal_body_weight" => lib.ideal_body_weight_devine(f("height_cm"), g("sex")),
            "egfr_ckd_epi" => lib.egfr_ckd_epi_2021(f("scr_mg_dl"), f("age_years"), g("sex")),
            "egfr_mdrd" => lib.egfr_mdrd(
                f("scr_mg_dl"),
                f("age_years"),
                g("sex"),
                json_bool(&v, "is_black", false),
            ),
            "creatinine_clearance" => lib.creatinine_clearance_cockcroft_gault(
                f("age_years"),
                f("weight_kg"),
                f("scr_mg_dl"),
                g("sex"),
            ),
            "mean_arterial_pressure" => lib.mean_arterial_pressure(f("systolic"), f("diastolic")),
            "anion_gap" => Ok(lib.anion_gap(f("na"), f("cl"), f("hco3"))),
            "corrected_calcium" => lib.corrected_calcium(f("measured_ca_mg_dl"), f("albumin_g_dl")),
            "winters_pco2" => lib.winters_expected_pco2(f("hco3")),
            "half_life" => lib.half_life_from_rate_constant(f("rate_constant")),
            "elimination_rate_constant" => lib.elimination_rate_constant(f("half_life")),
            "clearance" => lib.clearance(f("rate_constant"), f("volume_of_distribution")),
            "volume_of_distribution" => {
                lib.volume_of_distribution(f("dose"), f("initial_concentration"))
            }
            "steady_state_concentration" => {
                lib.steady_state_concentration(f("infusion_rate"), f("clearance"))
            }
            "weight_based_dose" => lib.weight_based_dose(f("dose_per_kg"), f("weight_kg")),
            "cha2ds2_vasc" => {
                let s = lib.cha2ds2_vasc_score(
                    json_bool(&v, "chf", false),
                    json_bool(&v, "hypertension", false),
                    f("age_years") as u32,
                    json_bool(&v, "diabetes", false),
                    json_bool(&v, "prior_stroke", false),
                    json_bool(&v, "vascular_disease", false),
                    g("sex"),
                );
                return Ok(json!({"metric": "cha2ds2_vasc", "score": s,
                    "note": "deterministic point sum (0-9), not a risk probability"})
                .to_string());
            }
            _ => return Err(McpSystemError::InvalidParameters),
        }
        .map_err(|_| McpSystemError::InvalidParameters)?;
        return Ok(json!({"metric": metric, "value": val}).to_string());
    }

    let patient: Patient = if let Some(p) = v.get("patient") {
        serde_json::from_value(p.clone()).map_err(|_| McpSystemError::InvalidParameters)?
    } else {
        let mut p = Patient::new();
        p.patient_id = patient_id.clone();
        p
    };
    lib.create_patient_record(patient)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let data_type = match score {
        "treatment" => ClinicalDataType::Treatment,
        "prognosis" => ClinicalDataType::Prognosis,
        "prevention" => ClinicalDataType::Prevention,
        _ => ClinicalDataType::Diagnosis,
    };
    let r = lib
        .analyze_clinical_data(&patient_id, data_type)
        // Honesty propagates to the tool surface: a not-implemented / data-missing capability
        // reports "not implemented", never a fabricated diagnosis dressed as a valid result.
        .map_err(|e| match e {
            MedicalError::NotImplemented(_) | MedicalError::InsufficientData(_) => {
                McpSystemError::ToolNotReady
            }
            _ => McpSystemError::InvalidParameters,
        })?;

    Ok(json!({
        "patient_id": patient_id,
        "analysis_id": r.result.analysis_id,
        "confidence": r.result.confidence_score,
        "recommendations": r.result.recommendations,
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn clinical_risk(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::clinical_engine::{
        cha2ds2_vasc_score, ckd_epi_egfr, framingham_10yr_risk, sofa_score, Cha2ds2VascInput,
        FraminghamInput, RenalInput, SofaInput,
    };

    let v = parse_tool_args(args)?;
    let score_type = json_str(&v, "score", "framingham");
    let input = v.get("input").cloned().unwrap_or_else(|| v.clone());

    let result = match score_type {
        "cha2ds2" | "cha2ds2_vasc" => {
            let inp = Cha2ds2VascInput {
                congestive_heart_failure: json_bool(&input, "congestive_heart_failure", false),
                hypertension: json_bool(&input, "hypertension", false),
                age_75_or_older: json_bool(&input, "age_75_or_older", false),
                diabetes: json_bool(&input, "diabetes", false),
                stroke_tia_history: json_bool(&input, "stroke_tia_history", false),
                vascular_disease: json_bool(&input, "vascular_disease", false),
                age_65_to_74: json_bool(&input, "age_65_to_74", false),
                sex_female: json_bool(&input, "sex_female", false),
            };
            let r = cha2ds2_vasc_score(&inp);
            json!({
                "score": "cha2ds2_vasc",
                "points": r.score,
                "annual_stroke_risk_pct": r.annual_stroke_risk_pct,
                "anticoagulation_recommended": r.anticoagulation_recommended
            })
        }
        "sofa" => {
            let inp = SofaInput {
                pao2_fio2_ratio: json_f64(&input, "pao2_fio2_ratio", 300.0),
                platelets_10_9_l: json_f64(&input, "platelets_10_9_l", 150.0),
                bilirubin_mg_dl: json_f64(&input, "bilirubin_mg_dl", 1.0),
                map_mmhg: json_f64(&input, "map_mmhg", 70.0),
                dopamine_dose: json_f64(&input, "dopamine_dose", 0.0),
                epinephrine_dose: json_f64(&input, "epinephrine_dose", 0.0),
                norepinephrine_dose: json_f64(&input, "norepinephrine_dose", 0.0),
                glasgow_coma_scale: input
                    .get("glasgow_coma_scale")
                    .and_then(Value::as_u64)
                    .unwrap_or(15) as u8,
                creatinine_mg_dl: json_f64(&input, "creatinine_mg_dl", 1.0),
                urine_output_ml_d: json_f64(&input, "urine_output_ml_d", 1000.0),
            };
            json!({"score": "sofa", "points": sofa_score(&inp)})
        }
        "egfr" | "renal" => {
            let inp = RenalInput {
                age: v.get("age").and_then(Value::as_u64).unwrap_or(55) as u8,
                sex_male: json_bool(&input, "sex_male", true),
                weight_kg: json_f64(&input, "weight_kg", 70.0),
                serum_creatinine: json_f64(&input, "serum_creatinine", 1.0),
            };
            json!({"score": "egfr", "egfr_ml_min": ckd_epi_egfr(&inp)})
        }
        _ => {
            let inp = FraminghamInput {
                age: v.get("age").and_then(Value::as_u64).unwrap_or(55) as u8,
                sex_male: json_bool(&input, "sex_male", true),
                total_cholesterol_mmol: json_f64(&input, "total_cholesterol_mmol", 5.5),
                hdl_cholesterol_mmol: json_f64(&input, "hdl_cholesterol_mmol", 1.2),
                systolic_bp: json_f64(&input, "systolic_bp", 130.0),
                bp_treated: json_bool(&input, "bp_treated", false),
                current_smoker: json_bool(&input, "current_smoker", false),
                diabetic: json_bool(&input, "diabetic", false),
            };
            let r = framingham_10yr_risk(&inp);
            json!({
                "score": "framingham",
                "risk_10yr": r.risk_10yr,
                "risk_10yr_pct": r.risk_10yr * 100.0,
                "category": format!("{:?}", r.category),
                "log_score": r.log_score
            })
        }
    };

    Ok(result.to_string())
}
