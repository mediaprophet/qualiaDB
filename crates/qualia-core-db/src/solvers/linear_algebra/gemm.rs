//! Dynamic-size general matrix multiply (GEMM) — the canonical dense-linear-algebra core.
//!
//! This is the **one** dynamic GEMM for the engine. Before this, three private
//! re-implementations competed (`specialized_libs/linear_algebra` heap GEMM,
//! `gguf_bridge` GPU `coop_gemv`, and the fixed-size [`super::Matrix4x4`]). The
//! specialized libs route their dynamic matmul here; the GPU `coop_gemv` kernel is
//! the *same contract* executed on `wgpu` and is checked against this code as its
//! CPU parity reference (`gemm_parity_probe`).
//!
//! Idiom (matches [`super::cholesky`]): **zero allocation**, caller-owned **row-major**
//! slices with explicit dimensions, fail-closed on a dimension mismatch
//! ([`SolversError::InvalidDimension`]). No `DMatrix`, no heap, no dependency.

use crate::solvers::SolversError;

/// Whether a GEMM operand is used as stored (`No`) or transposed (`Yes`).
///
/// `op(X)` below means `X` when `No` and `Xᵀ` when `Yes`. Transpose is expressed
/// by index arithmetic only — no operand is ever materialised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transpose {
    /// Use the operand as stored.
    No,
    /// Use the transpose of the operand.
    Yes,
}

/// General matrix multiply (BLAS-3 `gemm` shape), row-major, zero-heap:
///
/// ```text
/// C := alpha · op(A) · op(B) + beta · C
/// ```
///
/// where `op(A)` is `m×k`, `op(B)` is `k×n`, and `C` is `m×n`. Operands are
/// caller-owned row-major slices:
/// - `a` holds `op==No ? m×k : k×m` → always `m*k` elements.
/// - `b` holds `op==No ? k×n : n×k` → always `k*n` elements.
/// - `c` is `m*n` elements, read (when `beta != 0`) and overwritten in place.
///
/// `beta == 0.0` is honoured as a hard zero (the existing contents of `c`, which
/// may be uninitialised garbage, are *not* read) — matching BLAS semantics so a
/// fresh output buffer need not be zeroed first.
///
/// Returns [`SolversError::InvalidDimension`] if any slice length disagrees with
/// `m`, `n`, `k`.
pub fn gemm(
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
) -> Result<(), SolversError> {
    if a.len() != m * k || b.len() != k * n || c.len() != m * n {
        return Err(SolversError::InvalidDimension);
    }
    // op(A)[i][l] — A stored m×k (No) or k×m (Yes).
    let a_at = |i: usize, l: usize| -> f64 {
        match transa {
            Transpose::No => a[i * k + l],
            Transpose::Yes => a[l * m + i],
        }
    };
    // op(B)[l][j] — B stored k×n (No) or n×k (Yes).
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
            // beta==0 ⇒ hard zero, so c may be uninitialised on entry (BLAS rule).
            c[idx] = if beta == 0.0 {
                alpha * s
            } else {
                alpha * s + beta * c[idx]
            };
        }
    }
    Ok(())
}

/// Plain product `C := A · B` for `A` (`m×k`), `B` (`k×n`), `C` (`m×n`),
/// all row-major and caller-owned. Thin wrapper over [`gemm`] with
/// `alpha = 1`, `beta = 0` and no transposes.
pub fn matmul(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
    c: &mut [f64],
) -> Result<(), SolversError> {
    gemm(Transpose::No, Transpose::No, m, n, k, 1.0, a, b, 0.0, c)
}

