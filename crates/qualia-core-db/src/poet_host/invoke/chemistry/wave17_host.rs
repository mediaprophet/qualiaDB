//! Wave-17 Host binds: Chemistry SCF linear-algebra helpers (CPU / stack DIIS path).
//!
//! Wraps `jacobi_diagonalization`, `transpose`, `orthogonalization_matrix` for fixed
//! 2×2 / 3×3 — no GPU / forge / `caps()`.

use super::super::args;
use crate::specialized_libs::chemistry_modeling::scf::{
    jacobi_diagonalization, orthogonalization_matrix, transpose,
};
use crate::specialized_libs::shared::zero_heap_algebra::ZeroHeapMatrix;
use vibe::{Diagnostic, Span, Value};

fn parse_square_f64(
    a_val: &Value,
    span: Span,
    what: &str,
) -> Result<(usize, Vec<Vec<f64>>), Diagnostic> {
    let rows = match a_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                format!("{what}: a must be a list of rows"),
            ))
        }
    };
    let n = rows.len();
    if n != 2 && n != 3 {
        return Err(args::bad(span, format!("{what}: only 2×2 or 3×3 supported")));
    }
    let mut out = Vec::with_capacity(n);
    for row in rows {
        let vals = args::f64s(row).ok_or_else(|| {
            args::bad(span, format!("{what}: row must be [f64]"))
        })?;
        if vals.len() != n {
            return Err(args::bad(
                span,
                format!("{what}: rows must be length {n}"),
            ));
        }
        out.push(vals);
    }
    Ok((n, out))
}

fn fill_mat<const N: usize>(rows: &[Vec<f64>]) -> ZeroHeapMatrix<f64, N, N> {
    let mut mat = ZeroHeapMatrix::<f64, N, N>::zeros();
    for (i, row) in rows.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            mat.set(i, j, v);
        }
    }
    mat
}

fn mat_to_list<const N: usize>(m: &ZeroHeapMatrix<f64, N, N>) -> Value {
    let mut rows = Vec::with_capacity(N);
    for i in 0..N {
        let mut row = Vec::with_capacity(N);
        for j in 0..N {
            row.push(Value::F64(m.get(i, j)));
        }
        rows.push(Value::List(row));
    }
    Value::List(rows)
}

/// `Chemistry.jacobi_diagonalization` — eigen of real symmetric A (N∈{2,3}).
/// Args: `{ a: [[f64;N];N] }`. Out: `{ eigenvalues: [f64], eigenvectors: [[f64]] }`.
pub fn jacobi_diagonalization_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_val = args::rec(args_v, "a")
        .ok_or_else(|| args::bad(span, "jacobi_diagonalization needs a: matrix"))?;
    let (n, rows) = parse_square_f64(a_val, span, "jacobi_diagonalization")?;
    match n {
        2 => {
            let mat = fill_mat::<2>(&rows);
            match jacobi_diagonalization(&mat) {
                Ok((evals, evecs)) => Ok(args::record([
                    (
                        "eigenvalues",
                        Value::List(evals.iter().map(|v| Value::F64(*v)).collect()),
                    ),
                    ("eigenvectors", mat_to_list(&evecs)),
                ])),
                Err(e) => Err(args::bad(span, format!("jacobi_diagonalization: {e:?}"))),
            }
        }
        3 => {
            let mat = fill_mat::<3>(&rows);
            match jacobi_diagonalization(&mat) {
                Ok((evals, evecs)) => Ok(args::record([
                    (
                        "eigenvalues",
                        Value::List(evals.iter().map(|v| Value::F64(*v)).collect()),
                    ),
                    ("eigenvectors", mat_to_list(&evecs)),
                ])),
                Err(e) => Err(args::bad(span, format!("jacobi_diagonalization: {e:?}"))),
            }
        }
        _ => unreachable!(),
    }
}

/// `Chemistry.transpose` — square transpose (N∈{2,3}).
/// Args: `{ a: [[f64;N];N] }`. Out: `{ a: [[f64]] }`.
pub fn transpose_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_val = args::rec(args_v, "a").ok_or_else(|| args::bad(span, "transpose needs a: matrix"))?;
    let (n, rows) = parse_square_f64(a_val, span, "transpose")?;
    match n {
        2 => {
            let mat = fill_mat::<2>(&rows);
            Ok(args::record([("a", mat_to_list(&transpose(&mat)))]))
        }
        3 => {
            let mat = fill_mat::<3>(&rows);
            Ok(args::record([("a", mat_to_list(&transpose(&mat)))]))
        }
        _ => unreachable!(),
    }
}

/// `Chemistry.orthogonalization_matrix` — Löwdin X = S^{-1/2} (N∈{2,3}).
/// Args: `{ s: [[f64;N];N] }`. Out: `{ x: [[f64]] }`.
pub fn orthogonalization_matrix_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s_val = args::rec(args_v, "s")
        .ok_or_else(|| args::bad(span, "orthogonalization_matrix needs s: matrix"))?;
    let (n, rows) = parse_square_f64(s_val, span, "orthogonalization_matrix")?;
    match n {
        2 => {
            let mat = fill_mat::<2>(&rows);
            match orthogonalization_matrix(&mat) {
                Ok(x) => Ok(args::record([("x", mat_to_list(&x))])),
                Err(e) => Err(args::bad(span, format!("orthogonalization_matrix: {e:?}"))),
            }
        }
        3 => {
            let mat = fill_mat::<3>(&rows);
            match orthogonalization_matrix(&mat) {
                Ok(x) => Ok(args::record([("x", mat_to_list(&x))])),
                Err(e) => Err(args::bad(span, format!("orthogonalization_matrix: {e:?}"))),
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave17_jacobi_diagonalization_diag_2x2() {
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(2.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(5.0)]),
            ]),
        );
        let out = jacobi_diagonalization_host(&Value::Record(m), span()).unwrap();
        let evals = args::rec_f64_list(&out, "eigenvalues").unwrap();
        assert!((evals[0] - 2.0).abs() < 1e-10);
        assert!((evals[1] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn wave17_transpose_2x2() {
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(1.0), Value::F64(2.0)]),
                Value::List(vec![Value::F64(3.0), Value::F64(4.0)]),
            ]),
        );
        let out = transpose_host(&Value::Record(m), span()).unwrap();
        let a = args::rec(&out, "a").unwrap();
        let rows = match a {
            Value::List(l) => l,
            _ => panic!("expected list"),
        };
        let r0 = args::f64s(&rows[0]).unwrap();
        let r1 = args::f64s(&rows[1]).unwrap();
        assert!((r0[0] - 1.0).abs() < 1e-12 && (r0[1] - 3.0).abs() < 1e-12);
        assert!((r1[0] - 2.0).abs() < 1e-12 && (r1[1] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn wave17_orthogonalization_matrix_identity() {
        let mut m = BTreeMap::new();
        m.insert(
            "s".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(1.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(1.0)]),
            ]),
        );
        let out = orthogonalization_matrix_host(&Value::Record(m), span()).unwrap();
        let x = args::rec(&out, "x").unwrap();
        let rows = match x {
            Value::List(l) => l,
            _ => panic!("expected list"),
        };
        let r0 = args::f64s(&rows[0]).unwrap();
        let r1 = args::f64s(&rows[1]).unwrap();
        assert!((r0[0] - 1.0).abs() < 1e-9 && r0[1].abs() < 1e-9);
        assert!(r1[0].abs() < 1e-9 && (r1[1] - 1.0).abs() < 1e-9);
    }
}
