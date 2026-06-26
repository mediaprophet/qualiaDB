# Unified Math Substrate — working plan

**Status:** working document · **Started:** 2026-06-26 · **Directed by:** Timothy
**Companion to:** [`MODALITY_FIRST_CONSOLIDATION.md`](MODALITY_FIRST_CONSOLIDATION.md) (this is its
LLM-lane chapter) · the `.q42` v2 / STELLAR master plan (⚑ path to confirm — see §8).

---

## 1. The problem (stated plainly)

The LLM/`gguf` stack is a **second, parallel math pipeline**. It carries its own GPU GEMM
(`coop_gemv`), its own dequant, attention, FFN, and norms — implemented *LLM-private*, separate from
the modality math in `solvers/`. That is the same fragmentation the modality-first work is removing,
but at its deepest and most performance-critical point: there are now (at least) **three GEMMs** in
the tree — `solvers/linear_algebra` (fixed-size CPU), `specialized_libs/linear_algebra` (dynamic heap
CPU), and `gguf_bridge/gemm.rs` (GPU). Three implementations of one operation cannot be optimised,
scheduled, or proven once.

**Objective:** the math lives **once, organised by modality**, and is then **employed cleanly,
multi-threaded, by every consumer — including the LLM runtime**. `gguf`/`safetensors` become
**ingest-only** formats: the system ingests them and turns them into native, optimised **`.q42`**,
which is what the runtime actually executes.

## 2. Scope & sensitivity (read before touching anything)

This plan's target is the **LLM/GPU lane — the most actively-worked, most perf-tuned area of the
repo.** The measured wins there are **assets to promote into the substrate, never duplicates to
delete or rewrite**:
- resident weights: 3B decode 0.67 → 2.31 tok/s (3.4×), token-identical.
- `coop_gemv` cooperative kernel: +35% over naive, parity-proven vs CPU.
- FFN fusion, attention residency, Phase-2/3 — all measured, all committed.

**This document is a direction, not a go-ahead to cut into `gguf_bridge`.** Execution must be
Timothy-allocated and coordinated with whoever is live in that lane. Every routing step must be
**parity-gated and behaviour-identical**, using the lane's own proven method (a `QUALIA_LLM_*`
toggle, A/B against the existing kernel, byte-identical output before default-on). No barging, no
"reconcile", no competing.

## 3. Target architecture

```
            ┌─────────────────────────────────────────────────────────────┐
  consumers │  classical solvers   specialized_libs   LLM runtime          │
            │  (algebra/stats/…)    (domain compose)   (llm_agent, Phase-8) │
            └───────────────┬───────────────┬──────────────────┬───────────┘
                            │  one API, by modality            │ governance preserved
            ┌───────────────▼──────────────────────────────────▼───────────┐
  substrate │  solvers/linear_algebra (GEMM/GEMV + decompositions)          │
   (single  │  solvers/tensor        (contraction, quantized tensor ops)    │
   source   │  solvers/statistics · calculus · geometric_algebra · …        │
   of truth)│  ── hetero dispatch / execution layer ──                      │
            │     backend select (CPU / SIMD / GPU) + multi-threaded graph  │
            └───────────────┬───────────────────────────────────┬──────────┘
                     CPU/SIMD kernels                    GPU backend (vendor-neutral wgpu)
                                                         = promoted coop_gemv / dequant / attention
            ┌──────────────────────────────────────────────────────────────┐
   format   │  ingest: gguf / safetensors  ──►  native optimised .q42       │
            │  runtime executes .q42 ONLY                                    │
            └──────────────────────────────────────────────────────────────┘
```

Key points:
- **One GEMM, many backends.** `coop_gemv` (GPU) and the dequant kernels become the substrate's GPU
  backend, exposed through the linear-algebra/tensor API. The LLM stops calling them privately.
- **The LLM runtime stays a runtime.** `llm_agent`, the Phase-8 bifurcated compute, and the Webizen
  VM gates (`validate_intent`/`validate_output`) are preserved — the LLM becomes a *consumer* of the
  substrate, not the owner of the math. Governance is untouched.
- **Execution/dispatch is the "employ it cleanly, multi-threaded" layer** — the vendor-neutral
  `hetero_dispatch`/wgpu path (NOT QPU, which is deprioritised), with thread placement via
  `platform_scheduler`. It builds the execution graph and batches to GPU/NPU.

## 4. Inventory — the duplicate math to unify

| LLM-private today | Operation | Canonical substrate home |
|---|---|---|
| `gguf_bridge/gemm.rs` + `shaders/*` (`coop_gemv`) | dense GEMM/GEMV (GPU) | `solvers/linear_algebra` (GPU backend) |
| `gguf_bridge/cpu_ops.rs` (`stack_gemm_quant`, norms) | CPU GEMM, RMSNorm | `solvers/linear_algebra` + `solvers/tensor` |
| `gguf_bridge/attention.rs` | attention (QKᵀ, softmax, ·V) | `solvers/tensor` (+ `statistics::softmax`) |
| `gguf_bridge/ffn.rs` | FFN (gate/up/SiLU/down) | composition over `linear_algebra` GEMM |
| `inference/ggml_quants.rs` | Q4_K/Q8/F16 dequant | `solvers/tensor` (quantized tensor codec) |
| `inference/ternary.rs` | ternary quant/matmul | `solvers/tensor` (quantized tensor codec) |
| `gguf_bridge/embedding.rs`, `output.rs` | embed lookup, lm_head + top-k | `solvers/tensor` + `statistics` (top-k) |
| RoPE (in attention path) | rotary position embedding | `solvers/tensor` (or `geometric_algebra` — rotation) |

