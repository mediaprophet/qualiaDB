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

    // ── Best-path-with-CPU-floor offload (additive; behaviour-preserving) ──────────
    //
    // The capability-aware forge dispatcher (`wgsl_forge::dispatch::gemm_f64`) picks
    // the best compute path *actually present on this machine* for the raw product
    // `P = op(A)·op(B)`, while always keeping this CPU triple-loop as the floor. We hand
    // off the product whenever:
    //   1. an accelerator actually exists (`caps().cuda || caps().wgpu`) — so a no-GPU
    //      machine takes the EXACT existing CPU path below with zero behaviour change;
    //   2. the problem is large enough (`m·n·k >= GEMM_GPU_THRESHOLD`) to amortise
    //      dispatch/transfer overhead — small GEMMs stay on the CPU.
    //
    // `gemm_f64` computes only the *plain* row-major product (no alpha/beta/transpose),
    // so the full BLAS shape is reconstructed around it:
    //   • a transpose flag is honoured by *materialising* that operand once into a
    //     row-major scratch buffer (`op(A)` as `m×k`, `op(B)` as `k×n`) — an O(m·k)/O(k·n)
    //     copy dominated by the O(m·n·k) GEMM being offloaded. This unlocks the hot
    //     covariance `XᵀX` (PCA/ridge/linear) and attention `Q·Kᵀ`.
    //   • `alpha`/`beta` are applied in the O(m·n) CPU combine afterward, exactly as the
    //     loop below would (`beta == 0` is a hard zero, so `c` is not read — BLAS rule).
    // The materialised operand equals exactly what `a_at`/`b_at` would read, and the
    // dispatcher's CPU floor uses the same increasing-index accumulation as the loop
    // below, so the answers agree to f64 summation precision.
    //
    // f64 on the GPU is **CUDA-only**: WGSL has no `f64`, so for a non-NVIDIA GPU the
    // dispatcher itself falls back to its own f64 CPU floor (the `caps().wgpu` term just
    // means "an accelerator is present"; the actual f64 GPU kernel is native CUDA-f64).
    // Crucially, a forge error is NEVER propagated out of the solver: on `Err(_)` we fall
    // through to the CPU path below, which is always correct. Sub-threshold or
    // off-accelerator, the unchanged CPU code runs — byte-identical to before.
    // GPU best-path GEMM via the forge — only when it's compiled in (native +
    // wgsl-forge). On wasm32 the forge module doesn't exist; the CPU floor below runs.
    #[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
    {
        use crate::wgsl_forge::dispatch::{caps, GEMM_GPU_THRESHOLD};
        use std::borrow::Cow;
        let work = m.saturating_mul(n).saturating_mul(k);
        let caps = caps();
        if (caps.cuda || caps.wgpu) && work >= GEMM_GPU_THRESHOLD {
            // op(A): row-major m×k — stored m×k already (No) or k×m (Yes → transpose).
            let a_eff: Cow<[f64]> = match transa {
                Transpose::No => Cow::Borrowed(a),
                Transpose::Yes => {
                    let mut t = vec![0.0_f64; m * k];
                    for i in 0..m {
                        for l in 0..k {
                            t[i * k + l] = a[l * m + i];
                        }
                    }
                    Cow::Owned(t)
                }
            };
            // op(B): row-major k×n — stored k×n already (No) or n×k (Yes → transpose).
            let b_eff: Cow<[f64]> = match transb {
                Transpose::No => Cow::Borrowed(b),
                Transpose::Yes => {
                    let mut t = vec![0.0_f64; k * n];
                    for l in 0..k {
                        for j in 0..n {
                            t[l * n + j] = b[j * k + l];
                        }
                    }
                    Cow::Owned(t)
                }
            };
            // BLAS gemm dims are (m, n, k); the dispatcher takes (m, k, n): op(A) is m×k,
            // op(B) is k×n, C is m×n, so the mapping is gemm(m,n,k) → dispatch(m,k,n).
            if let Ok(product) = crate::wgsl_forge::dispatch::gemm_f64(m, k, n, &a_eff, &b_eff) {
                // Apply alpha/beta exactly as the CPU loop would (beta==0 ⇒ c not read).
                if beta == 0.0 {
                    for (ci, &p) in c.iter_mut().zip(product.iter()) {
                        *ci = alpha * p;
                    }
                } else {
                    for (ci, &p) in c.iter_mut().zip(product.iter()) {
                        *ci = alpha * p + beta * *ci;
                    }
                }
                return Ok(());
            }
            // Forge path was eligible but errored — fall through to the CPU floor.
        }
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

    // ── Best-path-with-CPU-floor offload (additive; behaviour-preserving) ──────────
    //
    // Mirror of the [`gemm`] fast-path above. The forge dispatcher
    // (`wgsl_forge::dispatch::gemv_f64`) computes the *plain* product `y = A·x` only
    // (it has no transpose support), so we only hand off when ALL hold:
    //   1. it IS the plain product — `transa == Transpose::No` — because `gemv_f64`
    //      cannot express `Aᵀ·x`;
    //   2. an accelerator actually exists (`caps().cuda || caps().wgpu`) — so a no-GPU
    //      machine takes the EXACT existing CPU loop below with zero behaviour change;
    //   3. the problem is large enough (`m·n >= GEMM_GPU_THRESHOLD`) to amortise
    //      dispatch/transfer overhead — small GEMVs stay on the CPU.
    //
    // Dimensions map directly: here `A` is m×n (row-major), `x` is length n, `y` is
    // length m — exactly `gemv_f64(m, n, a, x)`'s `y[M] = A[M×N]·x[N]` contract, so no
    // re-mapping is needed (unlike gemm's (m,n,k)→(m,k,n) swap). f64 on the GPU is
    // CUDA-only (WGSL has no f64); on a non-NVIDIA GPU the dispatcher itself falls back
    // to its own f64 CPU floor, whose increasing-`j` summation order matches this loop,
    // so answers agree to f64 summation precision.
    //
    // A forge error is NEVER propagated: on `Err(_)` we fall through to the CPU loop
    // below, which is always correct. Any transpose, sub-threshold size, or
    // off-accelerator run executes the unchanged CPU code — byte-identical to before.
    #[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
    if transa == Transpose::No {
        use crate::wgsl_forge::dispatch::{caps, GEMM_GPU_THRESHOLD};
        let work = m.saturating_mul(n);
        let caps = caps();
        if (caps.cuda || caps.wgpu) && work >= GEMM_GPU_THRESHOLD {
            if let Ok(result) = crate::wgsl_forge::dispatch::gemv_f64(m, n, a, x) {
                y.copy_from_slice(&result);
                return Ok(());
            }
            // Forge path was eligible but errored — fall through to the CPU floor.
        }
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
    fn plain_gemm_matches_cpu_above_threshold() {
        // A large plain product (48×48×48 = 110_592 ≥ GEMM_GPU_THRESHOLD) routed
        // through `gemm(No, No, .., 1.0, a, b, 0.0, c)`. On an accelerator box this
        // exercises the forge offload; on CI (no accelerator) it exercises the CPU
        // floor. EITHER WAY the result must equal a SEPARATE pure-CPU triple-loop
        // reference computed inline here, so the test is hermetic and GPU-agnostic.
        const M: usize = 48;
        const K: usize = 48;
        const N: usize = 48;

        // Deterministic data — no RNG, no GPU assumption.
        let mut a = vec![0.0f64; M * K];
        let mut b = vec![0.0f64; K * N];
        for i in 0..M {
            for l in 0..K {
                a[i * K + l] = ((i * 7 + l * 3) % 11) as f64 * 0.25 - 1.0;
            }
        }
        for l in 0..K {
            for j in 0..N {
                b[l * N + j] = ((l * 5 + j * 2) % 13) as f64 * 0.125 - 0.75;
            }
        }

        // Reference: independent triple loop, increasing-k accumulation order.
        let mut reference = vec![0.0f64; M * N];
        for i in 0..M {
            for j in 0..N {
                let mut acc = 0.0f64;
                for l in 0..K {
                    acc += a[i * K + l] * b[l * N + j];
                }
                reference[i * N + j] = acc;
            }
        }

        // Through the wired gemm (plain product → eligible for offload).
        let mut c = vec![0.0f64; M * N];
        gemm(
            Transpose::No,
            Transpose::No,
            M,
            N,
            K,
            1.0,
            &a,
            &b,
            0.0,
            &mut c,
        )
        .unwrap();

        approx(&c, &reference);
    }

    #[test]
    fn plain_gemv_matches_cpu_above_threshold() {
        // A large plain matrix–vector product (m=n=256 → work = 65_536 ≥
        // GEMM_GPU_THRESHOLD=32_768) routed through `matvec(No, ..)`. On an
        // accelerator box this exercises the forge offload (`dispatch::gemv_f64`);
        // on CI (no accelerator) it exercises the CPU floor. EITHER WAY the result
        // must equal a SEPARATE pure-CPU reference computed inline here, so the test
        // is hermetic and GPU-agnostic.
        const M: usize = 256;
        const N: usize = 256;

        // Deterministic data — no RNG, no GPU assumption.
        let mut a = vec![0.0f64; M * N];
        for i in 0..M {
            for j in 0..N {
                a[i * N + j] = ((i * 7 + j * 3) % 11) as f64 * 0.25 - 1.0;
            }
        }
        let mut x = vec![0.0f64; N];
        for (j, xj) in x.iter_mut().enumerate() {
            *xj = ((j * 5) % 13) as f64 * 0.125 - 0.75;
        }

        // Reference: independent dot-product per row, increasing-j accumulation order.
        let mut reference = vec![0.0f64; M];
        for i in 0..M {
            let row = i * N;
            let mut acc = 0.0f64;
            for j in 0..N {
                acc += a[row + j] * x[j];
            }
            reference[i] = acc;
        }

        // Through the wired matvec (plain product → eligible for offload).
        let mut y = vec![0.0f64; M];
        matvec(Transpose::No, M, N, &a, &x, &mut y).unwrap();

        approx(&y, &reference);
    }

    #[test]
    fn transposed_covariance_matches_cpu_above_threshold() {
        // The hot ML op: covariance `Cov = Xᵀ·X` for tall-skinny `X` (n≫p), expressed
        // as `gemm(Transpose::Yes, Transpose::No, p, p, n, 1.0, x, x, 0.0, cov)`. With
        // n=4096, p=8 the work is p·p·n = 262_144 ≥ GEMM_GPU_THRESHOLD, so on an
        // accelerator box this exercises the NEW transpose-materialising offload path;
        // on CI (no accelerator) it exercises the CPU floor. EITHER WAY the result must
        // equal a SEPARATE pure-CPU `XᵀX` reference computed inline here — hermetic,
        // GPU-agnostic, and the regression guard that materialise-then-dispatch equals
        // the index-arithmetic CPU loop.
        const NSAMP: usize = 4096; // k (contraction dim)
        const P: usize = 8; // m == n (feature dim)

        // X is NSAMP×P row-major (stored as op==Yes's k×m). Deterministic, no RNG.
        let mut x = vec![0.0f64; NSAMP * P];
        for r in 0..NSAMP {
            for c in 0..P {
                x[r * P + c] = ((r * 3 + c * 7) % 17) as f64 * 0.125 - 1.0;
            }
        }

        // Reference: Cov[i][j] = Σ_r X[r][i]·X[r][j], increasing-r accumulation.
        let mut reference = vec![0.0f64; P * P];
        for i in 0..P {
            for j in 0..P {
                let mut acc = 0.0f64;
                for r in 0..NSAMP {
                    acc += x[r * P + i] * x[r * P + j];
                }
                reference[i * P + j] = acc;
            }
        }

        // Through the wired gemm: op(A)=Xᵀ (P×NSAMP), op(B)=X (NSAMP×P) → C (P×P).
        let mut cov = vec![0.0f64; P * P];
        gemm(
            Transpose::Yes,
            Transpose::No,
            P,
            P,
            NSAMP,
            1.0,
            &x,
            &x,
            0.0,
            &mut cov,
        )
        .unwrap();

        approx(&cov, &reference);
        // Covariance must be symmetric.
        for i in 0..P {
            for j in 0..P {
                assert!((cov[i * P + j] - cov[j * P + i]).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn scaled_accumulate_transposed_matches_cpu_above_threshold() {
        // Exercises the offload's `alpha`/`beta` combine *together with* a transpose, on
        // the real PCA covariance shape: `C := alpha·(Xᵀ·X) + beta·C0` with
        // alpha=1/(NSAMP−1) (exactly what `pca::fit` passes) and a non-zero beta so the
        // `c`-read branch is hit. Above threshold (P²·NSAMP ≥ GEMM_GPU_THRESHOLD) → on an
        // accelerator this is the materialise→dispatch→combine path; on CI the CPU floor.
        // Either way it must equal the inline reference.
        const NSAMP: usize = 4096;
        const P: usize = 8;
        let alpha = 1.0 / (NSAMP as f64 - 1.0);
        let beta = 0.5;

        let mut x = vec![0.0f64; NSAMP * P];
        for r in 0..NSAMP {
            for c in 0..P {
                x[r * P + c] = ((r * 5 + c * 3) % 13) as f64 * 0.1 - 0.6;
            }
        }
        // Initial C0 — read because beta != 0.
        let mut c = vec![0.0f64; P * P];
        for (i, ci) in c.iter_mut().enumerate() {
            *ci = (i % 5) as f64 * 0.25;
        }
        let c0 = c.clone();

        // Reference: alpha·Σ_r X[r][i]·X[r][j] + beta·C0[i][j].
        let mut reference = vec![0.0f64; P * P];
        for i in 0..P {
            for j in 0..P {
                let mut acc = 0.0f64;
                for r in 0..NSAMP {
                    acc += x[r * P + i] * x[r * P + j];
                }
                reference[i * P + j] = alpha * acc + beta * c0[i * P + j];
            }
        }

        gemm(
            Transpose::Yes,
            Transpose::No,
            P,
            P,
            NSAMP,
            alpha,
            &x,
            &x,
            beta,
            &mut c,
        )
        .unwrap();
        approx(&c, &reference);
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
