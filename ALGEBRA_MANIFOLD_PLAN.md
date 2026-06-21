# Plan — Algebra breadth + the wgpu 10D manifold

Tracks LINEAR_ALGEBRA_ZK_TODO.md §2 (algebra breadth) and §3 (manifold). Timothy:
"algebra, generally (inc. quadratic equations, etc.) is incredibly important." Build it
properly, no placeholders, each piece tested; ZK-private variants optional, layered on the
real `prove_matrix_multiply` circuit pattern already landed.

## Existing homes (survey 2026-06-21 — do NOT duplicate)
- `specialized_libs/linear_algebra.rs` — general N×N dynamic `Matrix` (multiply/add/
  transpose/inverse/solve/private_matrix_multiply). **This is the home for general numeric
  algebra.** No determinant/eigen/SVD/polynomial yet.
- `solvers/linear_algebra/mod.rs` — FIXED 4×4 `Matrix4x4`/`Vector4` + Lanczos eigensolver,
  for the geometric/quantum substrate. Keep separate; don't generalise it.
- `solvers/symbolic_logic/mod.rs` — SAT / defeasible LOGIC, **not** computer algebra. A
  symbolic-algebra engine is a NEW module, kept distinct from this.
- `compute_universe.rs` + `tensor/{volume_gpu,bake_pipeline,resident_substrate}.rs` — the
  10D manifold + wgpu search path. COPY_DST validation bug fixed (commit 6f4b79a11).

## Conventions
- Numeric: `f64`, dynamic dimensions, return `Result<_, LinearAlgebraError>`.
- Every op: (a) numeric backend, (b) unit tests incl. an analytic check + an ill-cases
  test, (c) MCP tool where user-facing, (d) optional ZK variant once numeric is solid.
- Symbolic: its own module; exact rational/expression trees, no float drift.
- Honesty guard: no `privacy_preserved`/"proven" claims beyond what a test demonstrates.

---

## Phase 1 — Polynomial roots (quadratics first)  [DONE for numeric roots]
Home: `specialized_libs/linear_algebra.rs` (numeric) → later mirrored symbolically.
- [x] 1.1 `solve_quadratic(a,b,c) -> QuadraticRoots` — stable `q = -(b + sign(b)·√Δ)/2`;
      handles a≈0 (linear), Δ≈0 (double), Δ<0 (complex pair). `Complex` type added.
- [ ] 1.2 `solve_cubic` / `solve_quartic` closed form — OPTIONAL; covered numerically by 1.3.
- [x] 1.3 `polynomial_roots(coeffs)` — general degree via dependency-free Durand–Kerner
      (all complex roots simultaneously). Companion-matrix route deferred (1.3 suffices).
- [x] 1.4 Tests: x²−5x+6→{2,3}, x²+1→±i, x²−2x+1→double, linear fallback, cubic
      {1,2,3}, quartic x⁴−1→{1,−1,i,−i}.
- [x] 1.5 MCP tool `algebra_solve_polynomial` (coeffs → roots). Tested.

## Phase 2 — Eigenvalues / SVD / determinant (general N×N)  [core done]
Home: `specialized_libs/linear_algebra.rs`.
- [x] 2.1 `determinant(n, data)` via LU (partial pivoting); 0.0 for singular. Tested (2×2,
      3×3 = −306, singular).
- [ ] 2.2 `lu_decompose` (P,L,U) as a reusable primitive — det currently inlines LU.
- [x] 2.3 `eigen_symmetric(n, data)` — cyclic Jacobi; returns eigenvalues + eigenvector
      columns; rejects asymmetric input. Tested (A·v = λ·v, unit vectors).
- [ ] 2.4 `eigen_general` — QR + Hessenberg (real + complex spectra).
- [x] 2.5 `svd(m,n,data) -> Svd{singular_values,u,v}` — via eigendecomposition of AᵀA,
      σ descending, U = A·V·Σ⁻¹. Tested by reconstruction ‖A − UΣVᵀ‖ < 1e-9 (3×2 case).
