//! CHA₂DS₂-VASc stroke risk scoring for non-valvular atrial fibrillation.

use super::required;
use crate::clinical_engine::{cha2ds2_vasc_score, Cha2ds2VascInput};
use vibe::{Diagnostic, Span, Value};

pub const ALGORITHM: &str = "CHA₂DS₂-VASc stroke risk";
pub const VERSION: &str = "lip-2010-esc-2020";
pub const CITATION: &str = "Lip GY et al. Chest 2010; ESC 2020 atrial-fibrillation guidelines";
pub const APPLICABILITY: &str = "non-valvular atrial fibrillation in adults 18–120 years";
pub const UNITS: &str = "age years; score points; annual_stroke_risk_percent percent per year";

pub fn score(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    if !required::need_bool(args_v, span, "atrial_fibrillation")? {
        return Err(super::super::args::bad(
            span,
            "CHA₂DS₂-VASc applies only when atrial_fibrillation is true; inapplicable input cannot calculate",
        ));
    }
    let age = required::need_age_years(args_v, span, 18, 120, APPLICABILITY)?;
    let input = Cha2ds2VascInput {
        congestive_heart_failure: required::need_bool(args_v, span, "congestive_heart_failure")?,
        hypertension: required::need_bool(args_v, span, "hypertension")?,
        age_75_or_older: age >= 75,
        diabetes: required::need_bool(args_v, span, "diabetes")?,
        stroke_tia_history: required::need_bool(args_v, span, "stroke_tia_history")?,
        vascular_disease: required::need_bool(args_v, span, "vascular_disease")?,
        age_65_to_74: (65..75).contains(&age),
        sex_female: required::need_bool(args_v, span, "sex_female")?,
    };
    let r = cha2ds2_vasc_score(&input);
    let mut pairs = vec![
        ("score", Value::U64(u64::from(r.score))),
        (
            "annual_stroke_risk_percent",
            Value::F64(r.annual_stroke_risk_pct),
        ),
        (
            "anticoagulation_indicated",
            Value::Bool(r.anticoagulation_recommended),
        ),
        ("age_years", Value::U64(u64::from(age))),
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
    use crate::clinical_engine::Cha2ds2VascInput;
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

    fn max_score_args() -> Value {
        rec(&[
            ("age", Value::U64(80)),
            ("atrial_fibrillation", Value::Bool(true)),
            ("congestive_heart_failure", Value::Bool(true)),
            ("hypertension", Value::Bool(true)),
            ("diabetes", Value::Bool(true)),
            ("stroke_tia_history", Value::Bool(true)),
            ("vascular_disease", Value::Bool(true)),
            ("sex_female", Value::Bool(true)),
        ])
    }

    #[test]
    fn empty_record_cannot_calculate() {
        assert!(score(&Value::Record(BTreeMap::new()), span()).is_err());
    }

    #[test]
    fn missing_af_cannot_calculate() {
        let mut map = BTreeMap::new();
        if let Value::Record(source) = max_score_args() {
            map = source;
        }
        map.remove("atrial_fibrillation");
        assert!(score(&Value::Record(map), span()).is_err());
    }

    #[test]
    fn af_false_is_inapplicable() {
        let mut map = BTreeMap::new();
        if let Value::Record(source) = max_score_args() {
            map = source;
        }
        map.insert("atrial_fibrillation".into(), Value::Bool(false));
        assert!(score(&Value::Record(map), span()).is_err());
    }

    #[test]
    fn known_fixture_matches_engine_and_names_algorithm() {
        let expected = cha2ds2_vasc_score(&Cha2ds2VascInput {
            congestive_heart_failure: true,
            hypertension: true,
            age_75_or_older: true,
            diabetes: true,
            stroke_tia_history: true,
            vascular_disease: true,
            age_65_to_74: false,
            sex_female: true,
        });
        let Value::Record(result) = score(&max_score_args(), span()).unwrap() else {
            panic!("expected record");
        };
        assert_eq!(
            result.get("score"),
            Some(&Value::U64(u64::from(expected.score)))
        );
        assert_eq!(
            result.get("algorithm"),
            Some(&Value::String(ALGORITHM.into()))
        );
        assert_eq!(result.get("not_diagnosis"), Some(&Value::Bool(true)));
    }
}
