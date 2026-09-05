//! WASM playground clinical-risk JSON. Missing fields fail closed.

use serde_json::{json, Value};

const NOT_ADVICE: &str =
    "This number is not a diagnosis, treatment recommendation, or clinical advice.";

pub fn evaluate(calculator: &str, params: &Value) -> Result<Value, String> {
    match calculator {
        "framingham" => framingham(params),
        "chas2" | "cha2ds2" | "cha2ds2_vasc" => cha2ds2(params),
        "score2" => score2(params),
        other => Err(format!("unknown clinical calculator: {other}")),
    }
}

fn need_bool(v: &Value, key: &str) -> Result<bool, String> {
    v.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{key} must be true or false; omitting it is not a patient value"))
}

fn need_bool_alias(v: &Value, key: &str, alias: &str) -> Result<bool, String> {
    if v.get(key).is_some() {
        return need_bool(v, key);
    }
    if v.get(alias).is_some() {
        return need_bool(v, alias);
    }
    Err(format!(
        "{key} must be true or false; omitting it is not a patient value"
    ))
}

fn need_f64(v: &Value, key: &str) -> Result<f64, String> {
    let value = v
        .get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_u64().map(|n| n as f64)))
        .ok_or_else(|| format!("{key} is required; missing is not a patient value"))?;
    if !value.is_finite() {
        return Err(format!("{key} must be a finite number"));
    }
    Ok(value)
}

fn need_u8(v: &Value, key: &str) -> Result<u8, String> {
    let value = v
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} is required; missing is not a patient value"))?;
    u8::try_from(value).map_err(|_| format!("{key} is out of range"))
}

fn need_age(
    v: &Value,
    min_inclusive: u8,
    max_inclusive: u8,
    applicability: &str,
) -> Result<u8, String> {
    let age = need_u8(v, "age")?;
    if age < min_inclusive || age > max_inclusive {
        return Err(format!(
            "age {age} years is outside this algorithm's applicability ({min_inclusive}–{max_inclusive}; {applicability})"
        ));
    }
    Ok(age)
}

fn need_positive(v: &Value, key: &str, max_inclusive: f64) -> Result<f64, String> {
    let value = need_f64(v, key)?;
    if value <= 0.0 || value > max_inclusive {
        return Err(format!("{key} must be in (0, {max_inclusive}]"));
    }
    Ok(value)
}

fn provenance(algorithm: &str, version: &str) -> Value {
    json!({
        "algorithm": algorithm,
        "version": version,
        "not_diagnosis": true,
        "not_advice": NOT_ADVICE,
    })
}