(Full per-symbol mapping is the P0 deliverable below.)

## 5. Phased path (honest, staged, parity-gated)

- **P0 — this plan + full inventory.** Enumerate every math symbol in the LLM stack that duplicates a
  modality, map each to its substrate home (the table above, completed to function granularity).
  *No code.* Output: the mapping, reviewed by Timothy + the LLM-lane instrument.
- **P1 — define the substrate boundary.** A tensor/GEMM API in `solvers/` with explicit CPU/SIMD/GPU
  backends. **Adopt `coop_gemv` + resident-weights as the canonical GPU backend in place** (promote,
  expose through the API; do not rewrite). Parity-prove substrate-GEMM == current LLM GEMM.
- **P2 — unify the classical side first (low risk).** Route `solvers/linear_algebra` dynamic GEMM
  (and `specialized_libs`) through the same GPU backend, so there is one GEMM, CPU+GPU. This validates
  the API without touching the hot LLM decode path.
- **P3 — route the LLM through the substrate, one kernel at a time.** GEMM → attention → FFN →
  dequant. Each behind a `QUALIA_LLM_*` toggle, A/B vs the existing kernel, **byte-identical output**
  required before default-on. The lane's existing perf is the floor, not to regress.
- **P4 — native `.q42` ingest.** `gguf`/`safetensors` → optimised native `.q42` weight container
  (extend `q42_weight.rs`): chosen quantization + GPU-resident layout + the substrate's tensor map.
  The **heavy optimise pass runs once on capable hardware**; the user pays only the cheap native load
  (affordability rule). Runtime loads `.q42`.
- **P5 — collapse the parallel pipeline.** Once `.q42` is the native runtime path and the LLM math
  flows through the substrate, the `gguf`-specific runtime code becomes **ingest-only**. The second
  pipeline is gone.

## 6. Invariants & constraints (non-negotiable)

- **Zero-heap** stays the rule; LLM weight loading remains the documented, cfg-gated heap exception.
  Substrate tensor residency respects the cell (512 MB) / `SlgArena` (42 MB) budgets.
- **Affordability:** heavy ingest/optimise once on capable hardware; cheap native `.q42` fold on the
  user's device. We are not replacing datacenter compute — value is sovereignty/provenance/locality.
- **Native wgpu only** — no external LLM runtimes wired in (ollama/llama.cpp stay independent dev
  tools). GPU dispatch is vendor-neutral (`hetero_dispatch`/wgpu), **not QPU** (deprioritised).
- **Governance preserved** — Webizen VM gates and the Phase-8 Sentinel bifurcation are untouched; the
  LLM remains gated, the math just moves house.
- **Parity before perf** — every LLM routing step must be byte-identical and must not regress the
  measured tok/s. The lane's numbers are the floor.

## 7. Multi-threading model ("employ it cleanly")

The execution layer is the shared, thread-safe compute service: backend selection + an execution
graph that batches independent ops and schedules them across threads (`platform_scheduler`
`bind_inference_thread`/`bind_background_thread`). The Phase-8 LLM-engine + Sentinel two-thread
bifurcation is preserved on top — the substrate gives the engine thread a clean, batched math service
instead of a private kernel zoo.

## 8. Relationship to existing docs

- Extends [`MODALITY_FIRST_CONSOLIDATION.md`](MODALITY_FIRST_CONSOLIDATION.md) — this is its LLM-lane
  chapter (the math the consolidation hadn't yet reached).
- Must align with the `.q42` v2 multimodal-manifold / STELLAR master plan. **⚑ I could not find it at
  `.dev-docs/QUALIA_MASTER_PLAN.md` on this branch's base — Timothy: confirm the path so this plan
  cross-links instead of duplicating.**

## 9. ⚑ Where I need Timothy

1. **Sequencing vs the live LLM perf lane.** This *cannot* proceed by me reaching into `gguf_bridge`.
   It needs your allocation and coordination with whoever is live in that lane — or an explicit
   hand-off. How do you want P1–P3 sequenced against the in-flight perf work?
2. **Promote-not-rewrite, confirmed?** The plan treats `coop_gemv` / resident-weights / FFN-fusion as
   the canonical GPU backend to *promote in place*. Confirm that's the intent (vs any reimplementation).
3. **`.q42` weight authority.** Is `q42_weight.rs` the target native container to extend for P4, or is
   there a newer `.q42` v2 spec I should build P4 against?
4. **Master-plan path** (§8) so I anchor to it.

## 10. Open questions / risks

- The substrate GEMM API must be expressive enough for quantized + fused ops without forcing the LLM
  back into private kernels (the failure mode: an API so generic the hot path routes around it).
- Quantized tensor codecs (Q4_K/Q8/ternary) are math *and* format — they straddle `solvers/tensor`
  and the `.q42` ingest layer; the boundary needs care so dequant isn't re-duplicated.
- P4 (`.q42` ingest) is where "optimised" must be defined concretely (layout, quant choice, residency)
  — that's a design sub-plan of its own.
```
