# P3 — routing the LLM/gguf math through the substrate: read-only analysis

**Status:** ANALYSIS ONLY (no edits to `gguf_bridge/`). The actual routing is **gated on Timothy's
explicit GO + a NOTICES check** that the gguf perf lane is clear. · **Branch:** `0.0.21-la` · 2026-06-26

> Companion to `UNIFIED_MATH_SUBSTRATE_PLAN.md` and `LINEAR_ALGEBRA_SUBSTRATE.md`. Written so the GO
> decision is made against the real shape of the code, not a slogan.

---

## 1. The gguf CPU math surface (what's actually there)

| Entry point | File | What it is | Relation to the substrate |
|---|---|---|---|
| `stack_gemm_quant` | `gguf_bridge/mod.rs:858` | **Quantized fused-dequant f32 GEMV** — `out[i] = Σ_j dequant(W)[i][j]·input[j]`, dequantizing each weight row on the fly from `raw` bytes | **The one substrate-analogous op** — but type/storage-divergent (see §2) |
| `coop_gemv` (GPU) | `shaders/fused_transformer.wgsl` + `gguf_bridge/gemm.rs` | The promoted GPU GEMV (1 workgroup/row, coalesced, in-shader dequant). +35% measured | The substrate's **GPU backend** (per the plan: *promoted in place, never rewritten*) |
| `cpu_attention_pass` | `gguf_bridge/attention.rs:350` | `softmax(QKᵀ/√d)·V` + RoPE | LLM-specific composition (small matmuls + softmax); **not** general modality math |
| `rope_inplace` | `gguf_bridge/mod.rs:825` | Rotary positional embedding | LLM-specific |
| `rms_norm_inplace` / `silu_inplace` / `relu_inplace` / `add_residual_inplace` | `gguf_bridge/cpu_ops.rs` | Element-wise / reduction vector kernels (f32) | Neural-net primitives, not dense-LA |
| `dequant_*` / `dequantize_row_into` | `inference/ggml_quants` | Quantization codecs (Q4_K/Q6_K/Q8/F16…) | **Ingest/runtime codec**, not math to consolidate |

**Honest finding:** the only genuinely substrate-analogous operation is the **GEMM/GEMV**. Everything
else (RoPE, attention softmax, RMSNorm, SiLU, residual, dequant) is either LLM-domain composition or a
quant codec — it does **not** belong in `solvers/linear_algebra`. So P3 is *not* "delete the loops and
call the engine." It is one operation (GEMM), unified at the right level.

## 2. Why naive routing would be wrong (the load-bearing constraint)

The substrate `solvers/linear_algebra/gemm` is **dense `f64`**. The LLM GEMM is **quantized,
fused-dequant, `f32`**. They share the abstract operation but differ in two ways that are the whole
point of the LLM path:

1. **Numeric type — `f32`, not `f64`.** Doubling to `f64` doubles weight memory and traffic.
2. **Operand storage — quantized bytes with on-the-fly dequant.** Materializing a dense `f64` (or even
   `f32`) weight matrix throws away the quant compression *and* the dequant-into-GEMV fusion that makes
   edge inference fit in budget.

Routing the LLM through the dense-`f64` substrate would **multiply weight memory by ~4–8×** (quant→f64)
and add a separate dequant pass — blowing the 128 MB backend / 512 MB cell budgets and **violating the
affordability invariant** (no build may force a user onto datacenter-class RAM). That is the opposite of
the goal. **The substrate must not impose `f64` dense on the LLM.**

## 3. The correct unification — at the *contract*, with promoted backends

Unify the **GEMM contract**, not the implementation. One abstraction, multiple backends selected by the
consumer's needs; parity-tested against a shared reference:

```
GEMM contract  (op = C := α·op(A)·op(B) + β·C, row-major, fail-closed)
├── dense-f64 backend     solvers/linear_algebra/gemm        (scientific modalities — precision)
├── quant-f32 CPU backend  gguf_bridge stack_gemm_quant       (LLM CPU/fallback — PROMOTED in place)
└── quant-f32 GPU backend  coop_gemv / resident-weights       (LLM GPU — PROMOTED in place)
```

