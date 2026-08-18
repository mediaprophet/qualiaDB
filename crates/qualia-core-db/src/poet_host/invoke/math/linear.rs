//! `solvers::linear_algebra::gemm::matmul` — future seam stays under `qualia-math`.

use super::super::args;
use crate::solvers::linear_algebra::eigen::symmetric_eigen;
use crate::solvers::linear_algebra::gemm::matmul;
use crate::solvers::linear_algebra::lu::{lu_decompose, determinant as lu_determinant};
use crate::solvers::linear_algebra::spectral::eigenvalues_general;
use crate::solvers::linear_algebra::svd::svd as svd_decompose;
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

/// Transpose a row-major matrix: `out[j*rows+i] = data[i*cols+j]`. The output
/// dimensions are swapped (`rows` ← `cols`, `cols` ← `rows`). Wraps the engine's
/// GEMM transpose path's data layout in a value-level seam.
pub fn transpose(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    let mut out = vec![0.0_f64; m.rows * m.cols];
    for i in 0..m.rows {
        for j in 0..m.cols {
            out[j * m.rows + i] = m.data[i * m.cols + j];
        }
    }
    Ok(args::record([
        ("rows", Value::U64(m.cols as u64)),
        ("cols", Value::U64(m.rows as u64)),
        ("data", args::f64_list_value(out)),
    ]))
}

/// Determinant of a square row-major matrix via LU decomposition with partial
/// pivoting. Wraps `crate::solvers::linear_algebra::lu::determinant`.
pub fn determinant(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    if m.rows != m.cols {
        return Err(args::bad(span, "determinant needs a square matrix"));
    }
    let det = lu_determinant(m.rows, &m.data)
        .map_err(|e| args::bad(span, format!("determinant: {e:?}")))?;
    Ok(args::record([("determinant", Value::F64(det))]))
}

/// Solve a general linear system `A x = b` via LU decomposition with partial
/// pivoting. Wraps `crate::solvers::linear_algebra::lu::lu_decompose` + `Lu::solve`.
/// Input: record with `a` (matrix) and `b` (f64 list). Output: record {x: f64 list}.
pub fn solve(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = matrix(args_v, "a", span)?;
    if a.rows != a.cols {
        return Err(args::bad(span, "solve needs a square a"));
    }
    let b = args::rec(args_v, "b")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "solve needs b as a number list"))?;
    if b.len() != a.rows {
        return Err(args::bad(span, "solve: b length must equal a.rows"));
    }
    let lu = lu_decompose(a.rows, &a.data)
        .map_err(|e| args::bad(span, format!("solve lu_decompose: {e:?}")))?;
    let x = lu
        .solve(&b)
        .ok_or_else(|| args::bad(span, "solve: matrix is singular"))?;
    Ok(args::record([("x", args::f64_list_value(x))]))
}

/// Eigendecomposition of a symmetric `n×n` matrix by cyclic Jacobi rotations.
/// Wraps `crate::solvers::linear_algebra::eigen::symmetric_eigen`. Input must be
/// square and symmetric. Output: record {eigenvalues: f64 list, eigenvectors: {rows, cols, data}}.
pub fn eigen_symmetric(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    if m.rows != m.cols {
        return Err(args::bad(span, "eigen_symmetric needs a square matrix"));
    }
    let n = m.rows;
    let mut a = m.data;
    let mut eigvecs = vec![0.0_f64; n * n];
    symmetric_eigen(n, &mut a, &mut eigvecs)
        .map_err(|e| args::bad(span, format!("eigen_symmetric: {e:?}")))?;
    let eigenvalues: Vec<f64> = (0..n).map(|i| a[i * n + i]).collect();
    Ok(args::record([
        ("eigenvalues", args::f64_list_value(eigenvalues)),
        (
            "eigenvectors",
            args::record([
                ("rows", Value::U64(n as u64)),
                ("cols", Value::U64(n as u64)),
                ("data", args::f64_list_value(eigvecs)),
            ]),
        ),
    ]))
}

/// Eigenvalues of a general (not necessarily symmetric) `n×n` matrix as complex
/// numbers. Wraps `crate::solvers::linear_algebra::spectral::eigenvalues_general`.
/// Output: record {eigenvalues: list of {re: f64, im: f64}}.
pub fn eigenvalues(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    if m.rows != m.cols {
        return Err(args::bad(span, "eigenvalues needs a square matrix"));
    }
    let eigs = eigenvalues_general(m.rows, &m.data)
        .map_err(|e| args::bad(span, format!("eigenvalues: {e:?}")))?;
    let list: Vec<Value> = eigs
        .iter()
        .map(|z| {
            args::record([
                ("re", Value::F64(z.re)),
                ("im", Value::F64(z.im)),
            ])
        })
        .collect();
    Ok(args::record([("eigenvalues", Value::List(list))]))
}

/// Thin singular value decomposition `A = U·Σ·Vᵀ` of a row-major `m×n` matrix.
/// Wraps `crate::solvers::linear_algebra::svd::svd`. Singular values are returned
/// in descending order. Output: record {singular_values: f64 list, u: {rows, cols, data}, v: {rows, cols, data}}.
pub fn svd(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    let s = svd_decompose(m.rows, m.cols, &m.data)
        .map_err(|e| args::bad(span, format!("svd: {e:?}")))?;
    Ok(args::record([
        ("singular_values", args::f64_list_value(s.singular_values)),
        (
            "u",
            args::record([
                ("rows", Value::U64(m.rows as u64)),
                ("cols", Value::U64(m.cols as u64)),
                ("data", args::f64_list_value(s.u)),
            ]),
        ),
        (
            "v",
            args::record([
                ("rows", Value::U64(m.cols as u64)),
                ("cols", Value::U64(m.cols as u64)),
                ("data", args::f64_list_value(s.v)),
            ]),
        ),
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

    fn matrix_arg(rows: u64, cols: u64, data: Vec<f64>) -> Value {
        let mut m = BTreeMap::new();
        m.insert("matrix".into(), mat(rows, cols, data));
        Value::Record(m)
    }

    #[test]
    fn transpose_2x3() {
        // [[1,2,3],[4,5,6]] → 3×2 [[1,4],[2,5],[3,6]]
        let v = transpose(
            &matrix_arg(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("rows"), Some(&Value::U64(3)));
                assert_eq!(r.get("cols"), Some(&Value::U64(2)));
                match r.get("data") {
                    Some(Value::List(xs)) => {
                        assert_eq!(xs, &vec![
                            Value::F64(1.0),
                            Value::F64(4.0),
                            Value::F64(2.0),
                            Value::F64(5.0),
                            Value::F64(3.0),
                            Value::F64(6.0),
                        ]);
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
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
