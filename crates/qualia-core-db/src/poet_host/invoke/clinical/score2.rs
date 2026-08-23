//! SCORE2 10-year fatal and non-fatal cardiovascular disease risk.

use super::super::args;
use crate::clinical_engine::{score2_risk, Score2Input, Score2Region};
use vibe::{Diagnostic, Span, Value};

pub fn score(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let input = Score2Input {
        age: args::rec_u64(args_v, "age").unwrap_or(50) as u8,
        sex_male: args::rec_bool(args_v, "sex_male").unwrap_or(true),
        systolic_bp: args::rec_f64(args_v, "systolic_bp").unwrap_or(135.0),
        total_cholesterol_mmol: args::rec_f64(args_v, "total_cholesterol_mmol").unwrap_or(5.0),
        hdl_cholesterol_mmol: args::rec_f64(args_v, "hdl_cholesterol_mmol").unwrap_or(1.2),
        current_smoker: args::rec_bool(args_v, "current_smoker").unwrap_or(false),
        risk_region: Score2Region::Moderate,
    };
    let r = score2_risk(&input);
    Ok(args::record([
        ("risk_percent", Value::F64(r.risk_10yr_pct)),
        ("category", Value::String(format!("{:?}", r.category))),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_score2_default() {
        let v = score(&Value::Record(BTreeMap::new()), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("risk_percent"));
                assert!(r.contains_key("category"));
            }
            other => panic!("{other:?}"),
        }
    }
}
