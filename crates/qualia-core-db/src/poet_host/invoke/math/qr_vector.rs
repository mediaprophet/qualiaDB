//! Host bindings for QR, matvec, Cholesky solve, and rank-1 vector ops.
//!
//! Wraps `crate::solvers::linear_algebra::{qr, gemm::matvec, cholesky::cholesky_solve, vector}`
//! — pure computational `pub fn`s that had no exact `LinearAlgebra.*` Host twin.

use super::super::args;
use crate::solvers::linear_algebra::cholesky::cholesky_solve as chol_solve;
use crate::solvers::linear_algebra::qr::{qr_factor as qr_fac, qr_form_q, qr_solve_least_squares};
use crate::solvers::linear_algebra::vector::{
    add_assign as vec_add_assign, add_into as vec_add_into, axpy as blas_axpy,
    hadamard_assign as hadamard_mut, hadamard_into as hadamard, scale as vec_scale,
};
use crate::solvers::SolversError;
use vibe::{Diagnostic, Span, Value};

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

/// `LinearAlgebra.qr_factor` — Householder QR of an `m×n` (`m ≥ n`) row-major matrix.
/// Args: `{ matrix: { rows, cols, data } }`. Out: `{ a, tau, rows, cols }` (factored `a`).
pub fn qr_factor(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    if m.rows < m.cols {
        return Err(args::bad(span, "qr_factor needs rows >= cols"));
    }
    let mut a = m.data;
    let mut tau = vec![0.0; m.cols];
    qr_fac(m.rows, m.cols, &mut a, &mut tau)
        .map_err(|e| args::bad(span, format!("qr_factor: {e:?}")))?;
    Ok(args::record([
        ("a", args::f64_list_value(a)),
        ("tau", args::f64_list_value(tau)),
        ("rows", Value::U64(m.rows as u64)),
        ("cols", Value::U64(m.cols as u64)),
    ]))
}

/// `LinearAlgebra.qr_form_q` — thin `Q` from a factored `a`/`tau`.
/// Args: `{ a, tau, rows, cols }`. Out: `{ q: { rows, cols, data } }`.
pub fn qr_form_q_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args_v, "a").ok_or_else(|| args::bad(span, "qr_form_q needs a"))?;
    let tau =
        args::rec_f64_list(args_v, "tau").ok_or_else(|| args::bad(span, "qr_form_q needs tau"))?;
    let rows = args::rec_u64(args_v, "rows")
        .ok_or_else(|| args::bad(span, "qr_form_q needs rows"))? as usize;
    let cols = args::rec_u64(args_v, "cols")
        .ok_or_else(|| args::bad(span, "qr_form_q needs cols"))? as usize;
    let mut q = vec![0.0; rows * cols];
    qr_form_q(rows, cols, &a, &tau, &mut q)
        .map_err(|e| args::bad(span, format!("qr_form_q: {e:?}")))?;
    Ok(args::record([("q", mat_record(rows, cols, q))]))
}

/// `LinearAlgebra.qr_solve_least_squares` — `min‖A x − b‖` given factored QR.
/// Args: `{ a, tau, rows, cols, b }`. Out: `{ x }`.
pub fn qr_solve_least_squares_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args_v, "a")
        .ok_or_else(|| args::bad(span, "qr_solve_least_squares needs a"))?;
    let tau = args::rec_f64_list(args_v, "tau")
        .ok_or_else(|| args::bad(span, "qr_solve_least_squares needs tau"))?;
    let rows = args::rec_u64(args_v, "rows")
        .ok_or_else(|| args::bad(span, "qr_solve_least_squares needs rows"))?
        as usize;
    let cols = args::rec_u64(args_v, "cols")
        .ok_or_else(|| args::bad(span, "qr_solve_least_squares needs cols"))?
        as usize;
    let mut b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "qr_solve_least_squares needs b"))?;
    let mut x = vec![0.0; cols];
    qr_solve_least_squares(rows, cols, &a, &tau, &mut b, &mut x)
        .map_err(|e| args::bad(span, format!("qr_solve_least_squares: {e:?}")))?;
    Ok(args::record([("x", args::f64_list_value(x))]))
}