- [ ] 2.6 More tests: general (non-symmetric) spectra once 2.4 lands.
- [x] 2.7 MCP tool `algebra_matrix_analyze` (op ∈ {determinant, eigenvalues, eigen_symmetric,
      svd}). Tested.

## Phase 3 — Symbolic algebra (CAS)  [new module]
Home: NEW `specialized_libs/symbolic_algebra.rs` (distinct from `solvers/symbolic_logic`).
- [ ] 3.1 `Expr` tree: Const(rational), Var(sym), Add/Mul/Pow/Neg/Div. Zero-heap-friendly
      where it touches hot paths; CAS itself may allocate (it is a tooling/authoring path,
      not an NQuin hot path — keep it off the SlgArena).
- [ ] 3.2 `simplify` — constant folding, like-term collection, canonical ordering.
- [ ] 3.3 `differentiate(expr, var)` — symbolic derivative.
- [ ] 3.4 `solve_quadratic_symbolic` — exact `(-b ± √(b²−4ac))/2a`, returns Expr roots.
- [x] 3.4b `parse(&str) -> Expr` — recursive-descent parser (`+ - * / ^`, parens, sqrt,
      vars/numbers) so expressions can come from text. Tested.
- [ ] 3.5 `expand` / `factor` (at least quadratics → binomial factors over rationals) — TODO.
- [x] 3.6 Tests: symbolic derivative vs finite difference; simplify identities incl.
      x+x→2x; symbolic quadratic vs numeric; parse+differentiate roundtrip.
- [x] 3.7 MCP tool `cas` (op ∈ {differentiate, simplify, evaluate, solve_quadratic}). Tested.
- [~] 3.8 Bridge: `expr_citation_hash` gives a stable provenance hash so symbolic results can
      be cited into the graph. Full `Expr` ↔ NQuin-tree encoding still TODO.

## Phase 4 — wgpu 10D manifold
Home: `compute_universe.rs` + `tensor/*`.
- [x] 4.1 Audit. FIXED earlier: `count_buf` COPY_DST validation panic (commit 6f4b79a11).
      FOUND + documented: the GPU search uses the EUCLIDEAN metric (tensor_volume.wgsl,
      7 dims x,y,z,t,α,μ,σ) but the CPU fallback (`Q42TensorView::tensor_search_into`)
      uses `full_distance`, which switches on the node `v` topology class. They AGREE for
      `v == 0` (the common case / producer fixtures) but DIVERGE for `v != 0` — results
      then depend on GPU availability. Documented at the fallback site in
      `compute_universe.rs::run_tensor_search_producer_cycle`. **Open follow-up:** unify the
      metric (port `full_distance` to WGSL, or make the volume search euclidean-only).
- [x] 4.2 CPU reference: `volume_gpu::cpu_tensor_search_into` — GPU-independent linear scan
      mirroring the shader's exact 7-dim euclidean metric; the manifold search is now
      testable without a GPU. Tested analytically (radius sweeps; q/v/w confirmed
      metric-irrelevant).
- [x] 4.3 FrameLayout ABI respected — no manifold metadata layout change made; bake `t`
      clock stays at NQuin metadata [32:60] (see project-frame-layout-abi).
- [x] 4.4 `producer_cycle_with_global_substrate` passes (post COPY_DST fix); CPU-reference
      analytic test added. (Exact GPU==CPU-fallback equality cross-check is gated on the
      4.1 metric unification for v != 0.)

## Phase 5 — Optional: ZK-private variants
- [ ] Layer ZK on the new ops using the `prove_matrix_multiply` pattern (private witnesses,
      public result, real R1CS constraints). Only where there is a user-facing privacy need;
      integer/fixed-point encoding limitation documented.

## Verification gate (every phase)
`cargo test -p qualia-core-db --lib <area>` green; full `--lib` stays 990+/0; no new
`privacy_preserved`/"proven" claim without a test that exercises it.
