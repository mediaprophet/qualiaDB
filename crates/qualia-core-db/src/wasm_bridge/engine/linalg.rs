//! Dense linear algebra exports — matrix multiply / transpose / determinant /
//! linear solve / symmetric & general eigenvalues / SVD / polynomial roots.
//!
//! All matrices are row-major `f64`. These call the engine's canonical solvers
//! (`crate::solvers::linear_algebra::*`, `crate::solvers::polynomial`) — the same
//! code the native `matrix_operation` / `algebra_matrix_analyze` MCP tools use.

#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

#[derive(Deserialize)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    fn check(&self) -> Result<(), JsValue> {
        if self.rows == 0 || self.cols == 0 || self.data.len() != self.rows * self.cols {
            return Err(JsValue::from_str(
                "matrix: rows·cols must be non-zero and equal data.len()",
            ));
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct MatrixOut {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

#[derive(Serialize)]
struct Cplx {
    re: f64,
    im: f64,
}

/// `C = A · B`. Input `{ a:{rows,cols,data}, b:{rows,cols,data} }`,
/// output `{ rows, cols, data }`. Errors on a shape mismatch (`a.cols != b.rows`).
#[wasm_bindgen]
pub fn la_matmul_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: Matrix,
        b: Matrix,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    p.a.check()?;
    p.b.check()?;
    if p.a.cols != p.b.rows {
        return Err(JsValue::from_str("matmul: a.cols must equal b.rows"));
    }
    let (m, k, n) = (p.a.rows, p.a.cols, p.b.cols);
    let mut c = vec![0.0_f64; m * n];
    crate::solvers::linear_algebra::gemm::matmul(m, k, n, &p.a.data, &p.b.data, &mut c)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    Ok(serde_wasm_bindgen::to_value(&MatrixOut {
        rows: m,
        cols: n,
        data: c,
    })?)
}

/// Transpose. Input `{ rows, cols, data }` → output `{ rows:cols, cols:rows, data }`.
#[wasm_bindgen]
pub fn la_transpose_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let m: Matrix = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    m.check()?;
    let mut out = vec![0.0_f64; m.rows * m.cols];
    for i in 0..m.rows {
        for j in 0..m.cols {
            out[j * m.rows + i] = m.data[i * m.cols + j];
        }
    }
    Ok(serde_wasm_bindgen::to_value(&MatrixOut {
        rows: m.cols,
        cols: m.rows,
        data: out,
    })?)
}

/// Determinant of a square matrix via LU (partial pivoting).
/// Input `{ rows, cols, data }` (rows==cols) → `{ determinant }`.
#[wasm_bindgen]
pub fn la_determinant_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let m: Matrix = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    m.check()?;
    if m.rows != m.cols {
        return Err(JsValue::from_str("determinant: matrix must be square"));
    }
    let det = crate::solvers::linear_algebra::lu::determinant(m.rows, &m.data)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    #[derive(Serialize)]
    struct Out {
        determinant: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { determinant: det })?)
}

/// Solve `A · x = b` for a square `A` via LU. Input `{ a:{rows,cols,data}, b:[..] }`
/// (b length == a.rows) → `{ x:[..] }`. Errors if `A` is singular.
#[wasm_bindgen]
pub fn la_solve_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: Matrix,
        b: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    p.a.check()?;
    if p.a.rows != p.a.cols {
        return Err(JsValue::from_str("solve: A must be square"));
    }
    if p.b.len() != p.a.rows {
        return Err(JsValue::from_str("solve: b length must equal A.rows"));
    }
    let lu = crate::solvers::linear_algebra::lu::lu_decompose(p.a.rows, &p.a.data)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let x = lu
        .solve(&p.b)
        .ok_or_else(|| JsValue::from_str("solve: matrix is singular"))?;
    #[derive(Serialize)]
    struct Out {
        x: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { x })?)
}

/// Symmetric eigendecomposition (cyclic Jacobi). Input `{ rows, cols, data }`
/// (square, symmetric) → `{ eigenvalues:[..], eigenvectors:{rows,cols,data} }`
/// where eigenvector `j` is column `j` of the row-major `eigenvectors` matrix.
#[wasm_bindgen]
pub fn la_eigen_symmetric_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let m: Matrix = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    m.check()?;
    if m.rows != m.cols {
        return Err(JsValue::from_str("eigen_symmetric: matrix must be square"));
    }
    // Mirror the native facade: marshal into caller-owned buffers, call the engine's
    // canonical symmetric eigensolver; eigenvalues are the rotated diagonal.
    let mut a = m.data.clone();
    let mut vecs = vec![0.0_f64; m.rows * m.rows];
    crate::solvers::linear_algebra::eigen::symmetric_eigen(m.rows, &mut a, &mut vecs)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let vals: Vec<f64> = (0..m.rows).map(|i| a[i * m.rows + i]).collect();
    #[derive(Serialize)]
    struct Out {
        eigenvalues: Vec<f64>,
        eigenvectors: MatrixOut,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        eigenvalues: vals,
        eigenvectors: MatrixOut {
            rows: m.rows,
            cols: m.rows,
            data: vecs,
        },
    })?)
}

/// General (non-symmetric) eigenvalues via the characteristic polynomial.
/// Input `{ rows, cols, data }` (square) → `{ eigenvalues:[{re,im}] }`.
#[wasm_bindgen]
pub fn la_eigenvalues_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let m: Matrix = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    m.check()?;
    if m.rows != m.cols {
        return Err(JsValue::from_str("eigenvalues: matrix must be square"));
    }
    let roots = crate::solvers::linear_algebra::spectral::eigenvalues_general(m.rows, &m.data)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let eigenvalues: Vec<Cplx> = roots
        .iter()
        .map(|r| Cplx { re: r.re, im: r.im })
        .collect();
    #[derive(Serialize)]
    struct Out {
        eigenvalues: Vec<Cplx>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { eigenvalues })?)
}

/// Thin SVD `A = U·Σ·Vᵀ`. Input `{ rows, cols, data }` →
/// `{ singular_values:[..], u:{rows,cols,data}, v:{rows,cols,data} }`
/// (`u` is m×n, `v` is n×n; singular vectors are columns; values descending).
#[wasm_bindgen]
pub fn la_svd_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let m: Matrix = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    m.check()?;
    let s = crate::solvers::linear_algebra::svd::svd(m.rows, m.cols, &m.data)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    #[derive(Serialize)]
    struct Out {
        singular_values: Vec<f64>,
        u: MatrixOut,
        v: MatrixOut,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        singular_values: s.singular_values,
        u: MatrixOut {
            rows: m.rows,
            cols: m.cols,
            data: s.u,
        },
        v: MatrixOut {
            rows: m.cols,
            cols: m.cols,
            data: s.v,
        },
    })?)
}

/// All complex roots of a real polynomial (Durand–Kerner). Input
/// `{ coeffs:[cₙ,…,c₁,c₀] }` (descending) → `{ degree, roots:[{re,im}] }`.
#[wasm_bindgen]
pub fn la_polynomial_roots_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        coeffs: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let roots = crate::solvers::polynomial::polynomial_roots(&p.coeffs)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let out: Vec<Cplx> = roots
        .iter()
        .map(|r| Cplx { re: r.re, im: r.im })
        .collect();
    #[derive(Serialize)]
    struct Out {
        degree: usize,
        roots: Vec<Cplx>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        degree: p.coeffs.len().saturating_sub(1),
        roots: out,
    })?)
}
