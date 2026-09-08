//! App/REPL Linear Algebra Host primitives missing from the earlier freeze.
//!
//! `dot`, `norm`, `trace`, `identity`, `inverse` — thin codecs over slices and LU.

use super::super::args;
use crate::solvers::linear_algebra::lu::lu_decompose;
use vibe::{Diagnostic, Span, Value};

const IDENTITY_MAX: usize = 256;
const INVERSE_MAX: usize = 64;

struct Mat {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

fn matrix(args_v: &Value, key: &str, span: Span) -> Result<Mat, Diagnostic> {
    let rec = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("LinearAlgebra needs {key}")))?;
    let rows = args::rec_u64(rec, "rows")
        .ok_or_else(|| args::bad(span, format!("{key}.rows missing")))? as usize;
    let cols = args::rec_u64(rec, "cols")
        .ok_or_else(|| args::bad(span, format!("{key}.cols missing")))? as usize;
    let data = args::rec(rec, "data")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, format!("{key}.data needs a number list")))?;
    if data.len() != rows * cols {
        return Err(args::bad(span, format!("{key} data length != rows*cols")));
    }
    Ok(Mat { rows, cols, data })
}

fn mat_record(rows: usize, cols: usize, data: Vec<f64>) -> Value {
    args::record([
        ("rows", Value::U64(rows as u64)),
        ("cols", Value::U64(cols as u64)),
        ("data", args::f64_list_value(data)),
    ])
}

fn f64_list(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Vec<f64>, Diagnostic> {
    args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key} as a number list")))
}

/// `LinearAlgebra.dot` — `{ a, b }` lists of equal length. Out: `{ value }`.
pub fn la_dot(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = f64_list(args_v, "a", span, "LinearAlgebra.dot")?;
    let b = f64_list(args_v, "b", span, "LinearAlgebra.dot")?;
    if a.len() != b.len() || a.is_empty() {
        return Err(args::bad(span, "LinearAlgebra.dot needs equal non-empty lists"));
    }
    let value: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    if !value.is_finite() {
        return Err(args::bad(span, "LinearAlgebra.dot overflow"));
    }
    Ok(args::record([("value", Value::F64(value))]))
}

/// `LinearAlgebra.norm` — Euclidean L2 of `{ a }`. Out: `{ value }`.
pub fn la_norm(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = f64_list(args_v, "a", span, "LinearAlgebra.norm")?;
    if a.is_empty() {
        return Err(args::bad(span, "LinearAlgebra.norm needs a non-empty list"));
    }
    let value = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !value.is_finite() {
        return Err(args::bad(span, "LinearAlgebra.norm overflow"));
    }
    Ok(args::record([("value", Value::F64(value))]))
}

/// `LinearAlgebra.trace` — `{ a: { rows, cols, data } }` square. Out: `{ value }`.
pub fn la_trace(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "a", span)?;
    if m.rows != m.cols || m.rows == 0 {
        return Err(args::bad(span, "LinearAlgebra.trace needs a square matrix"));
    }
    let n = m.rows;
    let value: f64 = (0..n).map(|i| m.data[i * n + i]).sum();
    if !value.is_finite() {
        return Err(args::bad(span, "LinearAlgebra.trace overflow"));
    }
    Ok(args::record([("value", Value::F64(value))]))
}

/// `LinearAlgebra.identity` — `{ n }` with `1 <= n <= 256`. Out: `{ a }`.
pub fn la_identity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "LinearAlgebra.identity needs n"))? as usize;
    if n == 0 || n > IDENTITY_MAX {
        return Err(args::bad(
            span,
            format!("LinearAlgebra.identity n must be 1..={IDENTITY_MAX}"),
        ));
    }
    let mut data = vec![0.0; n * n];
    for i in 0..n {
        data[i * n + i] = 1.0;
    }
    Ok(args::record([("a", mat_record(n, n, data))]))
}