- The measured wins (`coop_gemv` +35%, resident-weights 3.4×, FFN fusion) are **promoted as the LLM
  backend, never rewritten or routed through f64.**
- "Math lives once, employed multi-threaded by all consumers" is realized as a **single dispatch
  contract + parity oracle**, not a single numeric kernel. The dense and quant backends are checked to
  agree (the existing `gemm_parity_probe`: GPU quant GEMM == CPU reference to `max_abs_err ~1e-5`).
- What the substrate work *adds* here is the **shared trait + the parity test wiring**, so a consumer
  can't silently fork a fourth GEMM, and so the LLM path is a *registered backend* of the substrate
  rather than a parallel pipeline.

## 4. The other half — gguf/safetensors → native `.q42` (ingest-only)

Separate from the GEMM unification: the plan's end-state is that gguf/safetensors are **ingest-only**,
converted once into native optimised `.q42`, which is the only thing the runtime executes. The
`dequant_*` codecs become part of that ingest. This is orthogonal to §3 and can sequence after it.

## 5. Proposed P3 sequence (for the GO decision — nothing here is started)

1. **Define the GEMM trait/contract** in `solvers/linear_algebra` (numeric-type- and operand-source-
   generic). Dense-f64 implements it (already the reference).
2. **Register** the LLM quant CPU + GPU GEMV as named backends of the contract — *adapter shims around
   the existing `stack_gemm_quant`/`coop_gemv`, not rewrites.*
3. **Wire the parity oracle** as a substrate-level test (promote the existing `gemm_parity_probe`).
4. **Then** the `.q42` ingest-only path (§4).

**Risk controls (mandatory):** every step parity-gated, output **byte-identical**, **no tok/s
regression** (re-run the A2000 decode coherence + the coop_gemv parity test), default-off behind a
toggle until proven — exactly the discipline the LLM lane already used for `coop_gemv`.

## 5b. Step 0 LANDED — the equivalence is now proven in code (CPU, no GPU)

Before building any trait, the *thesis* is now executable and regression-guarded:
`gguf_bridge/gemm.rs::substrate_parity_tests::llm_quant_gemv_is_the_substrate_gemm`
- dequantizes Q8 weights to a dense matrix, runs the engine `solvers::linear_algebra::gemm::matvec`
  on them, and shows the **actual** LLM CPU kernel (`stack_gemm_quant`) agrees to `exact_err < 1e-4`
  (f32 rounding only) across several shapes/seeds;
- separately characterises the Q8 quantization cost (`quant_err`, bounded).

This calls the real kernel — not a copy — so it is a live contract: any future change to the LLM GEMV
that breaks GEMM semantics fails here. Combined with the existing GPU↔CPU probe (`gemm_parity_probe`,
`coop_gemv_parity`), the full chain is established and testable:

```
substrate dense GEMM  ≡  LLM CPU GEMV (stack_gemm_quant)  ≡  LLM GPU GEMV (coop_gemv)
      (this test, CPU)            (existing GPU↔CPU probe, A2000)
```

The "AI inference" weight×activation step **is** the science modalities' matrix multiply, quantized.
That is the de-mystification, made executable. Steps 1–4 (the contract type + `.q42` ingest) build on
this proven floor.

## 6. Lane status (read at analysis time)

No open `CLAIM` on `gguf_bridge` in `NOTICES.md` — the last entries are `RELEASE`s (coop_gemv
`23eae6695`, modularization, Codex fold). The lane *appears* clear. **Even so, no edit to `gguf_bridge/`
will happen without Timothy's explicit GO**, because it is a perf-critical shared lane and the
coordination rule is defer-don't-barge.

---

### What I need from Timothy to start P3 (one decision)
**GO / no-go on the §5 sequence**, and confirm the gguf lane is yours-to-allocate-to-me right now. If
GO: I start with step 1 (the trait — pure addition in `solvers/`, touches no `gguf_bridge` code yet),
so even the first move is low-risk and reversible.
