//! `solvers::linear_algebra::gemm::matmul` — future seam stays under `qualia-math`.

use super::super::args;
use crate::solvers::linear_algebra::gemm::matmul;
use poet_vibe::{Diagnostic, Span, Value};

pub fn multiply(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = matrix(args_v, "a", span)?;
    let b = matrix(args_v, "b", span)?;
    if a.cols != b.rows {
        return Err(args::bad(span, "matmul: a.cols must equal b.rows"));
    }
    let mut c = vec![0.0; a.rows * b.cols];
    matmul(a.rows, a.cols, b.cols, &a.data, &b.data, &mut c)
        .map_err(|_| args::bad(span, "matmul dimension error"))?;
    Ok(args::record([
        ("rows", Value::U64(a.rows as u64)),
        ("cols", Value::U64(b.cols as u64)),
        ("data", args::f64_list_value(c)),
    ]))
}

struct Mat {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

fn matrix(args_v: &Value, key: &str, span: Span) -> Result<Mat, Diagnostic> {
    let rec = args::rec(args_v, key).ok_or_else(|| args::bad(span, format!("matmul needs {key}")))?;
    let rows = args::rec_u64(rec, "rows")
        .or_else(|| infer_rows(rec))
        .ok_or_else(|| args::bad(span, format!("{key}.rows missing")))? as usize;
    let cols = args::rec_u64(rec, "cols")
        .or_else(|| infer_cols(rec))
        .ok_or_else(|| args::bad(span, format!("{key}.cols missing")))? as usize;
    let data = args::rec(rec, "data")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, format!("{key}.data needs a number list")))?;
    if data.len() != rows * cols {
        return Err(args::bad(span, format!("{key} data length != rows*cols")));
    }
    Ok(Mat { rows, cols, data })
}

fn infer_rows(rec: &Value) -> Option<u64> {
    match rec {
        Value::Record(m) => m.get("data").and_then(args::list).map(|xs| xs.len() as u64),
        _ => None,
    }
}

fn infer_cols(_rec: &Value) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn mat(rows: u64, cols: u64, data: Vec<f64>) -> Value {
        let mut m = BTreeMap::new();
        m.insert("rows".into(), Value::U64(rows));
        m.insert("cols".into(), Value::U64(cols));
        m.insert("data".into(), args::f64_list_value(data));
        Value::Record(m)
    }

    #[test]
    fn identity_2x2() {
        let mut args = BTreeMap::new();
        args.insert("a".into(), mat(2, 2, vec![1.0, 0.0, 0.0, 1.0]));
        args.insert("b".into(), mat(2, 2, vec![2.0, 3.0, 4.0, 5.0]));
        let v = multiply(&Value::Record(args), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("data") {
                Some(Value::List(xs)) => {
                    assert_eq!(xs[0], Value::F64(2.0));
                    assert_eq!(xs[3], Value::F64(5.0));
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
