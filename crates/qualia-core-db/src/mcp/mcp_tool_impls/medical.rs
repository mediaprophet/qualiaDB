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
        use crate::specialized_libs::medical_computing::{ConditionModel, DiagnosticKnowledgeBase};
        use std::collections::HashMap;
        let findings: Vec<String> = v
            .get("findings")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
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
                            condition_id: c
                                .get("condition_id")
                                .and_then(Value::as_str)?
                                .to_string(),
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