/// `LinearAlgebra.add_into` — `c[i] = a[i] + b[i]`. Args: `{ a, b }`. Out: `{ c }`.
pub fn add_into(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args_v, "a").ok_or_else(|| args::bad(span, "add_into needs a"))?;
    let b = args::rec_f64_list(args_v, "b").ok_or_else(|| args::bad(span, "add_into needs b"))?;
    let mut c = vec![0.0; a.len()];
    vec_add_into(&a, &b, &mut c).map_err(|e| args::bad(span, format!("add_into: {e:?}")))?;
    Ok(args::record([("c", args::f64_list_value(c))]))
}

/// `LinearAlgebra.cholesky_solve` — solve `A x = b` given Cholesky factor `L`.
/// Args: `{ l, n, b }`. Out: `{ x }`.
pub fn cholesky_solve(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let l = args::rec_f64_list(args_v, "l")
        .ok_or_else(|| args::bad(span, "cholesky_solve needs l"))?;
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "cholesky_solve needs n"))? as usize;
    let b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "cholesky_solve needs b"))?;
    let mut x = vec![0.0; n];
    chol_solve(n, &l, &b, &mut x).map_err(|e| args::bad(span, format!("cholesky_solve: {e:?}")))?;
    Ok(args::record([("x", args::f64_list_value(x))]))
}

/// `LinearAlgebra.axpy` — `y += α·x` (BLAS axpy). Args: `{ alpha, x, y }`. Out: `{ y }`.
pub fn axpy(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args_v, "alpha").ok_or_else(|| args::bad(span, "axpy needs alpha"))?;
    let x = args::rec_f64_list(args_v, "x").ok_or_else(|| args::bad(span, "axpy needs x"))?;
    let mut y = args::rec_f64_list(args_v, "y").ok_or_else(|| args::bad(span, "axpy needs y"))?;
    blas_axpy(alpha, &x, &mut y).map_err(|e| args::bad(span, format!("axpy: {e:?}")))?;
    Ok(args::record([("y", args::f64_list_value(y))]))
}

/// `LinearAlgebra.hadamard_into` — element-wise product. Args: `{ a, b }`. Out: `{ c }`.
pub fn hadamard_into(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a =
        args::rec_f64_list(args_v, "a").ok_or_else(|| args::bad(span, "hadamard_into needs a"))?;
    let b =
        args::rec_f64_list(args_v, "b").ok_or_else(|| args::bad(span, "hadamard_into needs b"))?;
    let mut c = vec![0.0; a.len()];
    hadamard(&a, &b, &mut c).map_err(|e| args::bad(span, format!("hadamard_into: {e:?}")))?;
    Ok(args::record([("c", args::f64_list_value(c))]))
}

/// `LinearAlgebra.add_assign` — `a[i] += b[i]`. Args: `{ a, b }`. Out: `{ a }`.
pub fn add_assign(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut a =
        args::rec_f64_list(args_v, "a").ok_or_else(|| args::bad(span, "add_assign needs a"))?;
    let b =
        args::rec_f64_list(args_v, "b").ok_or_else(|| args::bad(span, "add_assign needs b"))?;
    vec_add_assign(&mut a, &b).map_err(|e| args::bad(span, format!("add_assign: {e:?}")))?;
    Ok(args::record([("a", args::f64_list_value(a))]))
}

/// `LinearAlgebra.hadamard_assign` — `a[i] *= b[i]`. Args: `{ a, b }`. Out: `{ a }`.
pub fn hadamard_assign(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut a = args::rec_f64_list(args_v, "a")
        .ok_or_else(|| args::bad(span, "hadamard_assign needs a"))?;
    let b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "hadamard_assign needs b"))?;
    hadamard_mut(&mut a, &b).map_err(|e| args::bad(span, format!("hadamard_assign: {e:?}")))?;
    Ok(args::record([("a", args::f64_list_value(a))]))
}

