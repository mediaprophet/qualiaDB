//! Simple OLS linear regression — `solvers::statistics::regression`.

use super::super::args;
use crate::solvers::statistics::regression::simple_linear_regression;
use poet_vibe::{Diagnostic, Span, Value};

/// Ordinary-least-squares fit of `y = intercept + slope·x`.
///
/// Input: record with `x` (f64 list) and `y` (f64 list).
/// Output: record with `slope`, `intercept`, `r_squared`,
/// `residual_std_error`, and `n`.
pub fn linear_regression(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec(args_v, "x")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "linear_regression needs x"))?;
    let y = args::rec(args_v, "y")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "linear_regression needs y"))?;
    if x.len() != y.len() {
        return Err(args::bad(span, "linear_regression: x and y length mismatch"));
    }
    let fit = simple_linear_regression(&x, &y)
        .ok_or_else(|| args::bad(span, "linear_regression: degenerate input (n < 3 or zero variance)"))?;
    Ok(args::record([
        ("slope", Value::F64(fit.slope)),
        ("intercept", Value::F64(fit.intercept)),
        ("r_squared", Value::F64(fit.r_squared)),
        ("residual_std_error", Value::F64(fit.residual_std_error)),
        ("n", Value::U64(fit.n as u64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn exact_line_recovered() {
        // y = 3 + 2x exactly.
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value(vec![0.0, 1.0, 2.0, 3.0, 4.0]));
        m.insert("y".into(), args::f64_list_value(vec![3.0, 5.0, 7.0, 9.0, 11.0]));
        let v = linear_regression(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        let slope = match rec.get("slope") {
            Some(Value::F64(n)) => *n,
            other => panic!("expected f64 slope, got {other:?}"),
        };
        let intercept = match rec.get("intercept") {
            Some(Value::F64(n)) => *n,
            other => panic!("expected f64 intercept, got {other:?}"),
        };
        let r2 = match rec.get("r_squared") {
            Some(Value::F64(n)) => *n,
            other => panic!("expected f64 r_squared, got {other:?}"),
        };
        let n = match rec.get("n") {
            Some(Value::U64(n)) => *n,
            other => panic!("expected u64 n, got {other:?}"),
        };
        assert!((slope - 2.0).abs() < 1e-9);
        assert!((intercept - 3.0).abs() < 1e-9);
        assert!((r2 - 1.0).abs() < 1e-9);
        assert_eq!(n, 5);
    }

    #[test]
    fn length_mismatch_errors() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        m.insert("y".into(), args::f64_list_value(vec![1.0, 2.0]));
        assert!(linear_regression(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn missing_x_errors() {
        let mut m = BTreeMap::new();
        m.insert("y".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        assert!(linear_regression(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn degenerate_zero_variance_errors() {
        // x has zero variance → slope undefined.
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value(vec![2.0, 2.0, 2.0]));
        m.insert("y".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        assert!(linear_regression(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }
}
