//! Framingham 10-year CVD risk — native `clinical_engine` only.

use super::super::args;
use crate::clinical_engine::{framingham_10yr_risk, FraminghamInput};
use poet_vibe::{Diagnostic, Span, Value};

pub fn score(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let input = FraminghamInput {
        age: args::rec_u64(args_v, "age").unwrap_or(55) as u8,
        sex_male: args::rec_bool(args_v, "sex_male").unwrap_or(true),
        total_cholesterol_mmol: args::rec_f64(args_v, "total_cholesterol_mmol").unwrap_or(5.2),
        hdl_cholesterol_mmol: args::rec_f64(args_v, "hdl_cholesterol_mmol").unwrap_or(1.3),
        systolic_bp: args::rec_f64(args_v, "systolic_bp").unwrap_or(130.0),
        bp_treated: args::rec_bool(args_v, "bp_treated").unwrap_or(false),
        current_smoker: args::rec_bool(args_v, "current_smoker").unwrap_or(false),
        diabetic: args::rec_bool(args_v, "diabetic").unwrap_or(false),
    };
    let r = framingham_10yr_risk(&input);
    Ok(args::record([
        ("risk_10yr", Value::F64(r.risk_10yr)),
        ("log_score", Value::F64(r.log_score)),
        ("category", Value::String(format!("{:?}", r.category))),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn risk_is_unit_interval() {
        let v = score(&Value::Record(BTreeMap::new()), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("risk_10yr") {
                Some(Value::F64(p)) => assert!((0.0..=1.0).contains(p)),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
