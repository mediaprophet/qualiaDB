//! Wave-13 Host bind: BLAS-style `gemm` on a **pure CPU** path.
//!
//! Host-missing `LinearAlgebra.gemm`. Intentionally does **not** call
//! `solvers::linear_algebra::gemm::gemm` — that entry probes forge/`caps()` and can
//! panic loading CUDA on machines without the driver. This module keeps a local
//! triple-loop (same indexing as the solver CPU floor) and rejects oversized work
//! so the Host never offloads.

use super::super::args;
use vibe::{Diagnostic, Span, Value};

/// Matches `wgsl_forge::dispatch::GEMM_GPU_THRESHOLD` — Host stays strictly below.
const HOST_GEMM_CPU_CAP: usize = 1 << 15;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Transpose {
    No,
    Yes,
}

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

/// Pure CPU `C := alpha·op(A)·op(B) + beta·C` (row-major). Same index convention as
/// the solver CPU floor — Host never touches forge/`caps()`.
fn gemm_cpu(
    transa: Transpose,
    transb: Transpose,
    m: usize,
    n: usize,
    k: usize,
    alpha: f64,
    a: &[f64],
    b: &[f64],
    beta: f64,
    c: &mut [f64],
) -> Result<(), ()> {
    if a.len() != m * k || b.len() != k * n || c.len() != m * n {
        return Err(());
    }
    let a_at = |i: usize, l: usize| -> f64 {
        match transa {
            Transpose::No => a[i * k + l],
            Transpose::Yes => a[l * m + i],
        }
    };
    let b_at = |l: usize, j: usize| -> f64 {
        match transb {
            Transpose::No => b[l * n + j],
            Transpose::Yes => b[j * k + l],
        }
    };
    for i in 0..m {
        for j in 0..n {
            let mut s = 0.0;
            for l in 0..k {
                s += a_at(i, l) * b_at(l, j);
            }
            let idx = i * n + j;
            c[idx] = if beta == 0.0 {
                alpha * s
            } else {
                alpha * s + beta * c[idx]
            };
        }
    }
    Ok(())
}

/// `LinearAlgebra.gemm` — `C := alpha·op(A)·op(B) + beta·C` (row-major, CPU-only).
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
        return Err(args::bad(span, "gemm: inner dimensions of op(A) and op(B) disagree"));
    }
    let k = k_a;
    let work = m.saturating_mul(n).saturating_mul(k);
    if work >= HOST_GEMM_CPU_CAP {
        return Err(args::bad(
            span,
            format!(
                "LinearAlgebra.gemm Host is CPU-only (m·n·k={work} ≥ {HOST_GEMM_CPU_CAP}); refuse GPU offload"
            ),
        ));
    }

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

    gemm_cpu(transa, transb, m, n, k, alpha, &a.data, &b.data, beta, &mut c)
        .map_err(|_| args::bad(span, "gemm: dimension error"))?;

    Ok(args::record([("c", mat_record(m, n, c))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn mat(rows: u64, cols: u64, data: Vec<f64>) -> Value {
        let mut m = BTreeMap::new();
        m.insert("rows".into(), Value::U64(rows));
        m.insert("cols".into(), Value::U64(cols));
        m.insert("data".into(), args::f64_list_value(data));
        Value::Record(m)
    }

    #[test]
    fn wave13_gemm_2x2_identity() {
        let mut args = BTreeMap::new();
        args.insert("a".into(), mat(2, 2, vec![1.0, 2.0, 3.0, 4.0]));
        args.insert("b".into(), mat(2, 2, vec![1.0, 0.0, 0.0, 1.0]));
        args.insert("alpha".into(), Value::F64(1.0));
        args.insert("beta".into(), Value::F64(0.0));
        let out = gemm_host(&Value::Record(args), span()).unwrap();
        let c = args::rec(&out, "c").unwrap();
        let data = args::rec_f64_list(c, "data").unwrap();
        assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn wave13_gemm_rejects_gpu_sized_work() {
        // Claim huge dims without allocating huge buffers — parse fails length check.
        // Instead: use dims whose product hits the Host cap with modest buffers.
        let side = 32usize;
        let data = vec![0.0_f64; side * side];
        let mut args = BTreeMap::new();
        args.insert("a".into(), mat(side as u64, side as u64, data.clone()));
        args.insert("b".into(), mat(side as u64, side as u64, data));
        // 32³ = 32768 == HOST_GEMM_CPU_CAP → reject
        assert!(gemm_host(&Value::Record(args), span()).is_err());
    }
}
