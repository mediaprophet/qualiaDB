//! OLS via `solvers::learning::regression` — future seam `qualia-ml`.

use super::super::args;
use crate::solvers::learning::regression::linear::fit;
use poet_vibe::{Diagnostic, Span, Value};

pub fn fit_ols(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rows = args::rec(args_v, "x")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "ols needs x: [[...], ...]"))?;
    let y = args::rec(args_v, "y")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "ols needs y"))?;
    let n = rows.len();
    let p = rows
        .first()
        .and_then(args::f64s)
        .map(|r| r.len())
        .ok_or_else(|| args::bad(span, "ols x rows must be number lists"))?;
    let mut x = Vec::with_capacity(n * p);
    for row in rows {
        let cells = args::f64s(row).ok_or_else(|| args::bad(span, "ols x row is not numbers"))?;
        if cells.len() != p {
            return Err(args::bad(span, "ols x rows must share a width"));
        }
        x.extend(cells);
    }
    let intercept = args::rec_bool(args_v, "intercept").unwrap_or(true);
    let model = fit(&x, &y, n, p, intercept).map_err(|e| args::bad(span, format!("ols: {e:?}")))?;
    Ok(args::record([
        ("n", Value::U64(model.n as u64)),
        ("r_squared", Value::F64(model.r_squared)),
        ("coefficients", args::f64_list_value(model.coefficients)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn fits_line() {
        let mut m = BTreeMap::new();
        m.insert(
            "x".into(),
            Value::List(vec![
                args::f64_list_value(vec![1.0]),
                args::f64_list_value(vec![2.0]),
                args::f64_list_value(vec![3.0]),
                args::f64_list_value(vec![4.0]),
            ]),
        );
        m.insert("y".into(), args::f64_list_value(vec![3.0, 5.0, 7.0, 9.0]));
        let v = fit_ols(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("coefficients") {
                Some(Value::List(cs)) if cs.len() == 2 => match (&cs[0], &cs[1]) {
                    (Value::F64(b0), Value::F64(b1)) => {
                        assert!((b0 - 1.0).abs() < 1e-6);
                        assert!((b1 - 2.0).abs() < 1e-6);
                    }
                    other => panic!("{other:?}"),
                },
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
