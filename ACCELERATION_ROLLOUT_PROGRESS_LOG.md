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

---

## 2026-06-29 · DAG-IR Phase 3a — native Elementwise LLM kit · DONE (GPU-certified)

The activation/arithmetic op-class that, with P2's Reduce + Broadcast, completes the **single-node
vocabulary** for RMSNorm / softmax / SwiGLU.

**What was built** — [`graph_ops/elementwise.rs`](crates/qualia-core-db/src/wgsl_forge/graph_ops/elementwise.rs):
three arities under one op-class — unary `Silu/Gelu/Exp/RecipSqrt/Relu/Recip` (`out=f(in)`), binary
`Add/Mul` (`out=a⊙b`), ternary `Fma` (`out=a·b+c`) — emitted from `OpNode::Elementwise{f}` by the
`WgslGraphLowerer`, each with an exact CPU oracle matching the WGSL math. `Scale`/`Bias` deliberately
defer to the certified `affine-f32` kernel (explicit `Err`, not a silent stub).

**Honest scope split** — the design's P3 also wanted **Softmax legalization** and the
**`MatMul→Elementwise` fusion** (fused-ffn emergent). Those are **multi-node** graphs; their GPU
certification needs the topo-order executor → they move to **P3b/P4** (the CPU-composed oracle is
buildable independently, but I won't claim a GPU-certified multi-node result before the executor
exists). Tracked, not faked.

**Measured** — `graph_ops::elementwise` 3 non-GPU pass/0 fail; **GPU-certify on A2000**:
`elementwise_gpu_matches_oracle` passes (Silu/Gelu/Exp/Relu/RecipSqrt/Recip + Add/Mul vs oracle,
f32 tol); full `wgsl_forge` non-GPU green; `--features cuda` + cli clean.

**⚑ Where I need the human** — none. (The fluid-model curation ask still stands for P7.)
**Next** — **P4: the topological multi-node executor** (Option A: per-node dispatch with intermediates
kept GPU-side in `QualiaSlabAllocator`). It unblocks everything multi-node at once: softmax, the
fused-ffn fusion, the ternary `{GatherDequant→MatMul}` split, and the full LLM decode-block DAG graded
against a graph-composed CPU oracle.

---

## 2026-06-29 · DAG-IR Phase 4 — topological multi-node executor · DONE (GPU-certified, the keystone)

The phase the whole LLM path was waiting on: a **whole `ComputeGraph` executed on the GPU**, intermediates
kept device-side, graded against a topologically-composed CPU oracle.

**What was built** — [`graph_ops/executor.rs`](crates/qualia-core-db/src/wgsl_forge/graph_ops/executor.rs):
- `execute_graph(graph, externals)` — runs nodes in topo order; per node it binds inputs, allocates an
  output, dispatches the op's kernel (Reduce/Broadcast/Elementwise native + **MatMul** via the gemm
  module), then hands the output to the next node **on the GPU** (no host readback).
- `execute_graph_cpu` — the composed differential oracle (threads node CPU floors in topo order).
- Graph builders: `softmax_graph` (7 nodes), `rmsnorm_graph` (5), `swiglu_ffn_graph` (5 — the LLM FFN).
- Added `EwKind::Sub`/`Div` (softmax needs them) + `WgpuComputeContext::copy_view` (GPU→GPU hand-off).

**The real bug, found & fixed on hardware** — wgpu forbids the **same buffer** being bound read-write
*and* read-only within one dispatch (read_write is exclusive). The first design put every tensor in one
slab → validation error on the A2000. Fix: inputs/params live in the **read slab**, each output is written
to the **read_write slab** then `copy_view`'d back into the read slab for the next node. Honest two-slab
device-side hand-off; the per-node submit + copy latency is the accepted Option-A cost (single-encoder
fusion is a later perf pass).

**Measured** — `execute_graph_*` CPU-oracle tests 3 pass/0 fail; **GPU-certify on the A2000**:
`execute_graph_gpu_matches_cpu_oracle` **passes** — softmax (1024-wide), RMSNorm (768), and the
**SwiGLU-FFN block** (MatMul·MatMul·Silu·Mul·MatMul, seq8×dim64×ffn128) all match the composed CPU oracle.
Full `wgsl_forge` non-GPU green; `--features cuda` + cli clean.

