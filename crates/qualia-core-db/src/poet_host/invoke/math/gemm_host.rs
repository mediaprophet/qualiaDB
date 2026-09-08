//! Host bind: BLAS-style `LinearAlgebra.gemm` through the engine solver.
//!
//! Routes to `solvers::linear_algebra::gemm::gemm` — CPU floor always, GPU when
//! `caps()` reports an accelerator and the work is large enough. Host does not
//! fork a second triple-loop or reject by size.

use super::super::args;
use crate::solvers::linear_algebra::gemm::{gemm, Transpose};
use vibe::{Diagnostic, Span, Value};

struct Mat {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

fn matrix(args_v: &Value, key: &str, span: Span) -> Result<Mat, Diagnostic> {
    let rec = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("LinearAlgebra.gemm needs {key}")))?;
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

fn parse_transpose(args_v: &Value, key: &str) -> Transpose {
    match args::rec_str(args_v, key).map(|s| s.to_ascii_lowercase()) {
        Some(s) if s == "yes" || s == "t" || s == "true" || s == "1" => Transpose::Yes,
        _ => Transpose::No,
    }
}

fn mat_record(rows: usize, cols: usize, data: Vec<f64>) -> Value {
    args::record([
        ("rows", Value::U64(rows as u64)),
        ("cols", Value::U64(cols as u64)),
        ("data", args::f64_list_value(data)),
    ])
}

/// `LinearAlgebra.gemm` — `C := alpha·op(A)·op(B) + beta·C` (row-major).
///
/// Args: `{ a, b, c?, alpha?, beta?, transa?, transb? }` where matrices are
/// `{ rows, cols, data }`. When `c` is omitted and `beta==0`, a zero buffer is used.
/// Out: `{ c: { rows, cols, data } }`.
pub fn gemm_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = matrix(args_v, "a", span)?;
    let b = matrix(args_v, "b", span)?;
    let transa = parse_transpose(args_v, "transa");
    let transb = parse_transpose(args_v, "transb");
    let alpha = args::rec_f64(args_v, "alpha").unwrap_or(1.0);
    let beta = args::rec_f64(args_v, "beta").unwrap_or(0.0);

    // BLAS dims: op(A) is m×k, op(B) is k×n, C is m×n.
    let (m, k_a) = match transa {
        Transpose::No => (a.rows, a.cols),
        Transpose::Yes => (a.cols, a.rows),
    };
    let (k_b, n) = match transb {
        Transpose::No => (b.rows, b.cols),
        Transpose::Yes => (b.cols, b.rows),
    };
    if k_a != k_b {
        return Err(args::bad(
            span,
            "gemm: inner dimensions of op(A) and op(B) disagree",
        ));
    }
    let k = k_a;

    let mut c = if let Ok(existing) = matrix(args_v, "c", span) {
        if existing.rows != m || existing.cols != n {
            return Err(args::bad(span, "gemm: c shape must be m×n"));
        }
        existing.data
    } else if beta == 0.0 {
        vec![0.0; m * n]
    } else {
        return Err(args::bad(span, "gemm: c required when beta != 0"));
    };

    gemm(
        transa, transb, m, n, k, alpha, &a.data, &b.data, beta, &mut c,
    )
    .map_err(|e| args::bad(span, format!("gemm: {e:?}")))?;

    Ok(args::record([("c", mat_record(m, n, c))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poet_host::PoetSnapshot;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

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

    fn identity_data(n: usize) -> Vec<f64> {
        let mut d = vec![0.0; n * n];
        for i in 0..n {
            d[i * n + i] = 1.0;
        }
        d
    }

    #[test]
    fn wave13_gemm_2x2_identity() {
        let mut args = BTreeMap::new();
        args.insert(
            "a".into(),
            json_to_vibe(&serde_json::json!({"rows": 2, "cols": 2, "data": [1.0, 2.0, 3.0, 4.0]})),
        );
        args.insert(
            "b".into(),
            json_to_vibe(&serde_json::json!({"rows": 2, "cols": 2, "data": [1.0, 0.0, 0.0, 1.0]})),
        );
        args.insert("alpha".into(), Value::F64(1.0));
        args.insert("beta".into(), Value::F64(0.0));
        let out = gemm_host(&Value::Record(args), span()).unwrap();
        let c = args::rec(&out, "c").unwrap();
        let data = args::rec_f64_list(c, "data").unwrap();
        assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn wave40_gemm_32x32_identity_succeeds() {
        let side = 32usize;
        let data = identity_data(side);
        let mut args = BTreeMap::new();
        args.insert(
            "a".into(),
            json_to_vibe(&serde_json::json!({
                "rows": side as u64,
                "cols": side as u64,
                "data": data,
            })),
        );
        let data_b = identity_data(side);
        args.insert(
            "b".into(),
            json_to_vibe(&serde_json::json!({
                "rows": side as u64,
                "cols": side as u64,
                "data": data_b,
            })),
        );
        let out = gemm_host(&Value::Record(args), span()).unwrap();
        let c = args::rec(&out, "c").unwrap();
        let got = args::rec_f64_list(c, "data").unwrap();
        assert_eq!(got, identity_data(side));
    }

    #[test]
    fn wave40_gemm_poet_json_1x1() {
        // Same JSON shape `scientific:gemm_live` / Tool Chest send.
        let json = serde_json::json!({
            "a": { "rows": 1, "cols": 1, "data": [2.0] },
            "b": { "rows": 1, "cols": 1, "data": [3.0] },
            "alpha": 1.0,
            "beta": 0.0
        });
        let mut snap = PoetSnapshot::default();
        let out = snap
            .invoke_id("LinearAlgebra.gemm", json_to_vibe(&json))
            .unwrap();
        let c = args::rec(&out, "c").unwrap();
        let data = args::rec_f64_list(c, "data").unwrap();
        assert!((data[0] - 6.0).abs() < 1e-12);
    }
}