/// Matrix–vector product `y := op(A) · x`.
///
/// `op(A)` is `m×n`; `a` holds `op==No ? m×n : n×m` (always `m*n` elements),
/// `x` is length `n`, `y` is length `m` (overwritten). Zero-heap; the dynamic
/// analogue of [`super::Matrix4x4::multiply_vector`], and the shape the GPU
/// `coop_gemv` decode kernel computes per output row.
pub fn matvec(
    transa: Transpose,
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
        match transa {
            Transpose::No => a[i * n + j],
            Transpose::Yes => a[j * m + i],
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

/// Transpose the `m×n` row-major matrix `a` into the `n×m` row-major buffer `out`.
/// Caller-owned, zero-heap. `a` is `m*n`, `out` is `n*m`.
pub fn transpose(m: usize, n: usize, a: &[f64], out: &mut [f64]) -> Result<(), SolversError> {
    if a.len() != m * n || out.len() != m * n {
        return Err(SolversError::InvalidDimension);
    }
    for i in 0..m {
        for j in 0..n {
            out[j * m + i] = a[i * n + j];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len());
        for i in 0..a.len() {
            assert!((a[i] - b[i]).abs() < 1e-9, "idx {i}: {} != {}", a[i], b[i]);
        }
    }

    #[test]
    fn matmul_rectangular() {
        // A (2×3) · B (3×2) = C (2×2)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let mut c = [0.0; 4];
        matmul(2, 3, 2, &a, &b, &mut c).unwrap();
        // [[1*7+2*9+3*11, 1*8+2*10+3*12],[4*7+5*9+6*11, 4*8+5*10+6*12]]
        approx(&c, &[58.0, 64.0, 139.0, 154.0]);
    }

    #[test]
    fn matmul_identity_is_noop() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let id = [1.0, 0.0, 0.0, 1.0];
        let mut c = [0.0; 4];
        matmul(2, 2, 2, &a, &id, &mut c).unwrap();
        approx(&c, &a);
    }

    #[test]
    fn gemm_alpha_beta_accumulate() {
        // C := 2·A·B + 3·C
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 0.0, 0.0, 1.0];
        let mut c = [1.0, 1.0, 1.0, 1.0];
        gemm(
            Transpose::No,
            Transpose::No,
            2,
            2,
            2,
            2.0,
            &a,
            &b,
            3.0,
            &mut c,
        )
        .unwrap();
        // 2·A + 3·C0 = 2·[1,2,3,4] + 3·[1,1,1,1]
        approx(&c, &[5.0, 7.0, 9.0, 11.0]);
    }

    #[test]
    fn gemm_beta_zero_ignores_garbage() {
        // beta=0 must not read c (here pre-filled with NaN-ish garbage).
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 0.0, 0.0, 1.0];
        let mut c = [f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        gemm(
            Transpose::No,
            Transpose::No,
            2,
            2,
            2,
            1.0,
            &a,
            &b,
            0.0,
            &mut c,
        )
        .unwrap();
        approx(&c, &a);
    }

    #[test]
    fn gemm_transpose_a_normal_equations() {
        // AᵀA for A (3×2): expect symmetric 2×2. (op(A)=Aᵀ is 2×3, op(A)=A... )
        // m=2, n=2, k=3: C = Aᵀ(2×3) · A(3×2).
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3×2, row-major
        let mut c = [0.0; 4];
        gemm(
            Transpose::Yes,
            Transpose::No,
            2,
            2,
            3,
            1.0,
            &a,
            &a,
            0.0,
            &mut c,
        )
        .unwrap();
        // AᵀA = [[1+9+25, 2+12+30],[2+12+30, 4+16+36]] = [[35,44],[44,56]]
        approx(&c, &[35.0, 44.0, 44.0, 56.0]);
    }

    #[test]
    fn gemm_transpose_b() {
        // A (2×3) · Bᵀ where B is (2×3) → C (2×2)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0]; // 2×3
        let mut c = [0.0; 4];
        gemm(
            Transpose::No,
            Transpose::Yes,
            2,
            2,
            3,
            1.0,
            &a,
            &b,
            0.0,
            &mut c,
        )
        .unwrap();
        // op(B) = Bᵀ (3×2): rows of A dotted with rows of B.
        // C[0][0]=1*1+2*0+3*1=4 ; C[0][1]=1*0+2*1+3*0=2
        // C[1][0]=4*1+5*0+6*1=10; C[1][1]=4*0+5*1+6*0=5
        approx(&c, &[4.0, 2.0, 10.0, 5.0]);
    }

    #[test]
    fn matvec_basic() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2×3
        let x = [1.0, 1.0, 1.0];
        let mut y = [0.0; 2];
        matvec(Transpose::No, 2, 3, &a, &x, &mut y).unwrap();
        approx(&y, &[6.0, 15.0]);
    }

    #[test]
    fn matvec_transposed_matches_dense_transpose() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2×3
        let x = [1.0, 1.0];
        let mut y = [0.0; 3];
        // op(A) = Aᵀ (3×2) · x(2)
        matvec(Transpose::Yes, 3, 2, &a, &x, &mut y).unwrap();
        // Aᵀ rows: [1,4],[2,5],[3,6] dotted with [1,1] = 5,7,9
        approx(&y, &[5.0, 7.0, 9.0]);
    }

    #[test]
    fn transpose_roundtrip() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2×3
        let mut t = [0.0; 6];
        transpose(2, 3, &a, &mut t).unwrap();
        approx(&t, &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]); // 3×2
        let mut back = [0.0; 6];
        transpose(3, 2, &t, &mut back).unwrap();
        approx(&back, &a);
    }

    #[test]
    fn gemm_via_transpose_equals_matvec() {
        // A·x as a GEMM with n=1 must equal matvec.
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2×3
        let x = [2.0, 0.0, 1.0];
        let mut y_mv = [0.0; 2];
        matvec(Transpose::No, 2, 3, &a, &x, &mut y_mv).unwrap();
        let mut y_gemm = [0.0; 2];
        gemm(
            Transpose::No,
            Transpose::No,
            2,
            1,
            3,
            1.0,
            &a,
            &x,
            0.0,
            &mut y_gemm,
        )
        .unwrap();
        approx(&y_mv, &y_gemm);
    }

    #[test]
    fn rejects_bad_dims() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0];
        let mut c = [0.0; 4];
        assert_eq!(
            matmul(2, 2, 2, &a, &b, &mut c),
            Err(SolversError::InvalidDimension)
        );
    }
}
