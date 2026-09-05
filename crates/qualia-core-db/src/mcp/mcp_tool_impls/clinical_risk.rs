//! MCP `clinical_risk`. Missing fields fail closed; they are never patient values.

use super::{parse_tool_args, McpSystemError};
use crate::clinical_engine::{
    cha2ds2_vasc_score, ckd_epi_egfr, framingham_10yr_risk, score2_risk, sofa_score,
    Cha2ds2VascInput, FraminghamInput, RenalInput, Score2Input, Score2Region, SofaInput,
};
use serde_json::{json, Map, Value};

const NOT_ADVICE: &str =
    "This number is not a diagnosis, treatment recommendation, or clinical advice.";

pub fn clinical_risk(args: &[u8]) -> Result<String, McpSystemError> {
    let root = parse_tool_args(args)?;
    let v = flatten(&root);
    let score = need_str(&v, "score")?;
    let result = match score {
        "framingham" => framingham(&v)?,
        "cha2ds2" | "cha2ds2_vasc" => cha2ds2(&v)?,
        "score2" => score2(&v)?,
        "sofa" => sofa(&v)?,
        "egfr" | "renal" => egfr(&v)?,
        _ => return Err(McpSystemError::InvalidParameters),
    };
    Ok(result.to_string())
}

fn flatten(root: &Value) -> Value {
    let mut map: Map<String, Value> = root.as_object().cloned().unwrap_or_default();
    if let Some(inner) = root.get("input").and_then(Value::as_object) {
        for (key, value) in inner {
            map.insert(key.clone(), value.clone());
        }
    }
    Value::Object(map)
}

fn need_str<'a>(v: &'a Value, key: &str) -> Result<&'a str, McpSystemError> {
    let value = v
        .get(key)
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    if value.trim().is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(value)
}

fn need_bool(v: &Value, key: &str) -> Result<bool, McpSystemError> {
    v.get(key)
        .and_then(Value::as_bool)
        .ok_or(McpSystemError::InvalidParameters)
}

fn need_f64(v: &Value, key: &str) -> Result<f64, McpSystemError> {
    let value = v
        .get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_u64().map(|n| n as f64)))
        .ok_or(McpSystemError::InvalidParameters)?;
    if !value.is_finite() {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(value)
}

fn need_u8(v: &Value, key: &str) -> Result<u8, McpSystemError> {
    let value = v
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(McpSystemError::InvalidParameters)?;
    u8::try_from(value).map_err(|_| McpSystemError::InvalidParameters)
}

