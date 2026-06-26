# Modality-First Consolidation — tracking

**Branch:** `0.0.21-modality-first` · **Started:** 2026-06-26 · **Directed by:** Timothy

## The rule (why this exists)

The "specialized" libraries grew their own private re-implementations of math and
logic that the engine already owns — a *fork of authority*. Every duplicated
`mean`/`matmul`/`solve` competes with the engine, can't be optimized uniformly,
loses provenance, and (measured) bleeds RAM/cycles by copying data out of the
canonical layout into private heap structures (~2,596 `Vec`/`HashMap`/`Arc`/`Mutex`/`String`
uses across 20 specialized-lib files at the start of this work).

**One logic engine is the single source of truth.** Specialized/domain libraries are
**composition only**: marshal domain parameters into a slice → call the engine →
translate results back to domain/RDF concepts. They MUST NOT carry their own
foundational math or logic.

**Direction of flow is engine-outward, never silo-inward.** Do *not* merge a
specialized lib's heap code *into* the engine (that contaminates the zero-heap source
of truth). Build a clean zero-heap home in the engine, then hollow the silo to *call* it.

## Canonical homes (route to these)

| Nature of the logic | Canonical home |
|---|---|
| Statistics (mean/var/median/correlation/tests/histogram) | `solvers/statistics/` ← **new** |
| Algebra / matrix / tensor | `solvers/linear_algebra/` |
| Calculus / ODE / integration / differential | `solvers/calculus/` |
| Optimization / root-find / curve-fit | `solvers/optimization/` |
| Symbolic / CAS / SAT / defeasible | `solvers/symbolic_logic/` |
| Probabilistic **logic** / Bayesian (not numeric stats) | `modalities/probabilistic.rs` |
| Deontic / jural | `modalities/deontic.rs`, `modalities/jural.rs` |
| Crypto primitives | `crypto/` (`fiduciary_crypto`, cryptographic primitives) |
| Hardware-accelerated compute | **vendor-neutral wgpu / `hetero_dispatch`** |

### ⚠ Two standing carve-outs
- **Hardware dispatch is NOT the QPU path.** The pasted directive routed batching
  through `qpu_dispatcher.rs`; the principle (one engine → execution graph → batch to
  GPU/NPU) is right, but the *live, measured* arbiter on this repo is the vendor-neutral
  wgpu / `hetero_dispatch` stack. Harmonize toward that.
- **QPU / quantum is DEPRIORITIZED** (standing rule, WAP §0.11). Do **not** consolidate,
  rewrite, or "unify" `specialized_libs/qpu_bridge/`, `specialized_libs/quantum_biology/`,
  `solvers/qpu/`, or `solvers/quantum_optimizers/`. Frozen until Timothy lifts it.

## Laundry list (per specialized lib)

Status: ☐ not started · ◑ partial · ☑ done. Heap-count = `Vec/HashMap/Arc/Mutex/String` uses at audit start.

