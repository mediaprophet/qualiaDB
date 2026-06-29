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

---

## 2026-06-29 · DAG-IR forge — design pass + Phase 1 spine · DONE (green slice)

Reframed after Timothy's point that the forge had been **accreting per-`kernel.id` string
emitters** (gemm/gemv/fft are hand-written WGSL; MSL/HLSL/PTX emit *zero* of them) instead of
*generating* kernels — and that the **q42 format already represents DAGs**. The fix: a typed
**compute-graph IR** the forge lowers to every backend in one pass, riding the q42 substrate.

**What was built**
- **Design pass** (ultracode workflow, 10 agents): parallel reads of the q42 substrate, the
  IR/emit pipeline, schedule/certify, and the ray-query path; three judged architecture
  proposals; adversarial verification against the real code. Output → [`docs/plans/dag-ir-forge.md`](docs/plans/dag-ir-forge.md)
  (full scope: 10-node op vocabulary, IR types, one-pass Lowerer architecture, honest q42
  encoding, RT-core `Neighbor` boundary, certify plan, P1–P7).
- **Phase 1 (spine):** new [`wgsl_forge/ir/graph.rs`](crates/qualia-core-db/src/wgsl_forge/ir/graph.rs) —
  `ComputeGraph`/`OpNode`(12-arm vocabulary)/`GraphNode`/`TensorRef`/`Shape`/`DType`, arena-backed,
  **acyclic by construction** (insertion == topo order), with shape/dtype + cycle asserts. A
  `Lowerer` visitor trait + `lower_graph` topological driver. `KernelSpec::to_graph()` bridges
  gemm/gemv/fft to one-node graphs. `emit_kernel_body` now **routes gemm/gemv/fft through the
  graph** (`to_graph → lower_graph → WgslDelegateLowerer`); the lowerer **delegates** to the proven
  `emit_*_wgsl` functions, so the WGSL is **byte-identical by construction** (the adversarial
  verifier's #1 risk — `source_hash` cache invalidation — neutralized).

**Honest scope** — Phase 1 establishes the *seam* (IR + topo-walk + trait dispatch + routing),
proven end-to-end; the leaf WGSL still delegates to the legacy emitters. Native graph-template
emission, the other op-classes, the LLM decode DAG, the CUDA/MSL/HLSL lowerers, q42 persistence,
and the RT `Neighbor` node are **scoped follow-on phases P2–P7** (tracked tasks #49–#54) — not
stubs hidden in P1.

**Measured** — `wgsl_forge::ir::graph` + emit byte-equal tests **11 passed/0 failed**; full
`wgsl_forge` non-GPU **91 passed/0 failed/28 ignored**; **GPU-certify on the A2000**: the routed
`generated_{gemm,gemv,fft}_matches_oracle_on_real_gpu` all **pass** (the kernels now flow through
the DAG-IR and still certify byte-equal on hardware). `cargo build --lib --features cuda` exit 0.

**⚑ Where I need the human** — none to *build* P2–P5 (LLM-pipeline path, all mechanical +
certifiable). The fluid/SPH side (P7) still folds in the **fluid-model curation ask** (B2). And
the **honest RT-core boundary** is in the plan: RT cores back the `Neighbor`/spatial op-class
only (SPH/N-body/kNN/ANN) — they do **not** accelerate the LLM dense matmuls.
**Next** — P2 (GatherDequant + Reduce + Broadcast), then P3 (Softmax + LLM elementwise + fused-ffn
as an emergent fusion).

---

## 2026-06-29 · DAG-IR Phase 2 — native Reduce + Broadcast op-nodes · DONE (GPU-certified)

The first **non-delegating** lowerings: Phase 1's gemm/gemv/fft delegate to legacy emitters; these
two op-classes have no legacy standalone kernel, so the WGSL is emitted **directly from the graph
node** — proving the native-template path the rest of the IR will use.

**What was built** — new [`wgsl_forge/graph_ops/`](crates/qualia-core-db/src/wgsl_forge/graph_ops/):
- `reduce.rs`: single-workgroup tree reduction (grid-stride fold → shared-mem tree → `output[0]`) for
  `Sum/Mean/Max/L2` — the RMSNorm-variance and softmax-max/denominator primitive. `reduce_wgsl(op,wg)`
  template + exact `reduce_cpu` (f64-accumulated floor) + `reduce_gpu` dispatch.
- `broadcast.rs`: index-remap tile (`out[i]=input[i%in_len]`) — RMSNorm/bias scale-fanout. Template +
  exact `broadcast_cpu` + `broadcast_gpu`.
- `WgslGraphLowerer` (no `KernelSpec`) + `emit_graph_wgsl` in `emit/wgsl.rs`: lowers `OpNode::Reduce`/
  `OpNode::Broadcast` from the node payload + per-node schedule. Phase 1's delegating lowerer is
  untouched (its byte-equal guarantee intact).

**Honest re-scope** — the design's P2 also bundled **GatherDequant + the ternary `{GatherDequant→MatMul}`
template**. That split is a *2-node* graph whose dequantized-weights intermediate must stay GPU-side,
which needs the **topological multi-dispatch executor P4 builds** (the adversarial verifier's §8.1
finding: `WgpuPipeline::dispatch` is one-encoder-per-call). Rather than half-build it, it's **moved to
P2b/P4-adjacency** and tracked — not faked into P2.

**Measured** — `wgsl_forge::graph_ops` + emit native-lowering test **10 passed/0 failed** (non-GPU);
**GPU-certify on A2000**: `reduce_gpu_matches_oracle` (all four kinds) + `broadcast_gpu_matches_oracle`
**pass**; full `wgsl_forge` non-GPU green; `--features cuda` + cli build clean.

**⚑ Where I need the human** — none this step.
**Next** — P3: complete the Elementwise LLM kit (Silu/Gelu/Exp/RecipSqrt/…) + Softmax sugar
(legalizes to Reduce→Broadcast→Elementwise — now that Reduce/Broadcast exist) + the MatMul→Elementwise
fusion that makes fused-ffn emergent.
