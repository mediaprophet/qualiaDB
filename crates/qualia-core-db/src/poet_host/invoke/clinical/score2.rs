//! SCORE2 10-year fatal and non-fatal cardiovascular disease risk.

use super::required;
use crate::clinical_engine::{score2_risk, Score2Input, Score2Region};
use vibe::{Diagnostic, Span, Value};

pub const ALGORITHM: &str = "SCORE2 10-year CVD risk";
pub const VERSION: &str = "score2-2021";
pub const CITATION: &str = "SCORE2 working group / ESC CVD Risk Collaboration 2021";
pub const APPLICABILITY: &str = "adults 40–69 years; European risk region required (not defaulted)";
pub const UNITS: &str = "age years; total_cholesterol_mmol mmol/L; hdl_cholesterol_mmol mmol/L; systolic_bp mmHg; risk_percent percent over 10 years";

pub fn score(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = Score2Input {
        age: required::need_age_years(args_v, span, 40, 69, APPLICABILITY)?,
        sex_male: required::need_bool(args_v, span, "sex_male")?,
        systolic_bp: required::need_positive_f64(args_v, span, "systolic_bp", "mmHg", 0.0, 260.0)?,
        total_cholesterol_mmol: required::need_positive_f64(
            args_v,
            span,
            "total_cholesterol_mmol",
            "mmol/L",
            0.0,
            20.0,
        )?,
        hdl_cholesterol_mmol: required::need_positive_f64(
            args_v,
            span,
            "hdl_cholesterol_mmol",
            "mmol/L",
            0.0,
            5.0,
        )?,
        current_smoker: required::need_bool(args_v, span, "current_smoker")?,
        risk_region: parse_region(args_v, span)?,
    };
    if input.hdl_cholesterol_mmol >= input.total_cholesterol_mmol {
        return Err(super::super::args::bad(
            span,
            "hdl_cholesterol_mmol (mmol/L) must be lower than total_cholesterol_mmol",
        ));
    }
    let r = score2_risk(&input);
    let mut pairs = vec![
        ("risk_percent", Value::F64(r.risk_10yr_pct)),
        ("category", Value::String(format!("{:?}", r.category))),
        (
            "risk_region",
            Value::String(format!("{:?}", input.risk_region)),
        ),
    ];
    pairs.extend(required::provenance(
        ALGORITHM,
        VERSION,
        CITATION,
        APPLICABILITY,
        UNITS,
    ));
    Ok(super::super::args::record(pairs))
}

fn parse_region(args_v: &Value, span: Span) -> Result<Score2Region, Diagnostic> {
    let raw = required::need_str(args_v, span, "risk_region")?;
    match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "low" => Ok(Score2Region::Low),
        "moderate" => Ok(Score2Region::Moderate),
        "high" => Ok(Score2Region::High),
        "very_high" | "veryhigh" => Ok(Score2Region::VeryHigh),
        other => Err(super::super::args::bad(
            span,
            format!("risk_region `{other}` is unknown; use low, moderate, high, or very_high"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clinical_engine::Score2Input;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut map = BTreeMap::new();
        for (key, value) in pairs {
            map.insert((*key).into(), value.clone());
        }
        Value::Record(map)
    }

    fn fixture() -> Score2Input {
        Score2Input {
            age: 55,
            sex_male: true,
            systolic_bp: 140.0,
            total_cholesterol_mmol: 5.5,
            hdl_cholesterol_mmol: 1.1,
            current_smoker: false,
            risk_region: Score2Region::High,
        }
    }

    fn args_from(input: &Score2Input) -> Value {
        rec(&[
            ("age", Value::U64(u64::from(input.age))),
            ("sex_male", Value::Bool(input.sex_male)),
            ("systolic_bp", Value::F64(input.systolic_bp)),
            (
                "total_cholesterol_mmol",
                Value::F64(input.total_cholesterol_mmol),
            ),
            (
                "hdl_cholesterol_mmol",
                Value::F64(input.hdl_cholesterol_mmol),
            ),
            ("current_smoker", Value::Bool(input.current_smoker)),
            ("risk_region", Value::String("high".into())),
        ])
    }

    #[test]
    fn empty_record_cannot_calculate() {
        assert!(score(&Value::Record(BTreeMap::new()), span()).is_err());
    }

    #[test]
    fn missing_region_is_not_moderate() {
        let mut map = BTreeMap::new();
        if let Value::Record(source) = args_from(&fixture()) {
            map = source;
        }
        map.remove("risk_region");
        assert!(score(&Value::Record(map), span()).is_err());
    }

    #[test]
    fn age_outside_applicability_cannot_calculate() {
        let mut map = BTreeMap::new();
        if let Value::Record(source) = args_from(&fixture()) {
            map = source;
        }
        map.insert("age".into(), Value::U64(75));
        assert!(score(&Value::Record(map), span()).is_err());
    }

    #[test]
    fn known_fixture_matches_engine_and_names_algorithm() {
        let input = fixture();
        let expected = score2_risk(&input);
        let Value::Record(result) = score(&args_from(&input), span()).unwrap() else {
            panic!("expected record");
        };
        match result.get("risk_percent") {
            Some(Value::F64(value)) => assert!((value - expected.risk_10yr_pct).abs() < 1e-12),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            result.get("algorithm"),
            Some(&Value::String(ALGORITHM.into()))
        );
        assert_eq!(result.get("not_diagnosis"), Some(&Value::Bool(true)));
    }
}
