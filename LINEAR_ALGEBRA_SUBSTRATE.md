# Linear-Algebra Substrate — status & contract for agents

**Branch:** `0.0.21-la` (worktree `.worktrees/qualia-la`) · **Maintainer lane:** the modality-first
consolidation (Timothy-directed) · **Last updated:** 2026-06-26

> **Read this if you need matrix/vector math, or if you are about to add some.**
> The point of this work is that dense linear algebra **lives once, in the engine, by modality** —
> not re-implemented inside every "specialized" library. If you need a matmul, a solve, a
> factorisation, or eigenvalues: **call the engine. Do not write your own loop.** If the engine is
> missing what you need, add it *here* (see "How to extend") rather than in a domain lib.

---

## 1. Where it lives

```
crates/qualia-core-db/src/solvers/linear_algebra/
  mod.rs        fixed-size Matrix4x4 / Vector4 / Tensor3x3x3, Lanczos, StaticLuDecomposition (legacy, kept)
  gemm.rs       dynamic GEMM / matvec / transpose  ← the canonical dense-matmul core
  qr.rs         Householder QR + least-squares + square solve
  cholesky.rs   SPD factor / solve / determinant
  eigen.rs      symmetric eigendecomposition (closed-form 3×3 + general Jacobi)
```

All of it is **zero-allocation**: every routine operates on **caller-owned, row-major `&[f64]` /
`&mut [f64]`** slices with explicit dimensions, and **fails closed** on a shape mismatch
(`SolversError::InvalidDimension`) or a singular/ill-posed input (`SolversError::SingularMatrix`,
`InvalidParameters`). No `DMatrix`, no heap, no external dependency (no `nalgebra` — built native per
Timothy's directive).

`SolversError` is defined in `solvers/mod.rs`.

---

## 2. API surface (current)

### `gemm.rs` — the dense-matmul core
| Fn | Contract |
|----|----------|
| `gemm(transa, transb, m, n, k, alpha, a, b, beta, c)` | `C := α·op(A)·op(B) + β·C`, BLAS-3 shape. `op` = `Transpose::{No,Yes}` (transpose by indexing, no operand materialised). `β == 0` is a hard zero (fresh `c` need not be pre-zeroed). |
| `matmul(m, k, n, a, b, c)` | `C := A·B` (thin wrapper, `α=1, β=0`). |
| `matvec(transa, m, n, a, x, y)` | `y := op(A)·x`. |
| `transpose(m, n, a, out)` | `out := Aᵀ`. |

### `qr.rs` — orthogonal factorisation
| Fn | Contract |
|----|----------|
| `qr_factor(m, n, a, tau)` | In-place Householder (LAPACK `geqrf` layout): upper triangle of `a` ← `R`, below-diagonal ← reflectors, `tau` ← scalings. `m ≥ n`. |
| `qr_form_q(m, n, a, tau, q)` | Materialise the thin `Q` (`m×n`), `Q·R_n = A`, `QᵀQ = I`. |
| `qr_solve_least_squares(m, n, a, tau, b, x)` | `min‖A·x − b‖` (square solve when `m=n`). Scale-relative rank check → `SingularMatrix`. |

### `cholesky.rs` — SPD systems
`cholesky_factor(n, a, l)` · `cholesky_solve(n, l, b, x)` · `cholesky_determinant(n, l)`.

### `eigen.rs` — symmetric eigenproblems
| Fn | Contract |
|----|----------|
| `symmetric_eigen_3x3(a) -> [f64;3]` | Closed-form (Smith's), eigenvalues **descending**. For principal stresses/strains. |
| `symmetric_eigen(n, a, eigvecs)` | Cyclic Jacobi, in-place: `a`'s diagonal ← eigenvalues, `eigvecs` columns ← eigenvectors. |

---

## 3. The GPU parity contract (for the LLM / gguf lane)

`gemm::gemm` is **the CPU reference** for the GPU `coop_gemv` decode kernel
(`gguf_bridge`/`shaders/fused_transformer.wgsl`). They compute the same thing. The existing
`gemm_parity_probe` checks GPU output against a CPU GEMM to `max_abs_err ~1e-5`. When the unified-math
substrate work routes the LLM runtime through the substrate (plan P3, **gated on Timothy's explicit
go**), the GPU backend is the *promoted* `coop_gemv`/resident-weights path — **promoted in place, never
rewritten** — and parity is enforced against this CPU code, byte-identical, no tok/s regression.

Until P3 is authorised, **do not touch `gguf_bridge/`** for this — it is a live perf lane worked by
another instrument. See `UNIFIED_MATH_SUBSTRATE_PLAN.md`.

---

## 4. Status — what is done

Real cross-silo **duplicates removed** (the actual goal):
- ✅ `specialized_libs/linear_algebra` dynamic matmul → `gemm` (inline triple-loop deleted; latent
  `α·AB·(1+β)` β-bug fixed).
- ✅ `specialized_libs/linear_algebra` `solve_linear_system` (was inline Gauss-Jordan) → `qr`.
- ✅ **eigendecomposition unified**: `linear_algebra::eigen_symmetric` (Jacobi) **and**
  `engineering_analysis::principal_stresses` (closed-form 3×3) — two implementations of one operation
  — both now call `eigen.rs`.

Honest scope finding (mirrors the earlier statistics triage): the remaining "math" in the specialized
libs is **thinner than the heap-counts imply**.
- `machine_learning.rs` — stub scaffold (`output_data: vec![1u8;100]`), no real GEMM/gradient.
- `chemistry`/`physics`/`engineering` — **no generic ODE integrators** (no RK4/Euler loops); the real
  kinetics/thermo work is closed-form analytic.
- `lu_decompose` / `determinant` / `svd` / `characteristic_polynomial` / `eigenvalues_general` /
  `polynomial_roots` (+ `Complex`/`Lu`/`Svd`) in `specialized_libs/linear_algebra.rs` are **single-copy
  and correct** — just *located* in the silo. Relocating them to the engine carries
  `LinearAlgebraError`→`SolversError` churn that touches `mcp/mcp_tool_impls.rs`, so per CLAUDE.md §11
  it is **deferred to the dedicated library-ization pass**, not done mid-feature.

## 5. Status — in progress / planned (this lane)

- ▶ doing  Native **nalgebra-gap** build-out (no dependency): Givens rotations → general (non-symmetric)
  eigenvalues via Hessenberg + QR-iteration → SVD. Net-new engine capability; also the homes the
  silo's `svd`/`eigenvalues_general` will route to later.
- ⏸ deferred (→ library-ization pass)  relocate single-copy `lu`/`det`/`svd`/polynomial cluster to the
  engine + hollow the silo wrappers (error-type conversion, touches `mcp`).
- 🚦 gated (→ Timothy GO + NOTICES check)  **P3**: route the LLM/gguf math through this substrate.

## 6. How to extend (the rule)

1. New dense-LA capability goes in a **focused submodule** of `solvers/linear_algebra/` (`foo.rs` with
   its own `#[cfg(test)]`), wired via `mod.rs` — never a growing monolith (CLAUDE.md §11).
2. **Zero-heap, caller-owned slices, fail-closed.** Match the idiom in `cholesky.rs`/`gemm.rs`.
3. A specialized/domain lib must **call** the engine and only marshal its domain types at the boundary
   — it must not carry its own matmul/solve/factorisation. Direction of flow is **engine-outward**.
4. Log each step in `MODALITY_FIRST_CONSOLIDATION.md` and a `NOTICES.md` line.

---

*Tracking doc: `MODALITY_FIRST_CONSOLIDATION.md` · Substrate plan: `UNIFIED_MATH_SUBSTRATE_PLAN.md`.*