fn merge(mut body: Value, meta: Value) -> Value {
    if let (Some(target), Some(extra)) = (body.as_object_mut(), meta.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    body
}

fn mmol_or_mgdl(v: &Value, mmol_key: &str, mgdl_key: &str, max_mmol: f64) -> Result<f64, String> {
    if v.get(mmol_key).is_some() {
        return need_positive(v, mmol_key, max_mmol);
    }
    if v.get(mgdl_key).is_some() {
        return Ok(need_positive(v, mgdl_key, max_mmol * 38.67)? / 38.67);
    }
    Err(format!(
        "{mmol_key} (mmol/L) or {mgdl_key} (mg/dL) is required; missing is not a patient value"
    ))
}

fn framingham(v: &Value) -> Result<Value, String> {
    let age = need_age(v, 30, 74, "adults 30–74 years")?;
    let sex_male = need_bool_alias(v, "sex_male", "male")?;
    let systolic_bp = need_positive(v, "systolic_bp", 260.0)?;
    let total_cholesterol_mmol = mmol_or_mgdl(v, "total_cholesterol_mmol", "total_chol", 20.0)?;
    let hdl_cholesterol_mmol = mmol_or_mgdl(v, "hdl_cholesterol_mmol", "hdl_chol", 5.0)?;
    if hdl_cholesterol_mmol >= total_cholesterol_mmol {
        return Err(
            "hdl cholesterol must be lower than total cholesterol; inverted lipids cannot calculate"
                .into(),
        );
    }
    let current_smoker = need_bool_alias(v, "current_smoker", "smoker")?;
    let diabetic = need_bool(v, "diabetic")?;
    let bp_treated = need_bool_alias(v, "bp_treated", "hypertension_treated")?;

    let ln_age = (age as f64).max(1.0).ln();
    let ln_tc = (total_cholesterol_mmol * 38.67).max(1.0).ln();
    let ln_hdl = (hdl_cholesterol_mmol * 38.67).max(1.0).ln();
    let ln_sbp = systolic_bp.max(1.0).ln();

    let (sum, mean, s0) = if sex_male {
        let mut s = 3.06117 * ln_age + 1.12370 * ln_tc - 0.93263 * ln_hdl
            + (if bp_treated { 1.99881 } else { 1.93303 }) * ln_sbp;
        if current_smoker {
            s += 0.65451;
        }
        if diabetic {
            s += 0.57367;
        }
        (s, 23.9802_f64, 0.88936_f64)
    } else {
        let mut s = 2.32888 * ln_age + 1.20904 * ln_tc - 0.70833 * ln_hdl
            + (if bp_treated { 2.82263 } else { 2.76157 }) * ln_sbp;
        if current_smoker {
            s += 0.52873;
        }
        if diabetic {
            s += 0.69154;
        }
        (s, 26.1931_f64, 0.95012_f64)
    };

    let risk = ((1.0 - s0.powf((sum - mean).exp())) * 100.0).clamp(0.0, 100.0);
    let category = if risk < 10.0 {
        "Low"
    } else if risk < 20.0 {
        "Intermediate"
    } else {
        "High"
    };

    Ok(merge(
        json!({
            "calculator": "framingham",
            "risk_10yr_pct": risk,
            "category": category,
        }),
        provenance(
            "Framingham 10-year CVD risk (D'Agostino 2008 Cox, mg/dL lipids)",
            "dagostino-2008",
        ),
    ))
}

fn cha2ds2(v: &Value) -> Result<Value, String> {
    if !need_bool(v, "atrial_fibrillation")? {
        return Err(
            "CHA₂DS₂-VASc applies only when atrial_fibrillation is true; inapplicable input cannot calculate"
                .into(),
        );
    }
    let age = need_age(v, 18, 120, "non-valvular atrial fibrillation in adults")?;
    let congestive_heart_failure = need_bool_alias(v, "congestive_heart_failure", "chf")?;
    let hypertension = need_bool(v, "hypertension")?;
    let diabetes = need_bool(v, "diabetes")?;
    let stroke_tia_history = need_bool_alias(v, "stroke_tia_history", "stroke")?;
    let vascular_disease = need_bool(v, "vascular_disease")?;
    let sex_female = need_bool_alias(v, "sex_female", "female")?;
    let age_75_or_older = age >= 75;
    let age_65_to_74 = (65..75).contains(&age);

    let score = congestive_heart_failure as u8
        + hypertension as u8
        + if age_75_or_older { 2 } else { 0 }
        + diabetes as u8
        + if stroke_tia_history { 2 } else { 0 }
        + vascular_disease as u8
        + if age_65_to_74 { 1 } else { 0 }
        + sex_female as u8;

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
    let anticoagulation_recommended = if sex_female { score >= 3 } else { score >= 2 };

    Ok(merge(
        json!({
            "calculator": "cha2ds2_vasc",
            "score": score,
            "annual_stroke_risk_pct": annual_risk,
            "anticoagulation_recommended": anticoagulation_recommended,
        }),
        provenance("CHA₂DS₂-VASc stroke risk", "lip-2010-esc-2020"),
    ))
}

fn score2(v: &Value) -> Result<Value, String> {
    let age = need_age(v, 40, 69, "adults 40–69 years")?;
    let sex_male = need_bool_alias(v, "sex_male", "male")?;
    let systolic_bp = need_positive(v, "systolic_bp", 260.0)?;
    let total_chol = if v.get("total_cholesterol_mmol").is_some() {
        need_positive(v, "total_cholesterol_mmol", 20.0)?
    } else {
        need_positive(v, "total_chol", 20.0)?
    };
    let hdl_chol = if v.get("hdl_cholesterol_mmol").is_some() {
        need_positive(v, "hdl_cholesterol_mmol", 5.0)?
    } else {
        need_positive(v, "hdl_chol", 5.0)?
    };
    if hdl_chol >= total_chol {
        return Err(
            "hdl cholesterol must be lower than total cholesterol; inverted lipids cannot calculate"
                .into(),
        );
    }
    let current_smoker = need_bool_alias(v, "current_smoker", "smoker")?;
    let region_mult = match need_str(v, "risk_region")?
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "low" => 0.71,
        "moderate" => 1.00,
        "high" => 1.56,
        "very_high" | "veryhigh" => 2.27,
        _ => {
            return Err(
                "risk_region is unknown; use low, moderate, high, or very_high (not defaulted)"
                    .into(),
            )
        }
    };

    let non_hdl = total_chol - hdl_chol;
    let age_c = (age as f64 - 60.0) / 5.0;
    let sbp_c = (systolic_bp - 120.0) / 20.0;
    let chol_c = (non_hdl - 3.3) / 0.5;
    let smoke = current_smoker as u8 as f64;

    let (b_age, b_sbp, b_chol, b_smoke, baseline_surv) = if sex_male {
        (0.3742_f64, 0.2628, 0.1401, 0.5865, 0.9605_f64)
    } else {
        (0.4648_f64, 0.3131, 0.1002, 0.7742, 0.9776_f64)
    };

    let linear = b_age * age_c + b_sbp * sbp_c + b_chol * chol_c + b_smoke * smoke;
    let base_risk = 1.0 - baseline_surv.powf(linear.exp());
    let calibrated_pct = (base_risk * region_mult * 100.0).clamp(0.0, 100.0);
    let category = if calibrated_pct < 5.0 {
        "Low"
    } else if calibrated_pct <= 10.0 {
        "Moderate"
    } else if calibrated_pct <= 20.0 {
        "High"
    } else {
        "VeryHigh"
    };

    Ok(merge(
        json!({
            "calculator": "score2",
            "risk_10yr_pct": calibrated_pct,
            "category": category,
        }),
        provenance("SCORE2 10-year CVD risk", "score2-2021"),
    ))
}

