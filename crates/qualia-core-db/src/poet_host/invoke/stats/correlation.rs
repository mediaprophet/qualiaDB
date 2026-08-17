//! Pearson correlation — `solvers::statistics::correlation`.

use super::super::args;
use crate::solvers::statistics::correlation::pearson;
use poet_vibe::{Diagnostic, Span, Value};

pub fn pearson_r(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec(args_v, "x")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "pearson needs x"))?;
    let y = args::rec(args_v, "y")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "pearson needs y"))?;
    pearson(&x, &y)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "pearson needs two series of length ≥ 2"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn perfect_positive() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        m.insert("y".into(), args::f64_list_value(vec![2.0, 4.0, 6.0]));
        match pearson_r(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::F64(r) => assert!((r - 1.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
    }
}
