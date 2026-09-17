//! Framingham 10-year CVD risk — native `clinical_engine` only.

use super::required;
use crate::clinical_engine::{framingham_10yr_risk, FraminghamInput};
use vibe::{Diagnostic, Span, Value};

pub const ALGORITHM: &str = "Framingham 10-year CVD risk";
pub const VERSION: &str = "wilson-1998-atp3";
pub const CITATION: &str = "Wilson PW et al. Circulation 1998;97:1837-47 (ATP III sex-specific)";
pub const APPLICABILITY: &str = "adults 30–74 years; lipid and blood-pressure inputs required";
pub const UNITS: &str = "age years; total_cholesterol_mmol mmol/L; hdl_cholesterol_mmol mmol/L; systolic_bp mmHg; risk_10yr fraction 0–1";

pub fn score(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let input = FraminghamInput {
        age: required::need_age_years(args_v, span, 30, 74, APPLICABILITY)?,
        sex_male: required::need_bool(args_v, span, "sex_male")?,
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
        systolic_bp: required::need_positive_f64(args_v, span, "systolic_bp", "mmHg", 0.0, 260.0)?,
        bp_treated: required::need_bool(args_v, span, "bp_treated")?,
        current_smoker: required::need_bool(args_v, span, "current_smoker")?,
        diabetic: required::need_bool(args_v, span, "diabetic")?,
    };
    if input.hdl_cholesterol_mmol >= input.total_cholesterol_mmol {
        return Err(super::super::args::bad(
            span,
            "hdl_cholesterol_mmol (mmol/L) must be lower than total_cholesterol_mmol",
        ));
    }
    let r = framingham_10yr_risk(&input);
    let mut pairs = vec![
        ("risk_10yr", Value::F64(r.risk_10yr)),
        ("log_score", Value::F64(r.log_score)),
        ("category", Value::String(format!("{:?}", r.category))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clinical_engine::FraminghamInput;
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

    fn high_risk_male() -> FraminghamInput {
        FraminghamInput {
            age: 60,
            sex_male: true,
            total_cholesterol_mmol: 6.5,
            hdl_cholesterol_mmol: 0.9,
            systolic_bp: 162.0,
            bp_treated: false,
            current_smoker: true,
            diabetic: true,
        }
    }

    fn args_from(input: &FraminghamInput) -> Value {
        rec(&[
            ("age", Value::U64(u64::from(input.age))),
            ("sex_male", Value::Bool(input.sex_male)),
            (
                "total_cholesterol_mmol",
                Value::F64(input.total_cholesterol_mmol),
            ),
            (
                "hdl_cholesterol_mmol",
                Value::F64(input.hdl_cholesterol_mmol),
            ),
            ("systolic_bp", Value::F64(input.systolic_bp)),
            ("bp_treated", Value::Bool(input.bp_treated)),
            ("current_smoker", Value::Bool(input.current_smoker)),
            ("diabetic", Value::Bool(input.diabetic)),
        ])
    }

    #[test]
    fn empty_record_cannot_calculate() {
        assert!(score(&Value::Record(BTreeMap::new()), span()).is_err());
    }

    #[test]
    fn age_outside_applicability_cannot_calculate() {
        let mut map = BTreeMap::new();
        if let Value::Record(source) = args_from(&high_risk_male()) {
            map = source;
        }
        map.insert("age".into(), Value::U64(20));
        assert!(score(&Value::Record(map), span()).is_err());
    }

    #[test]
    fn known_fixture_matches_engine_and_names_algorithm() {
        let input = high_risk_male();
        let expected = framingham_10yr_risk(&input);
        let Value::Record(result) = score(&args_from(&input), span()).unwrap() else {
            panic!("expected record");
        };
        match result.get("risk_10yr") {
            Some(Value::F64(value)) => assert!((value - expected.risk_10yr).abs() < 1e-12),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            result.get("algorithm"),
            Some(&Value::String(ALGORITHM.into()))
        );
        assert_eq!(result.get("version"), Some(&Value::String(VERSION.into())));
        assert_eq!(result.get("not_diagnosis"), Some(&Value::Bool(true)));
    }
}