fn need_str<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    let value = v
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} is required; missing is not a default"))?;
    if value.trim().is_empty() {
        return Err(format!("{key} is required; empty is not a default"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_playground_params_cannot_calculate() {
        let v = json!({"age": 65, "systolic_bp": 140, "smoker": true});
        assert!(evaluate("framingham", &v).is_err());
        assert!(evaluate("chas2", &v).is_err());
        assert!(evaluate("score2", &v).is_err());
    }

    #[test]
    fn complete_framingham_returns_provenance() {
        let out = evaluate(
            "framingham",
            &json!({
                "age": 55,
                "sex_male": true,
                "systolic_bp": 120.0,
                "total_chol": 200.0,
                "hdl_chol": 50.0,
                "smoker": false,
                "diabetic": false,
                "bp_treated": false
            }),
        )
        .expect("ok");
        assert_eq!(out["calculator"], "framingham");
        assert_eq!(out["not_diagnosis"], true);
        assert!(out["risk_10yr_pct"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn score2_region_is_required() {
        let mut params = json!({
            "age": 60,
            "sex_male": true,
            "systolic_bp": 140.0,
            "total_chol": 5.0,
            "hdl_chol": 1.3,
            "smoker": true
        });
        assert!(evaluate("score2", &params).is_err());
        params
            .as_object_mut()
            .unwrap()
            .insert("risk_region".into(), json!("high"));
        let out = evaluate("score2", &params).expect("ok");
        assert_eq!(out["calculator"], "score2");
    }
}