/// `LinearAlgebra.scale` — `a[i] *= s`. Args: `{ a, s }`. Out: `{ a }`.
pub fn scale(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut a = args::rec_f64_list(args_v, "a").ok_or_else(|| args::bad(span, "scale needs a"))?;
    let s = args::rec_f64(args_v, "s").ok_or_else(|| args::bad(span, "scale needs s"))?;
    vec_scale(&mut a, s);
    Ok(args::record([("a", args::f64_list_value(a))]))
}

/// `LinearAlgebra.matvec` — `y := op(A)·x` (row-major). Args:
/// `{ matrix: { rows, cols, data }, x, transpose? }`. Out: `{ y }`.
///
/// `rows`/`cols` describe stored `A`. When `transpose` is false, `op(A)=A` (`rows×cols`),
/// `x` length `cols`, `y` length `rows`. When true, `op(A)=Aᵀ` (`cols×rows`), `x` length
/// `rows`, `y` length `cols`.
///
/// Host uses the **CPU floor** of `solvers::linear_algebra::gemm::matvec` (same index
/// arithmetic). The library entry also probes `wgsl_forge::caps()` before the threshold
/// check, which can panic when `cuda.dll` is absent — Host must not take that path.
pub fn matvec(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let m = matrix(args_v, "matrix", span)?;
    let x = args::rec_f64_list(args_v, "x").ok_or_else(|| args::bad(span, "matvec needs x"))?;
    let transpose = args::rec_bool(args_v, "transpose").unwrap_or(false);
    let (op_rows, op_cols) = if transpose {
        (m.cols, m.rows)
    } else {
        (m.rows, m.cols)
    };
    let mut y = vec![0.0; op_rows];
    matvec_cpu(transpose, op_rows, op_cols, &m.data, &x, &mut y)
        .map_err(|e| args::bad(span, format!("matvec: {e:?}")))?;
    Ok(args::record([("y", args::f64_list_value(y))]))
}

/// `LinearAlgebra.symmetric_eigen_3x3` — closed-form symmetric 3×3 eigenvalues
/// (Smith), sorted descending. Pure CPU; no forge/CUDA. Args: `{ a }` (row-major
/// length 9). Out: `{ eigenvalues }`.
pub fn symmetric_eigen_3x3(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args_v, "a")
        .ok_or_else(|| args::bad(span, "symmetric_eigen_3x3 needs a: length-9 list"))?;
    if a.len() != 9 {
        return Err(args::bad(span, "symmetric_eigen_3x3: a must have length 9"));
    }
    let mut buf = [0.0f64; 9];
    buf.copy_from_slice(&a);
    let e = crate::solvers::linear_algebra::eigen::symmetric_eigen_3x3(&buf);
    Ok(args::record([("eigenvalues", args::f64_list_value(e))]))
}