| Lib | heap | Re-implements | Route to | Status |
|---|---|---|---|---|
| `statistical_computing.rs` | 231 | mean, median, variance, correlation, t-test, histogram, Laplace noise | `solvers/statistics/` | ☑ all numeric kernels rerouted (Laplace DP noise stays — it's domain privacy policy, not a math kernel) |
| `linear_algebra/` + `.rs` | 22 (+dir) | matmul, transpose, decompositions (dynamic, heap) | `solvers/linear_algebra/` (needs a dynamic, caller-owned GEMM entry added — engine currently only has fixed-size `Matrix4x4`/Lanczos) | ☐ |
| `symbolic_algebra.rs` | 32 | polynomial/CAS/simplify | `solvers/symbolic_logic/` | ☐ |
| `machine_learning.rs` | 268 | GEMM, gradient descent, stats | `solvers/linear_algebra` + `optimization` + `statistics` | ☐ |
| `medical_computing/` | 630 | descriptive stats, linear fits | `solvers/statistics` + `linear_algebra`; domain stays | ☐ |
| `financial_modeling/` | 388 | stats, NPV/IRR (root-find), regression | `solvers/statistics` + `optimization` | ☐ |
| `physics_simulation.rs` | 139 | ODE integration, vector math | `solvers/calculus` + `linear_algebra` | ☐ |
| `engineering_analysis.rs` | 196 | stress/fatigue numerics, linear solves | `solvers/linear_algebra` + `calculus` + `statistics` | ☐ |
| `chemistry_modeling.rs` | 167 | reaction kinetics (ODE), linear algebra | `solvers/calculus` + `linear_algebra` | ☐ |
| `cryptographic_library/` | 360 | (mostly real already; audit for re-impl vs `crypto/`) | `crypto/` | ☐ audit |
| `qpu_bridge/`, `quantum_biology/` | — | — | **FROZEN — QPU deprioritized** | ⏸ |

### Cross-cutting follow-ups
- ☐ Inject the rule into root AI-instruction files (`.cursorrules`, `AI_INSTRUCTIONS.md`)
  as the anti-recurrence mechanism (the strict directive).
- ☐ Non-blocking CI scan: flag inline math loops in `specialized_libs/` that don't call
  the `solvers`/`modalities` namespace (Phase 3 enforcement — report first, gate later).
- ⚑ **Engine coverage gap:** `solvers/linear_algebra` is fixed-size (`Matrix4x4`, Lanczos)
  only. The specialized libs do *dynamic*-size matmul. Consolidating LA needs a zero-heap,
  caller-owned-buffer **dynamic GEMM** added to the engine first — it's not a pure rewire.

---

## Progress log

### 2026-06-26 — Step 1: statistics home + first reroute · **done (green)**
- **Built** `solvers/statistics/` (new canonical home): `mod.rs` + `descriptive.rs` —
  zero-allocation kernels over caller-owned slices (`mean`, `variance{sample,pop}`,
  `std_dev`, `median_sorted`, `median_in_place` via non-allocating `sort_unstable_by`,
  `min`, `max`, `sum`). Each is a focused single function with its own test.
- **Rerouted** `statistical_computing.rs` `mean()`, `median()`, `variance()` to call the
  engine (the wrapper keeps marshalling its heap `Dataset` — that's the MCP composition
  boundary — but the math is now the engine's). Deleted the inline kernels.
- **Wired** `pub mod statistics;` into `solvers/mod.rs`.
- **Measured:** `cargo test -p qualia-core-db --lib solvers::statistics::` → **6 passed**;
  `statistical_computing::` → **8 passed** (reroute behaviour-preserving). Non-test
  `cargo build -p qualia-core-db --lib` green (48.9s). Caveat: the crate's *test harness*
  emits transient incremental-compile errors in unrelated webizen/identity modules on a
  cold incremental pass (0 on the next pass) — a cargo wart from the in-flight reorg, not
  this change; the stable state is clean.
- **⚑ Needs Timothy:** (1) confirm the QPU carve-out above (the pasted directive points at
  the quantum dispatcher you've deprioritized — I've routed hardware to the wgpu path
  instead). (2) The LA consolidation needs a dynamic GEMM added to the engine first
  (coverage gap above) — OK to build that as the next home? (3) Want the rule injected into
  `.cursorrules`/`AI_INSTRUCTIONS.md` now, or after more migrations land?
- **Next:** finish `statistical_computing` (correlation, t_test, histogram → `solvers/statistics`),
  then `machine_learning`/`medical`/`financial` stats consumers (they all want the same home).

### 2026-06-26 — Step 2: finish statistical_computing · **done (green)**
- **Built** three more engine homes in `solvers/statistics/`: `correlation.rs`
  (`pearson`, `kendall`, `rank_into` — ranking writes into caller-owned scratch),
  `hypothesis.rs` (`one_sample_t` + `TTest`, reusing the descriptive mean/variance),
  `histogram.rs` (`histogram_into` filling a caller-owned counts buffer + `HistRange`).
- **Rerouted** the wrapper's `pearson_correlation`, `kendall_correlation`, `rank_values`,
  `one_sample_t_test`, `compute_histogram` to call the engine. All inline math removed
  from `statistical_computing.rs`; the wrapper now only marshals its heap `Dataset` and
  owns scratch/output buffers (the legitimate composition boundary).
- **⚑ Behaviour change (correctness, flagged not hidden):** the old `rank_values`
  tie-averaging was *buggy* (gave `[1,2,2,…]` for tied data instead of `[1,2.5,2.5,4]`).
  The engine's `rank_into` does proper average ranks, so **Spearman correlation on tied
  data now returns the correct value**. No existing test asserted the old wrong value, so
  nothing broke — but downstream consumers that memorised old Spearman numbers should know.
- **Measured:** `solvers::statistics::` → **18 passed**; `statistical_computing::` → **8 passed**.
- **Next:** `machine_learning` / `medical_computing` / `financial_modeling` stats consumers
  → same `solvers/statistics` home; then the LA dynamic-GEMM home (coverage gap above).

### 2026-06-26 — Stats column triage (honest finding)
Drained the rest of the stats duplication and found it's **thin**, not a big seam:
- `machine_learning.rs` — orchestration **scaffold over stubs** (`execute_inference` returns
  hardcoded `output_data: vec![1u8;100], confidence: 0.95`). No real math kernels to migrate.
- `financial_modeling` — has `normal_cdf`/`normal_pdf` (a genuine stats primitive) → routed
  to a new `solvers/statistics/distributions.rs`; the rest is domain Black-Scholes.
- `medical_computing` — no descriptive-stats math to migrate.
So `statistical_computing` + financial's Gaussian were the real statistics duplication.

### 2026-06-26 — geometric_algebra → solvers/ + simd_kernel split · **done (green)** · worktree `0.0.21-la`
- **Moved** `geometric_algebra` (numeric GA: rotors/multivectors/geometric product — a
  *solver*, not a logic modality) into `solvers/geometric_algebra/`; path-preserved via a
  `crate::geometric_algebra` re-export in `lib.rs` (mcp/n3logic/wasm_playground untouched).
- **Split** the 595-line `simd_kernel.rs` monolith into a library (§11): `simd_backend.rs`
  (AVX2/FMA + `[f32;8]` kernels), `types.rs` (Multivector/Rotor/Translator), `operations.rs`
  (grade-aware products, rotor/translator apply), `mod.rs` re-exporting verbatim.
- **Measured:** `solvers::geometric_algebra::` → **14 passed**; lib builds clean.

### 2026-06-26 — nalgebra review + LA phase 1 (Cholesky) · **done (green)** · worktree `0.0.21-la`
- **Reviewed** nalgebra @5f927f6c vs qualia. Gaps: QR (Householder+col-pivot), **Cholesky**,
  Givens/Householder primitives, Hessenberg→Schur, sym-tridiagonal, bidiagonal→SVD, exp/pow,
  fast 2×2/3×3 SVD; geometry types (Rotation3, unit-quaternion+slerp, Isometry/Similarity,
  perspective/orthographic projection, dual-quaternion). Plan: native, zero-heap, caller-owned,
  **no dependency** — this is also the dynamic-LA home the consolidation needs.
- **Built** `solvers/linear_algebra/cholesky.rs` — SPD factor + solve + determinant over
  caller-owned row-major slices, fails closed on non-SPD. **6 tests pass.**
- **Isolation:** all of this is on worktree `.worktrees/qualia-la` (branch `0.0.21-la`) off the
  committed stats work, on its own target dir — clean of the main tree's uncommitted churn (the
  `uuid`/`wgpu::Maintain` breakage in `services`/`lora` is *not* committed; confirmed by a clean
  build here).
- **Next (LA phase 1 cont.):** Householder + Givens primitives → QR (+ least-squares). Then
  geometry types beside `geometric_algebra` (extend its rotors/quaternions, don't duplicate).
- **⚑ Needs Timothy:** the geometry types will *extend* `geometric_algebra`'s existing
  rotor/quaternion — confirm that's the intended home (vs a separate `solvers/geometry/`).

### 2026-06-26 — LA phase 2: dynamic GEMM core · **done (green)** · worktree `0.0.21-la`
- **Built** `solvers/linear_algebra/gemm.rs` — the **one** dynamic dense GEMM the engine was
  missing (the coverage gap the consolidation flagged: prior LA was fixed-size `Matrix4x4`/Lanczos
  only). BLAS-3 shape `C := α·op(A)·op(B) + β·C` with a `Transpose` enum (transpose by index
  arithmetic, no operand materialised), plus `matmul`, `matvec`, `transpose`. **Zero allocation,
  caller-owned row-major slices, fail-closed** on dimension mismatch (`InvalidDimension`); `β==0`
  honoured as a hard zero (fresh output need not be pre-zeroed, BLAS rule).
- **Why this shape:** it is simultaneously (a) the nalgebra-parity dynamic matmul, (b) the home the
  `specialized_libs` heap GEMMs route to, and (c) the **CPU parity reference** the GPU `coop_gemv`
  decode kernel is checked against (`gemm_parity_probe`) — one contract, three consumers, one
  implementation.
- **Measured:** `solvers::linear_algebra::` → **22 passed** (11 new GEMM: rectangular, identity,
  α/β accumulate, β=0-ignores-garbage, AᵀA normal-equations, Bᵀ, matvec ±transpose, transpose
  round-trip, gemm≡matvec, bad-dims reject). Lib builds clean.
- **Next:** route `specialized_libs/linear_algebra` dynamic matmul → this `gemm`/`matmul` (hollow
  the first LA silo); then Householder/Givens → QR (+ least-squares via the `Transpose::Yes` normal
  equations already proven here).
- **⚑ Needs Timothy:** none this step.

### 2026-06-26 — LA phase 3: hollow the first LA silo (matmul reroute) · **done (green)** · worktree `0.0.21-la`
- **Rerouted** `specialized_libs/linear_algebra/computation.rs::execute_multiplication` (the public
  `LinearAlgebraLibrary::matrix_multiply` GEMM) to call the engine `solvers::linear_algebra::gemm`.
  The inline triple-loop is deleted; the wrapper now only marshals its heap `Matrix` operands into
  the caller-owned buffer (the legitimate composition boundary) and calls the engine.
- **⚑ Correctness fix (flagged not hidden):** the old inline loop did
  `result[i] += beta * result[i]` *after* computing the product into the freshly-zeroed `result`,
  i.e. it returned `α·AB·(1+β)` instead of BLAS `α·AB + β·C`. Since this entry point always
  allocates a fresh zeroed output (there is no prior C), the engine path now returns the correct
  `α·AB` (β·0 = 0). Any caller that passed `beta != 0` and memorised the old inflated value will see
  a corrected number; no test asserted the old wrong value.
- **Measured:** `cargo test -p qualia-core-db --lib linear_algebra` → **42 passed** (engine 22 +
  specialized 20, incl. the rerouted `test_matrix_multiplication`).
- **Not yet routable:** the dynamic determinant / inverse / solve / eigen / SVD in
  `specialized_libs/linear_algebra.rs` need engine homes that don't exist yet (QR/eigen/SVD are the
  nalgebra-parity gaps) — they reroute after those are built. Sequencing is correct: build the
  engine home, then hollow.
- **Next:** Householder + Givens → QR (+ least-squares) in `solvers/linear_algebra`.
- **⚑ Needs Timothy:** none this step.

### 2026-06-26 — LA phase 4: Householder QR + least-squares, reroute square solve · **done (green)** · worktree `0.0.21-la`
- **Built** `solvers/linear_algebra/qr.rs` — the engine's missing stable factorisation (prior LA had
  only fixed `Matrix4x4` LU + the silo's heap routines). Householder `qr_factor` (in-place LAPACK
  `geqrf` layout: R in the upper triangle, reflectors below, scalings in `tau`), `qr_form_q` (thin
  `Q`), and `qr_solve_least_squares` (applies `Qᵀ`, back-substitutes `R`). **Zero allocation,
  caller-owned, fail-closed** with a **scale-relative** rank check (a pivot small vs the largest `R`
  diagonal → `SingularMatrix`, not a divide-by-~0). Solves square systems *and* overdetermined
  least-squares — numerically stable, no `AᵀA` conditioning blow-up.