fn need_age(v: &Value, min_inclusive: u8, max_inclusive: u8) -> Result<u8, McpSystemError> {
    let age = need_u8(v, "age")?;
    if age < min_inclusive || age > max_inclusive {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(age)
}

fn need_positive_f64(
    v: &Value,
    key: &str,
    min_exclusive: f64,
    max_inclusive: f64,
) -> Result<f64, McpSystemError> {
    let value = need_f64(v, key)?;
    if value <= min_exclusive || value > max_inclusive {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(value)
}

fn lipids_ordered(total: f64, hdl: f64) -> Result<(), McpSystemError> {
    if hdl >= total {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(())
}

fn provenance(
    algorithm: &str,
    version: &str,
    citation: &str,
    applicability: &str,
    units: &str,
) -> Value {
    json!({
        "algorithm": algorithm,
        "version": version,
        "citation": citation,
        "applicability": applicability,
        "units": units,
        "not_diagnosis": true,
        "not_advice": NOT_ADVICE,
    })
}

fn merge_result(mut body: Value, meta: Value) -> Value {
    if let (Some(target), Some(extra)) = (body.as_object_mut(), meta.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    body
}

fn framingham(v: &Value) -> Result<Value, McpSystemError> {
    let input = FraminghamInput {
        age: need_age(v, 30, 74)?,
        sex_male: need_bool(v, "sex_male")?,
        total_cholesterol_mmol: need_positive_f64(v, "total_cholesterol_mmol", 0.0, 20.0)?,
        hdl_cholesterol_mmol: need_positive_f64(v, "hdl_cholesterol_mmol", 0.0, 5.0)?,
        systolic_bp: need_positive_f64(v, "systolic_bp", 0.0, 260.0)?,
        bp_treated: need_bool(v, "bp_treated")?,
        current_smoker: need_bool(v, "current_smoker")?,
        diabetic: need_bool(v, "diabetic")?,
    };
    lipids_ordered(input.total_cholesterol_mmol, input.hdl_cholesterol_mmol)?;
    let r = framingham_10yr_risk(&input);
    Ok(merge_result(
        json!({
            "score": "framingham",
            "risk_10yr": r.risk_10yr,
            "risk_10yr_pct": r.risk_10yr * 100.0,
            "category": format!("{:?}", r.category),
            "log_score": r.log_score
        }),
        provenance(
            "Framingham 10-year CVD risk",
            "wilson-1998-atp3",
            "Wilson PW et al. Circulation 1998;97:1837-47 (ATP III sex-specific)",
            "adults 30–74 years; lipid and blood-pressure inputs required",
            "age years; total_cholesterol_mmol mmol/L; hdl_cholesterol_mmol mmol/L; systolic_bp mmHg; risk_10yr fraction 0–1",
        ),
    ))
}

fn cha2ds2(v: &Value) -> Result<Value, McpSystemError> {
    if !need_bool(v, "atrial_fibrillation")? {
        return Err(McpSystemError::InvalidParameters);
    }
    let age = need_age(v, 18, 120)?;
    let input = Cha2ds2VascInput {
        congestive_heart_failure: need_bool(v, "congestive_heart_failure")?,
        hypertension: need_bool(v, "hypertension")?,
        age_75_or_older: age >= 75,
        diabetes: need_bool(v, "diabetes")?,
        stroke_tia_history: need_bool(v, "stroke_tia_history")?,
        vascular_disease: need_bool(v, "vascular_disease")?,
        age_65_to_74: (65..75).contains(&age),
        sex_female: need_bool(v, "sex_female")?,
    };
    let r = cha2ds2_vasc_score(&input);
    Ok(merge_result(
        json!({
            "score": "cha2ds2_vasc",
            "points": r.score,
            "annual_stroke_risk_pct": r.annual_stroke_risk_pct,
            "anticoagulation_recommended": r.anticoagulation_recommended,
            "age_years": age
        }),
        provenance(
            "CHA₂DS₂-VASc stroke risk",
            "lip-2010-esc-2020",
            "Lip GY et al. Chest 2010; ESC 2020 atrial-fibrillation guidelines",
            "non-valvular atrial fibrillation in adults 18–120 years",
            "age years; score points; annual_stroke_risk_percent percent per year",
        ),
    ))
}

fn parse_region(v: &Value) -> Result<Score2Region, McpSystemError> {
    match need_str(v, "risk_region")?
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "low" => Ok(Score2Region::Low),
        "moderate" => Ok(Score2Region::Moderate),
        "high" => Ok(Score2Region::High),
        "very_high" | "veryhigh" => Ok(Score2Region::VeryHigh),
        _ => Err(McpSystemError::InvalidParameters),
    }
}

fn score2(v: &Value) -> Result<Value, McpSystemError> {
    let input = Score2Input {
        age: need_age(v, 40, 69)?,
        sex_male: need_bool(v, "sex_male")?,
        systolic_bp: need_positive_f64(v, "systolic_bp", 0.0, 260.0)?,
        total_cholesterol_mmol: need_positive_f64(v, "total_cholesterol_mmol", 0.0, 20.0)?,
        hdl_cholesterol_mmol: need_positive_f64(v, "hdl_cholesterol_mmol", 0.0, 5.0)?,
        current_smoker: need_bool(v, "current_smoker")?,
        risk_region: parse_region(v)?,
    };
    lipids_ordered(input.total_cholesterol_mmol, input.hdl_cholesterol_mmol)?;
    let r = score2_risk(&input);
    Ok(merge_result(
        json!({
            "score": "score2",
            "risk_percent": r.risk_10yr_pct,
            "category": format!("{:?}", r.category),
            "risk_region": format!("{:?}", input.risk_region)
        }),
        provenance(
            "SCORE2 10-year CVD risk",
            "score2-2021",
            "SCORE2 working group / ESC CVD Risk Collaboration 2021",
            "adults 40–69 years; European risk region required (not defaulted)",
            "age years; total_cholesterol_mmol mmol/L; hdl_cholesterol_mmol mmol/L; systolic_bp mmHg; risk_percent percent over 10 years",
        ),
    ))
}

fn sofa(v: &Value) -> Result<Value, McpSystemError> {
    let gcs = need_u8(v, "glasgow_coma_scale")?;
    if !(3..=15).contains(&gcs) {
        return Err(McpSystemError::InvalidParameters);
    }
    let input = SofaInput {
        pao2_fio2_ratio: need_positive_f64(v, "pao2_fio2_ratio", 0.0, 800.0)?,
        platelets_10_9_l: need_positive_f64(v, "platelets_10_9_l", 0.0, 2000.0)?,
        bilirubin_mg_dl: need_f64(v, "bilirubin_mg_dl")?,
        map_mmhg: need_positive_f64(v, "map_mmhg", 0.0, 300.0)?,
        dopamine_dose: need_f64(v, "dopamine_dose")?,
        epinephrine_dose: need_f64(v, "epinephrine_dose")?,
        norepinephrine_dose: need_f64(v, "norepinephrine_dose")?,
        glasgow_coma_scale: gcs,
        creatinine_mg_dl: need_f64(v, "creatinine_mg_dl")?,
        urine_output_ml_d: need_f64(v, "urine_output_ml_d")?,
    };
    if input.bilirubin_mg_dl < 0.0
        || input.dopamine_dose < 0.0
        || input.epinephrine_dose < 0.0
        || input.norepinephrine_dose < 0.0
        || input.creatinine_mg_dl < 0.0
        || input.urine_output_ml_d < 0.0
    {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok(merge_result(
        json!({ "score": "sofa", "points": sofa_score(&input) }),
        provenance(
            "SOFA sequential organ failure assessment",
            "vincent-1996",
            "Vincent JL et al. Intensive Care Med 1996;22:707-10",
            "adult ICU; every organ-system input required",
            "PaO2/FiO2 mmHg; platelets 10^9/L; bilirubin mg/dL; MAP mmHg; vasopressors µg/kg/min; GCS 3–15; creatinine mg/dL; urine mL/day",
        ),
    ))
}

fn egfr(v: &Value) -> Result<Value, McpSystemError> {
    let input = RenalInput {
        age: need_age(v, 18, 120)?,
        sex_male: need_bool(v, "sex_male")?,
        // CKD-EPI 2021 creatinine equation does not use weight; this field is
        // struct padding only and is not a patient default in the result.
        weight_kg: 0.0,
        serum_creatinine: need_positive_f64(v, "serum_creatinine", 0.0, 30.0)?,
    };
    Ok(merge_result(
        json!({ "score": "egfr", "egfr_ml_min": ckd_epi_egfr(&input) }),
        provenance(
            "CKD-EPI 2021 eGFR (creatinine, without race)",
            "ckd-epi-2021",
            "Inker LA et al. NEJM 2021;385:1737-49",
            "adults 18–120 years; serum creatinine required",
            "age years; serum_creatinine mg/dL; eGFR mL/min/1.73m²",
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clinical_engine::framingham_10yr_risk;

    fn parse_ok(body: Value) -> Value {
        let out = clinical_risk(body.to_string().as_bytes()).expect("ok");
        serde_json::from_str(&out).expect("json")
    }

    fn parse_err(body: Value) {
        assert!(clinical_risk(body.to_string().as_bytes()).is_err());
    }

    fn framingham_complete() -> Value {
        json!({
            "score": "framingham",
            "age": 60,
            "sex_male": true,
            "total_cholesterol_mmol": 6.5,
            "hdl_cholesterol_mmol": 0.9,
            "systolic_bp": 162.0,
            "bp_treated": false,
            "current_smoker": true,
            "diabetic": true
        })
    }

    #[test]
    fn missing_score_cannot_calculate() {
        parse_err(json!({ "age": 55 }));
    }

    #[test]
    fn incomplete_framingham_cannot_calculate() {
        parse_err(json!({
            "score": "framingham",
            "age": 55,
            "input": { "sex_male": true, "systolic_bp": 140.0 }
        }));
    }

    #[test]
    fn omitted_boolean_is_not_false() {
        let mut body = framingham_complete();
        body.as_object_mut().unwrap().remove("diabetic");
        parse_err(body);
    }

    #[test]
    fn complete_framingham_matches_engine_and_names_algorithm() {
        let parsed = parse_ok(framingham_complete());
        let expected = framingham_10yr_risk(&FraminghamInput {
            age: 60,
            sex_male: true,
            total_cholesterol_mmol: 6.5,
            hdl_cholesterol_mmol: 0.9,
            systolic_bp: 162.0,
            bp_treated: false,
            current_smoker: true,
            diabetic: true,
        });
        assert!((parsed["risk_10yr"].as_f64().unwrap() - expected.risk_10yr).abs() < 1e-12);
        assert_eq!(parsed["algorithm"], "Framingham 10-year CVD risk");
        assert_eq!(parsed["not_diagnosis"], true);
    }

    #[test]
    fn score2_is_not_framingham_and_region_is_required() {
        parse_err(json!({
            "score": "score2",
            "age": 55,
            "sex_male": true,
            "systolic_bp": 140.0,
            "total_cholesterol_mmol": 5.5,
            "hdl_cholesterol_mmol": 1.1,
            "current_smoker": false
        }));
        let parsed = parse_ok(json!({
            "score": "score2",
            "age": 55,
            "sex_male": true,
            "systolic_bp": 140.0,
            "total_cholesterol_mmol": 5.5,
            "hdl_cholesterol_mmol": 1.1,
            "current_smoker": false,
            "risk_region": "high"
        }));
        assert_eq!(parsed["score"], "score2");
        assert_eq!(parsed["risk_region"], "High");
        assert_eq!(parsed["not_diagnosis"], true);
    }

    #[test]
    fn cha2ds2_requires_atrial_fibrillation() {
        parse_err(json!({
            "score": "cha2ds2_vasc",
            "age": 80,
            "atrial_fibrillation": false,
            "congestive_heart_failure": true,
            "hypertension": true,
            "diabetes": true,
            "stroke_tia_history": true,
            "vascular_disease": true,
            "sex_female": true
        }));
    }

    #[test]
    fn sofa_and_egfr_do_not_invent_organ_values() {
        parse_err(json!({ "score": "sofa" }));
        parse_err(json!({ "score": "egfr", "age": 55 }));
        let sofa = parse_ok(json!({
            "score": "sofa",
            "pao2_fio2_ratio": 400.0,
            "platelets_10_9_l": 180.0,
            "bilirubin_mg_dl": 0.8,
            "map_mmhg": 80.0,
            "dopamine_dose": 0.0,
            "epinephrine_dose": 0.0,
            "norepinephrine_dose": 0.0,
            "glasgow_coma_scale": 15,
            "creatinine_mg_dl": 0.9,
            "urine_output_ml_d": 1500.0
        }));
        assert_eq!(sofa["points"], 0);
        let renal = parse_ok(json!({
            "score": "egfr",
            "age": 60,
            "sex_male": true,
            "serum_creatinine": 1.0
        }));
        assert!(renal["egfr_ml_min"].as_f64().unwrap() > 0.0);
        assert_eq!(renal["not_diagnosis"], true);
    }
}