**⚑ Where I need the human** — none. (Fluid model still stands for P7.)
**Next** — P4b (assemble the full attention block + ternary GatherDequant split + an honest kernel-level
uplift benchmark), then P5 (`CudaCLowerer` — the same graph → CUDA-C in one pass), P6 (q42 persistence),
P7 (RT `Neighbor` + fluids).

---

## 2026-06-29 · DAG-IR P4c (part 1) — tiled tensor-core WMMA GEMM + `gemm_f32_tc` · DONE (GPU-certified)

Triggered by Timothy's "have we *ensured a wgpu pathway*, or just pushed to alternatives?" The honest
audit found the tensor-core kernels were proven single-*tile* primitives (8×8×8 coopmat / 16×16×16 WMMA),
**not** GEMM backends, and the `MatMul.tc` flag was a no-op. This step builds the real tiled GEMM.

**What was built**
- `emit/cuda_c.rs::WMMA_GEMM_TILED_SRC` — a **tiled** WMMA GEMM: one warp per 16×16 output tile, looping
  the proven single-tile primitive across `K/16` inner tiles (registers accumulate). `C[M,N] f32 =
  A[M,K] f16 · B[K,N] f16`, `M/N/K` multiples of 16. A real tensor-core GEMM, not one tile.
- `dispatch::gemm_tc_cuda` — f32→f16 host pack, `dims=[m,n,k]` storage buffer, NVRTC compile, grid of
  `(M/16)·(N/16)` warps.
