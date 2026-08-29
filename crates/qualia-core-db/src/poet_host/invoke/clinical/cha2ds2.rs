//! CHA2DS2-VASc stroke risk scoring for atrial fibrillation.

use super::super::args;
use crate::clinical_engine::{cha2ds2_vasc_score, Cha2ds2VascInput};
use vibe::{Diagnostic, Span, Value};

pub fn score(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let age = args::rec_u64(args_v, "age").unwrap_or(65) as u8;
    let input = Cha2ds2VascInput {
        congestive_heart_failure: args::rec_bool(args_v, "congestive_heart_failure")
            .unwrap_or(false),
        hypertension: args::rec_bool(args_v, "hypertension").unwrap_or(false),
        age_75_or_older: age >= 75 || args::rec_bool(args_v, "age_75_or_older").unwrap_or(false),
        diabetes: args::rec_bool(args_v, "diabetes").unwrap_or(false),
        stroke_tia_history: args::rec_bool(args_v, "stroke_tia_history").unwrap_or(false),
        vascular_disease: args::rec_bool(args_v, "vascular_disease").unwrap_or(false),
        age_65_to_74: (65..75).contains(&age)
            || args::rec_bool(args_v, "age_65_to_74").unwrap_or(false),
        sex_female: args::rec_bool(args_v, "sex_female").unwrap_or(false),
    };
    let r = cha2ds2_vasc_score(&input);
    Ok(args::record([
        ("score", Value::U64(r.score as u64)),
        (
            "annual_stroke_risk_percent",
            Value::F64(r.annual_stroke_risk_pct),
        ),
        (
            "anticoagulation_indicated",
            Value::Bool(r.anticoagulation_recommended),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_cha2ds2_default() {
        let v = score(&Value::Record(BTreeMap::new()), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("score"));
                assert!(r.contains_key("annual_stroke_risk_percent"));
            }
            other => panic!("{other:?}"),
        }
    }
}
