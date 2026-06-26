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
