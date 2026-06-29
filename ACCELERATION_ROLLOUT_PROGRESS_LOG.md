# Acceleration rollout — progress log

Honest per-step record (per project rule §9) of wiring QualiaDB's compute consumers to the
capability-aware WGSL-Forge dispatcher (`crate::wgsl_forge::dispatch`), which picks the best
compute path actually present on the machine (CUDA-f64 / WGSL-f32 / df64 / CPU floor) per call.
Scope this session: **everything except the LLM path** (deferred to its own refactor, per Timothy).
Companion inventory: [`docs/plans/acceleration-integration-map.md`](docs/plans/acceleration-integration-map.md).

---

## 2026-06-29 · A0 — dispatcher generalization + new pairwise op · DONE

**What was built**
- `solvers/linear_algebra/gemm.rs`: generalized the keystone `gemm()` offload from *plain-only*
  (No/No, α=1, β=0) to **all transpose combos and arbitrary α/β** — materialize the transposed
  operand into row-major scratch on the offload path (O(mk)/O(kn), dominated by the O(mnk) GEMM),
  then apply α/β in the O(mn) CPU combine. Unlocks the hot covariance `XᵀX` and attention `Q·Kᵀ`.
- `wgsl_forge/dispatch.rs`: new `pairwise_sq_dist_f64` (all-pairs ‖aᵢ−bⱼ‖² via the `gemm_f64`
  cross-term + row norms, negatives clamped) + exact direct reference `pairwise_sq_dist_cpu_f64`.

**Measured** — `wgsl_forge::dispatch` 15 passed/0 failed (+5 new incl. pairwise identity-vs-direct);
`solvers::linear_algebra::gemm` 14 passed/0 failed (+2: transposed covariance 4096×8, scaled+β
accumulate, both above threshold vs inline CPU reference). Off-accelerator the CPU floor is
byte-identical to before.

**⚑ Where I need the human** — none this step.
**Next** — wire the real consumers (A1–A4).

---

## 2026-06-29 · A1–A4 — wire ML / solver consumers · DONE

**What was built** (each best-path with a byte-identical CPU floor; gated on `caps()` + `GEMM_GPU_THRESHOLD`)
- `linear_algebra/svd.rs`: `AᵀA` gram + `U = A·V` routed through the forge GEMM.
- `linear_algebra/spectral.rs`: Faddeev–LeVerrier `A·M` (the O(n⁴) characteristic-polynomial loop)
  routed through `matmul`.
- `learning/clustering/kmeans.rs`: assignment `AllPairs` via `pairwise_sq_dist_f64` (exact per-point
  `nearest` CPU floor below threshold; lowest-index tie-break preserved).
- `learning/classification/svm.rs`: kernel matrix — linear `X·Xᵀ` GEMM, RBF `exp(−γ·pairwise)`; the
  SMO working-set loop stays CPU.
- Transitively accelerated (already call the keystone `gemm()`/`matvec()`, which now offloads the
  transposed/scaled cases too): `dimensionality/pca.rs` covariance, `regression/{ridge,linear,bayesian}.rs`,
  `attention.rs` (`Q·Kᵀ`, scores·V), `sequential/kalman.rs`, `specialized_libs/linear_algebra`.

**Honest skips (NOT faked)** — `eigen.rs` cyclic-Jacobi (fine-grained data-dependent rotations, not a
GEMM); `qr.rs` Householder panels; `clustering/gmm.rs` E-step (per-component diagonal-covariance
reduction — each component rescales features, not a uniform GEMM/pairwise); `cholesky.rs` unblocked
(no single dense GEMM — a *blocked* SYRK/GEMM rewrite is a mid-feature algorithm change, **deferred**
to a dedicated pass per §11/§12). `specialized_libs/{machine_learning,statistical_computing}` are
stubs (real LLM GEMM lives in the excluded `gguf_bridge` substrate lane).

**Measured** — solvers + specialized_libs + wgsl_forge + audio = **910 passed / 0 failed / 26 ignored**;
`cargo build --lib --features cuda` + `cargo build -p qualia-cli` both **exit 0**. Purely additive,
behaviour-preserving. Committed `73aae685`.

**⚑ Where I need the human** — none this step (cholesky-blocked deferral is an engineering call, flagged
above; raise it if you want it prioritized rather than batched into the library-ization pass).
**Next** — formalize the orphan physics shaders (B1) + surface the fluid/quantum_bio curation ask (B2).

---

## 2026-06-29 · B1 — formalize MD + kinematics as certified forge kernels · DONE

**What was built** — new `wgsl_forge/physics/` module (§11 sub-directory): the two genuinely-real
orphan shaders, corrected and certified.
- `shaders/molecular_dynamics.wgsl` + `physics/molecular_dynamics.rs`: velocity-Verlet drift/kick under
  a constant force + PBC. **Fixed the bug** that the orphan updated position but *left velocity
  unchanged* (half an integrator). Flat `f32` layout (no `vec3` std430 padding). Exact CPU oracle,
  naga-validated, GPU-certified.
- `shaders/kinematics.wgsl` + `physics/kinematics.rs`: softened (Plummer) inverse-square N-body +
  symplectic-Euler. **Fixed the data race** — the orphan read and wrote the same buffer (non-
  deterministic); now double-buffered (forces from input only). Exact CPU oracle, naga-validated,
  GPU-certified.
- Both embed their `.wgsl` via `include_str!` (single source of truth).

**Measured** — `wgsl_forge::physics` 6 oracle/naga tests passed/0 failed; **both GPU-certify tests
PASS on the RTX A2000** (`md_gpu_matches_oracle`, `nbody_gpu_matches_oracle` — real WGSL dispatch vs
CPU oracle within f32 tol). `cargo build --lib --features cuda` exit 0. Caveat: these are correct,
certified, *available* kernels; no production consumer calls them yet (they were orphans — formalizing
makes them real and trustworthy; wiring a physics consumer is a separate future step).

**⚑ Where I need the human** — see B2 below.
**Next** — B2 curation ask; then final commit/push.

---

## 2026-06-29 · B2 — fluid_dynamics + quantum_bio · BLOCKED on curation (⚑ Timothy)

These two orphan shaders **cannot be formalized honestly without a physical-model decision**, which is
yours to make — inventing the model would violate the no-faking / give-evidence-real-weight norms.

- **`fluid_dynamics.wgsl`** is a placeholder: the whole kernel is `velocity *= 0.99` (labelled "mock
  Navier–Stokes"). A real kernel needs a **chosen scheme** — e.g. explicit diffusion/advection stencil,
  Jacobi pressure-projection, or SPH — each with different state and boundary handling.
  **⚑ Ask:** which fluid model do you want (and at what fidelity)? Given one, I'll formalize + certify it
  exactly like MD/kinematics.

- **`quantum_bio.wgsl`** is **non-compiling** (invalid inline-struct syntax; it shadows the WGSL builtins
  `exp`/`sqrt`/`sin`/`cos` with crude Taylor approximations) **and partly demo-grade** (one uniform reused
  as "singlet rate", "drug concentration", etc.). The WKB tunneling and Arrhenius parts are real physics;
  the radical-pair / drug-binding parts are illustrative.
  **⚑ Ask:** which observables are real targets, and what are their correct parameterizations? This is
  curation-grade (and brushes medical/chemistry claims you reserve) — I won't invent it.

**Next** — none until direction; the rest of the rollout is complete.