/// `LinearAlgebra.inverse` — LU + identity columns. Fail closed if singular.
/// Args: `{ a: { rows, cols, data } }`. Out: `{ a: { rows, cols, data } }`.
pub fn la_inverse(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "a", span)?;
    if m.rows != m.cols || m.rows == 0 {
        return Err(args::bad(span, "LinearAlgebra.inverse needs a square matrix"));
    }
    let n = m.rows;
    if n > INVERSE_MAX {
        return Err(args::bad(
            span,
            format!("LinearAlgebra.inverse n must be <= {INVERSE_MAX}"),
        ));
    }
    let lu = lu_decompose(n, &m.data).map_err(|e| args::bad(span, format!("inverse: {e:?}")))?;
    if lu.singular {
        return Err(args::bad(span, "LinearAlgebra.inverse: singular matrix"));
    }
    let mut inv = vec![0.0; n * n];
    for col in 0..n {
        let mut e = vec![0.0; n];
        e[col] = 1.0;
        let x = lu
            .solve(&e)
            .ok_or_else(|| args::bad(span, "LinearAlgebra.inverse: singular matrix"))?;
        for row in 0..n {
            inv[row * n + col] = x[row];
        }
    }
    Ok(args::record([("a", mat_record(n, n, inv))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poet_host::PoetSnapshot;
    use std::collections::BTreeMap;

    fn json_to_vibe(v: &serde_json::Value) -> Value {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::I64(i)
                } else if let Some(u) = n.as_u64() {
                    Value::U64(u)
                } else {
                    Value::F64(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(items) => {
                Value::List(items.iter().map(json_to_vibe).collect())
            }
            serde_json::Value::Object(map) => {
                let mut rec = BTreeMap::new();
                for (k, val) in map {
                    rec.insert(k.clone(), json_to_vibe(val));
                }
                Value::Record(rec)
            }
        }
    }

    fn invoke(id: &str, json: serde_json::Value) -> Value {
        let mut snap = PoetSnapshot::default();
        snap.invoke_id(id, json_to_vibe(&json)).unwrap()
    }

    #[test]
    fn wave40_dot_poet_json() {
        let out = invoke(
            "LinearAlgebra.dot",
            serde_json::json!({ "a": [1.0, 2.0, 3.0], "b": [4.0, 5.0, 6.0] }),
        );
        let v = args::rec_f64(&out, "value").unwrap();
        assert!((v - 32.0).abs() < 1e-12);
    }

    #[test]
    fn wave40_norm_poet_json() {
        let out = invoke(
            "LinearAlgebra.norm",
            serde_json::json!({ "a": [3.0, 4.0] }),
        );
        let v = args::rec_f64(&out, "value").unwrap();
        assert!((v - 5.0).abs() < 1e-12);
    }

    #[test]
    fn wave40_trace_identity() {
        let out = invoke(
            "LinearAlgebra.trace",
            serde_json::json!({
                "a": { "rows": 2, "cols": 2, "data": [1.0, 0.0, 0.0, 1.0] }
            }),
        );
        assert!((args::rec_f64(&out, "value").unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn wave40_identity_n3() {
        let out = invoke("LinearAlgebra.identity", serde_json::json!({ "n": 3 }));
        let a = args::rec(&out, "a").unwrap();
        let data = args::rec_f64_list(a, "data").unwrap();
        assert_eq!(
            data,
            vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn wave40_inverse_2x2() {
        let out = invoke(
            "LinearAlgebra.inverse",
            serde_json::json!({
                "a": { "rows": 2, "cols": 2, "data": [1.0, 2.0, 3.0, 4.0] }
            }),
        );
        let a = args::rec(&out, "a").unwrap();
        let data = args::rec_f64_list(a, "data").unwrap();
        // det = -2 → [[-2, 1], [1.5, -0.5]]
        assert!((data[0] + 2.0).abs() < 1e-9);
        assert!((data[1] - 1.0).abs() < 1e-9);
        assert!((data[2] - 1.5).abs() < 1e-9);
        assert!((data[3] + 0.5).abs() < 1e-9);
    }

    #[test]
    fn wave40_inverse_singular_fails() {
        let mut snap = PoetSnapshot::default();
        let err = snap
            .invoke_id(
                "LinearAlgebra.inverse",
                json_to_vibe(&serde_json::json!({
                    "a": { "rows": 2, "cols": 2, "data": [1.0, 2.0, 2.0, 4.0] }
                })),
            )
            .unwrap_err();
        assert!(err.message.contains("singular"));
    }
}
