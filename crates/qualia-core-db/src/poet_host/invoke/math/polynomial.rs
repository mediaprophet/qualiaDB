//! Polynomial roots — `solvers::polynomial::polynomial_roots`.

use super::super::args;
use crate::solvers::polynomial::polynomial_roots;
use vibe::{Diagnostic, Span, Value};

/// Find all complex roots of a real polynomial (DESCENDING coefficients,
/// `coeffs[0]·x^n + … + coeffs[n]`).
///
/// Input: record with `coeffs` (f64 list, descending order).
/// Output: record `{ degree: usize, roots: list of { re: f64, im: f64 } }`.
pub fn roots(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coeffs = args::rec(args_v, "coeffs")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "polynomial_roots needs coeffs"))?;
    let result = polynomial_roots(&coeffs).map_err(|_| {
        args::bad(
            span,
            "polynomial_roots: degenerate or non-finite coefficients",
        )
    })?;
    let degree = coeffs.len().saturating_sub(1);
    let roots_list = Value::List(
        result
            .iter()
            .map(|c| args::record([("re", Value::F64(c.re)), ("im", Value::F64(c.im))]))
            .collect(),
    );
    Ok(args::record([
        ("degree", Value::U64(degree as u64)),
        ("roots", roots_list),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn quadratic_two_real_roots() {
        // x² − 3x + 2 = 0 → roots 1, 2.
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![1.0, -3.0, 2.0]));
        let v = roots(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        assert_eq!(rec.get("degree"), Some(&Value::U64(2)));
        let list = match rec.get("roots") {
            Some(Value::List(l)) => l,
            other => panic!("expected roots list, got {other:?}"),
        };
        assert_eq!(list.len(), 2);
        // Collect real parts; both roots are real (im ≈ 0).
        let mut reals: Vec<f64> = list
            .iter()
            .map(|r| match r {
                Value::Record(m) => match m.get("re") {
                    Some(Value::F64(n)) => *n,
                    _ => panic!("missing re"),
                },
                _ => panic!("not a record"),
            })
            .collect();
        reals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((reals[0] - 1.0).abs() < 1e-6);
        assert!((reals[1] - 2.0).abs() < 1e-6);
        // Imaginary parts are zero.
        for r in list {
            if let Value::Record(m) = r {
                if let Some(Value::F64(im)) = m.get("im") {
                    assert!(im.abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn complex_conjugate_pair() {
        // x² + 1 = 0 → ±i.
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![1.0, 0.0, 1.0]));
        let v = roots(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        let list = match rec.get("roots") {
            Some(Value::List(l)) => l,
            other => panic!("expected roots list, got {other:?}"),
        };
        assert_eq!(list.len(), 2);
        let mut ims: Vec<f64> = list
            .iter()
            .map(|r| match r {
                Value::Record(m) => match m.get("im") {
                    Some(Value::F64(n)) => *n,
                    _ => panic!("missing im"),
                },
                _ => panic!("not a record"),
            })
            .collect();
        ims.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((ims[0] + 1.0).abs() < 1e-6);
        assert!((ims[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn missing_coeffs_errors() {
        let m = BTreeMap::new();
        assert!(roots(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn degenerate_all_zero_errors() {
        // All-zero polynomial → ComputationError.
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![0.0, 0.0, 0.0]));
        assert!(roots(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }
}