- **Rerouted** `specialized_libs/linear_algebra.rs::solve_linear_system` (was an inline Gauss-Jordan
  duplicate) → engine QR. Inline elimination deleted; the wrapper marshals into caller-owned buffers
  and calls the engine. Same solution for nonsingular systems; both fail closed on singular (QR uses
  the relative tol, old used an absolute `1e-10` pivot).
- **Measured:** `solvers::linear_algebra::qr::` → **8 passed** (square + tall reconstruct `Q·R==A`,
  `QᵀQ==I`, exact + noisy line-fit least-squares with hand-checked normal-equations values, rank-
  deficient fail-closed, bad-dims). Full `linear_algebra` filter → **50 passed**, no regressions.
- **Next:** relocate the dynamic `lu_decompose`/`determinant`/`eigen_symmetric`/`svd` free functions
  (currently misplaced in `specialized_libs/linear_algebra.rs`, Vec-based) into the engine
  `solvers/linear_algebra` as their canonical home, then hollow the silo's `determinant`/
  `matrix_inverse` methods to call them. Then Givens rotations (for selective zeroing / RQ).
- **⚑ Needs Timothy:** none this step.

### 2026-06-26 — LA phase 5: eigendecomposition unified (a REAL cross-silo duplicate killed) · **done (green)** · worktree `0.0.21-la`
- **The duplication:** symmetric-eigenvalue math existed in **two** silos —
  `specialized_libs/linear_algebra::eigen_symmetric` (cyclic Jacobi) and
  `specialized_libs/engineering_analysis::principal_stresses` (closed-form 3×3 Smith's algorithm).
  One operation, two implementations. This is exactly the pathology the consolidation targets.
- **Built** `solvers/linear_algebra/eigen.rs` — the engine's single home: `symmetric_eigen_3x3`
  (closed-form, zero-heap, descending) and `symmetric_eigen` (general Jacobi, in-place caller-owned
  buffers, yields eigenvectors). Symmetric-only, fail-closed (`InvalidParameters` on asymmetry).
- **Rerouted BOTH consumers** to the engine: `eigen_symmetric` now marshals into caller-owned buffers
  and calls `symmetric_eigen` (inline Jacobi deleted; exact error messages + eigenvector/eigenvalue
  semantics preserved); `principal_stresses` now flattens the stress tensor and calls
  `symmetric_eigen_3x3` (inline closed-form deleted). The closed-form is the *same* math, now owned
  once by the engine.
- **Measured:** `eigen` filter → **9 passed** (6 engine: closed-form diagonal/known/≡Jacobi,
  Jacobi eigenvector-reconstruction A·v=λv, asymmetry + bad-dims fail-closed; 3 silo rerouted).
  `engineering_analysis` → **13 passed** (uniaxial + pure-shear stress states exercise the rerouted
  `principal_stresses`). Behaviour-preserving.
- **⚠ Process note (honest):** the shell cwd silently drifted to the **main tree** mid-step, so the
  first eigen test run built main-tree code (no `eigen.rs`) and reported 0 tests — a false pass-shaped
  result. Caught it, re-ran inside the worktree → real 9/13 green. Earlier gemm/qr/solve runs were
  unaffected (their worktree-only `qr` tests had passed, which is only possible in the worktree).
  Going forward: `cd` into the worktree explicitly each run.
- **Next:** relocate the remaining dynamic `lu_decompose`/`determinant`/`svd` to the engine (same
  pattern), then route `chemistry`/`physics` ODE math to `solvers/calculus`.
- **⚑ Needs Timothy:** none this step.
