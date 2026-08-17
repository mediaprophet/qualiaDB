//! Constant-acceleration kinematics — `MechanicalAnalyzer`.

use super::super::args;
use crate::specialized_libs::engineering_analysis::MechanicalAnalyzer;
use poet_vibe::{Diagnostic, Span, Value};

pub fn run(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x0 = args::rec_f64(args_v, "x0").unwrap_or(0.0);
    let v0 = args::rec_f64(args_v, "v0").unwrap_or(0.0);
    let a = args::rec_f64(args_v, "a").unwrap_or(0.0);
    let times = args::rec(args_v, "t")
        .and_then(args::f64s)
        .unwrap_or_else(|| vec![0.0, 1.0]);
    let mut ma = MechanicalAnalyzer::new();
    let r = ma
        .analyze_kinematics(x0, v0, a, &times)
        .map_err(|e| args::bad(span, format!("kinematics: {e:?}")))?;
    Ok(args::record([
        ("positions", args::f64_list_value(r.positions)),
        ("velocities", args::f64_list_value(r.velocities)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn rest_stays_put() {
        let mut m = BTreeMap::new();
        m.insert("x0".into(), Value::F64(1.0));
        m.insert("t".into(), args::f64_list_value(vec![0.0, 1.0]));
        match run(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("positions") {
                Some(Value::List(xs)) => assert_eq!(xs[1], Value::F64(1.0)),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