/// CPU floor matching `gemm::matvec` (no forge / CUDA probe).
fn matvec_cpu(
    transpose: bool,
    m: usize,
    n: usize,
    a: &[f64],
    x: &[f64],
    y: &mut [f64],
) -> Result<(), SolversError> {
    if a.len() != m * n || x.len() != n || y.len() != m {
        return Err(SolversError::InvalidDimension);
    }
    let a_at = |i: usize, j: usize| -> f64 {
        if transpose {
            a[j * m + i]
        } else {
            a[i * n + j]
        }
    };
    for i in 0..m {
        let mut s = 0.0;
        for j in 0..n {
            s += a_at(i, j) * x[j];
        }
        y[i] = s;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::linear_algebra::cholesky::cholesky_factor;
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

    fn f64s(v: &Value, key: &str) -> Vec<f64> {
        match args::rec(v, key).and_then(args::f64s) {
            Some(xs) => xs,
            None => panic!("missing {key} in {v:?}"),
        }
    }

    #[test]
    fn qr_square_solve_known() {
        // [[2,1],[1,3]] x = [3,5] → [0.8, 1.4]
        let mut args = BTreeMap::new();
        args.insert("matrix".into(), mat(2, 2, vec![2.0, 1.0, 1.0, 3.0]));
        let fac = qr_factor(&Value::Record(args), span()).unwrap();
        let a = f64s(&fac, "a");
        let tau = f64s(&fac, "tau");
        let mut solve_args = BTreeMap::new();
        solve_args.insert("a".into(), args::f64_list_value(a));
        solve_args.insert("tau".into(), args::f64_list_value(tau));
        solve_args.insert("rows".into(), Value::U64(2));
        solve_args.insert("cols".into(), Value::U64(2));
        solve_args.insert("b".into(), args::f64_list_value(vec![3.0, 5.0]));
        let out = qr_solve_least_squares_host(&Value::Record(solve_args), span()).unwrap();
        let x = f64s(&out, "x");
        assert!((x[0] - 0.8).abs() < 1e-9);
        assert!((x[1] - 1.4).abs() < 1e-9);
    }

    #[test]
    fn qr_form_q_orthonormal_columns() {
        let mut args = BTreeMap::new();
        args.insert(
            "matrix".into(),
            mat(3, 3, vec![12.0, -51.0, 4.0, 6.0, 167.0, -68.0, -4.0, 24.0, -41.0]),
        );
        let fac = qr_factor(&Value::Record(args), span()).unwrap();
        let mut q_args = BTreeMap::new();
        q_args.insert("a".into(), args::rec(&fac, "a").unwrap().clone());
        q_args.insert("tau".into(), args::rec(&fac, "tau").unwrap().clone());
        q_args.insert("rows".into(), Value::U64(3));
        q_args.insert("cols".into(), Value::U64(3));
        let out = qr_form_q_host(&Value::Record(q_args), span()).unwrap();
        let q = args::rec(&out, "q").unwrap();
        let data = args::rec(q, "data").and_then(args::f64s).unwrap();
        // First column unit length.
        let n0 = (data[0] * data[0] + data[3] * data[3] + data[6] * data[6]).sqrt();
        assert!((n0 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn add_into_residual() {
        let mut args = BTreeMap::new();
        args.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        args.insert("b".into(), args::f64_list_value(vec![10.0, 20.0, 30.0]));
        let out = add_into(&Value::Record(args), span()).unwrap();
        assert_eq!(f64s(&out, "c"), vec![11.0, 22.0, 33.0]);
    }

    #[test]
    fn cholesky_solve_spd() {
        let a = [4.0, 12.0, -16.0, 12.0, 37.0, -43.0, -16.0, -43.0, 98.0];
        let mut l = [0.0; 9];
        cholesky_factor(3, &a, &mut l).unwrap();
        let mut args = BTreeMap::new();
        args.insert("l".into(), args::f64_list_value(l.to_vec()));
        args.insert("n".into(), Value::U64(3));
        args.insert("b".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        let out = cholesky_solve(&Value::Record(args), span()).unwrap();
        let x = f64s(&out, "x");
        assert_eq!(x.len(), 3);
        // A x ≈ b via factor path — residual check with known A.
        let ax0 = a[0] * x[0] + a[1] * x[1] + a[2] * x[2];
        let ax1 = a[3] * x[0] + a[4] * x[1] + a[5] * x[2];
        let ax2 = a[6] * x[0] + a[7] * x[1] + a[8] * x[2];
        assert!((ax0 - 1.0).abs() < 1e-9);
        assert!((ax1 - 2.0).abs() < 1e-9);
        assert!((ax2 - 3.0).abs() < 1e-9);
    }

    #[test]
    fn axpy_and_hadamard() {
        let mut ax = BTreeMap::new();
        ax.insert("alpha".into(), Value::F64(0.5));
        ax.insert("x".into(), args::f64_list_value(vec![1.0, 1.0, 1.0]));
        ax.insert("y".into(), args::f64_list_value(vec![10.0, 20.0, 30.0]));
        let y = f64s(&axpy(&Value::Record(ax), span()).unwrap(), "y");
        assert_eq!(y, vec![10.5, 20.5, 30.5]);

        let mut h = BTreeMap::new();
        h.insert("a".into(), args::f64_list_value(vec![2.0, 3.0, 4.0]));
        h.insert("b".into(), args::f64_list_value(vec![5.0, 0.0, -1.0]));
        let c = f64s(&hadamard_into(&Value::Record(h), span()).unwrap(), "c");
        assert_eq!(c, vec![10.0, 0.0, -4.0]);
    }

    #[test]
    fn qr_factor_rejects_wide_matrix() {
        let mut args = BTreeMap::new();
        args.insert("matrix".into(), mat(2, 3, vec![1.0; 6]));
        assert!(qr_factor(&Value::Record(args), span()).is_err());
    }

    #[test]
    fn add_assign_hadamard_assign_scale() {
        let mut aa = BTreeMap::new();
        aa.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        aa.insert("b".into(), args::f64_list_value(vec![10.0, 20.0, 30.0]));
        assert_eq!(
            f64s(&add_assign(&Value::Record(aa), span()).unwrap(), "a"),
            vec![11.0, 22.0, 33.0]
        );

        let mut ha = BTreeMap::new();
        ha.insert("a".into(), args::f64_list_value(vec![2.0, 3.0, 4.0]));
        ha.insert("b".into(), args::f64_list_value(vec![5.0, 0.0, -1.0]));
        assert_eq!(
            f64s(&hadamard_assign(&Value::Record(ha), span()).unwrap(), "a"),
            vec![10.0, 0.0, -4.0]
        );

        let mut sc = BTreeMap::new();
        sc.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        sc.insert("s".into(), Value::F64(2.0));
        assert_eq!(
            f64s(&scale(&Value::Record(sc), span()).unwrap(), "a"),
            vec![2.0, 4.0, 6.0]
        );
    }

    #[test]
    fn matvec_plain_and_transpose() {
        // A = [[1,2],[3,4],[5,6]] (3×2), x = [1,1] → y = [3,7,11]
        let mut plain = BTreeMap::new();
        plain.insert(
            "matrix".into(),
            mat(3, 2, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        );
        plain.insert("x".into(), args::f64_list_value(vec![1.0, 1.0]));
        assert_eq!(
            f64s(&matvec(&Value::Record(plain), span()).unwrap(), "y"),
            vec![3.0, 7.0, 11.0]
        );

        // Aᵀ x with x = [1,1,1] → [9,12]
        let mut tr = BTreeMap::new();
        tr.insert(
            "matrix".into(),
            mat(3, 2, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        );
        tr.insert("x".into(), args::f64_list_value(vec![1.0, 1.0, 1.0]));
        tr.insert("transpose".into(), Value::Bool(true));
        assert_eq!(
            f64s(&matvec(&Value::Record(tr), span()).unwrap(), "y"),
            vec![9.0, 12.0]
        );
    }

    #[test]
    fn matvec_rejects_bad_x_len() {
        let mut args = BTreeMap::new();
        args.insert("matrix".into(), mat(2, 2, vec![1.0, 0.0, 0.0, 1.0]));
        args.insert("x".into(), args::f64_list_value(vec![1.0]));
        assert!(matvec(&Value::Record(args), span()).is_err());
    }

    #[test]
    fn wave12_symmetric_eigen_3x3_diagonal() {
        let mut args = BTreeMap::new();
        // diag(3,2,1) → eigenvalues descending [3,2,1]
        args.insert(
            "a".into(),
            args::f64_list_value(vec![3.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]),
        );
        let out = symmetric_eigen_3x3(&Value::Record(args), span()).unwrap();
        assert_eq!(f64s(&out, "eigenvalues"), vec![3.0, 2.0, 1.0]);
    }
}