- `dispatch::gemm_f32_tc` — the **opt-in capability-selected** entry point that makes `MatMul.tc` real:
  WGSL coopmat (portable, *dormant until #9741*) → CUDA WMMA (NVIDIA tensor cores, **now**) → plain f32
  floor. Reduced precision (f16 inputs) is opt-in; plain `gemm_f32` stays full-precision.

**Honest scope** — this is the **CUDA tensor-core** half of P4c, certified now. The **WGSL coopmat** half
(the actual *wgpu* tensor-core pathway) is a tiled coopmat WGSL kernel + `coopmat_usable()` probe; it is
**dormant on wgpu 29.0.3** (coopmat multiply returns zeros), so it is built+verified **together with the
#9741 soft-fork experiment** (task #56) — there's no way to certify its tiling before the multiply works.
So today the *active* tensor-core path is CUDA WMMA; the wgpu coopmat slot is wired-in-spirit (the
selection point + probe gate) and lights up when the fix lands. Recorded in `docs/WGPU_UPSTREAM_TRACKING.md`.

**Measured** — `gemm_tc_cuda_tiled_matches_f16_reference` **passes on the A2000** (64×64×64 = 4×4 tiles ×
4 K-tiles, vs the f16-rounded f32 reference, with a non-zero sanity check that rules out the
broken-multiply symptom); `gemm_f32_tc_falls_to_plain_floor` (non-GPU) green; lib `--features cuda` + cli
build clean.

**⚑ Where I need the human** — none to build; the coopmat-half + soft-fork (#56) is the experiment that
perturbs the dep pin (your earlier "experimental OK for v1" covers it).
**Next** — P4c part 2 (tiled coopmat WGSL + probe, alongside the #9741 soft-fork), then wire `gemm_f32_tc`
into the DAG-IR `MatMul.tc` (cleanest via the P5 CudaCLowerer, where tensor-core matmul composes without a
host round-trip). Then P4b / P5 / P6 / P7.

---

## 2026-06-29 · DAG-IR P4c **part 2** — tiled coopmat WGSL GEMM + `coopmat_usable()` probe + `MatMul.tc` real in the executor — DONE (wgpu pathway built; multiply dormant until #9741)

**Step / phase** — DAG-IR forge P4c part 2 (the *wgpu* tensor-core half). Status: **done** (built, naga-validated, executor-wired, A2000-checked); coopmat **execution** remains dormant on wgpu 29.0.3 (an upstream bug, not our code).

**What was built**
- `emit/coopmat.rs::matmul_tc_wgsl_tiled` (+ `MATMUL_TC_TILED_ENTRY`) — a **tiled** cooperative-matrix
  GEMM that loops the proven single-8×8×8-tile primitive over arbitrary `m/n/k` (multiples of 8): one
  workgroup (== one subgroup, `@workgroup_size(32)`) per 8×8 output tile, accumulating across K in a
  coopmat register fragment. Bindings `a`(0)/`b`(1)/`c`(2,rw)/`dims`(3)=[m,n,k]. **Mirrors**
  `WMMA_GEMM_TILED_SRC` structurally, so both backends tile the same DAG node identically.
- `dispatch::gemm_f32_tc_coopmat` — host-side wgpu dispatch of that kernel (transient ctx, two-slab
  bindings, `num_tiles=(m/8)·(n/8)` workgroups), mirroring `gemm_f64_df64`. Pure wgpu (no CUDA).
- `dispatch::coopmat_usable()` — runtime probe (the f32 mirror of `df64_usable`): runs a tiny 8×8×8
  all-ones coopmat GEMM (exact result 8.0) and accepts coopmat only if the result matches. On 29.0.3 the
  multiply returns zeros → probe is **false**; it flips **true automatically** once a wgpu release / the
  soft-fork carries #9741. Gated first on `caps().coopmat`; memoised.
- `dispatch::gemm_f32_tc` — tier-1 coopmat slot now **filled** (coopmat → CUDA WMMA → plain f32 floor),
  selected via `coopmat_usable()`.
- `graph_ops/executor.rs` — `OpNode::MatMul.tc` is now **real**: `tc=true` routes to the coopmat tiled
  kernel (device-side, in the slab model) when `coopmat_usable()` & 8-multiple dims, else the certified
  plain GEMM floor. Extracted `dispatch_matmul_plain` / added `dispatch_matmul_coopmat`. (The CUDA WMMA
  tensor-core path is reached host-side via `gemm_f32_tc` and graph-side by the P5 CudaCLowerer — not from
  the wgpu executor, which would otherwise break the GPU-side intermediate model.)

**Measured results (A2000, real)**
- `cooperative_matrix_tiled_gemm_validates` — naga-validates the tiled kernel (4 bindings, entry
  `matmul_tc_tiled`). **green.**
- `coopmat_usable_respects_caps_and_is_cached` — the probe **runs on the A2000**, returns **false**
  (29.0.3 coopmat multiply = zeros, #9741), is consistent with `caps()`, and is memoised. **green.**
- `gemm_f32_tc_coopmat_rejects_non_8_multiples` + `gemm_f32_tc_falls_to_plain_floor` — **green.**
- `execute_graph_gpu_matches_cpu_oracle` (P4 multi-node: softmax / RMSNorm / SwiGLU-FFN) — **still green**
  after the MatMul-arm refactor.
- `gemm_f32_tc_coopmat_matches_cpu_reference` — `#[ignore]` GPU cert (16×16×16 = 2×2 tiles × 2 K-tiles);
  asserts the real tensor-core result + a non-zero sanity check. **Dormant until #9741** — it is the test
  that lights up the moment the soft-fork (#56) / a wgpu release makes the multiply compute.
- Full `wgsl_forge::` non-ignored sweep: **106 passed, 0 failed**; lib + `--features cuda` build clean.

**Honest state of the "is there a wgpu tensor-core pathway?" question** — **yes, and it is now built, not
just slotted.** The tiled coopmat kernel exists and is naga-valid; the probe + selection + executor wiring
are in place and exercised on hardware. The *only* thing that cannot be certified today is the coopmat
**multiply itself**, which is a wgpu 29.0.3 upstream defect (#9741, merged upstream, unreleased) — so the
GPU cert is `#[ignore]` and `coopmat_usable()` correctly keeps the path dormant until the fix ships. Active
tensor-core GEMM today = CUDA WMMA; portable wgpu coopmat = built + self-activating.

**⚑ Where I need the human** — none this step. The remaining coopmat *verification* rides the #56 soft-fork
(perturbs the dep pin; covered by your "experimental OK for v1").
**Next** — P5 CudaCLowerer (same graph → CUDA-C in one pass; the clean home for `MatMul.tc → WMMA`
graph-side), then P4b attention + GatherDequant + uplift bench, P6 q42 binding, P7 RT/Stencil/MSL/HLSL.
