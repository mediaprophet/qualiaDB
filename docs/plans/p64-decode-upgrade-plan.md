# P64 + Native Decode Upgrade Plan

**Status:** living plan (must be updated when new opportunities are found)  
**Date:** 2026-07-10 (rewritten end-of-day: progress + beat-Ollama backlog)  
**Principal:** Timothy Charles Holborn  
**Progress log:** [`P64_DECODE_UPGRADE_PROGRESS_LOG.md`](../../P64_DECODE_UPGRADE_PROGRESS_LOG.md)  
**Normative container:** [`docs/manuals/standards/p64-weight-container-standard.md`](../manuals/standards/p64-weight-container-standard.md)  
**Related analysis:** [`docs/reports/inference-performance-analysis-for-fable.md`](../reports/inference-performance-analysis-for-fable.md)  
**Code:** `crates/qualia-core-db/src/q42/p64_weight.rs`, `inference/ggml_quants.rs`, `gguf_bridge/`, `shaders/`, `inference/cuda_lane.rs`

---

## 0. Living-document rule (mandatory)

**If new improvement opportunities are found while implementing, measuring, or reviewing — update this plan in the same session before claiming the phase done.**

1. Append a dated **Opportunity capture** bullet under §11 with: what was found, why it matters, which workstream it belongs to.
2. Adjust acceptance tests if the opportunity changes the success bar.
3. Mirror a one-line entry in the progress log § “Plan deltas”.
4. Do **not** leave opportunities only in chat transcripts or NOTICES without updating this file.

This plan is the authority for “what remaining work exists.” Stale plans are a defect.

---

## 1. Goal (honest)

**Primary product goal:** native in-process decode on named P64 that is **faster than same-host Ollama** on the same class of model/quant, with quality and governance intact.

| Bar | Metric (3B Q4 SoA on RTX A2000 12GB unless noted) | Status |
|-----|--------------------------------------------------|--------|
| **G0** — product not broken | Coherent chat, correct greedy facts, honest tok/s metric | ✅ |
| **G1** — double session-start | ≥ 13 tok/s decode | ❌ (~9.8) |
| **G2** — mid-gap | ≥ 25 tok/s decode | ❌ |
| **G3** — within 2× of Ollama | ≥ 35 tok/s decode | ❌ |
| **G4** — **beat Ollama** | **> ~70 tok/s** decode *and* competitive prefill | ❌ **stretch / design target** |
| **G5** — proof | Effective GB/s ≥ 40% of A2000 weight-bandwidth roofline **or** documented hard ceiling | ❌ (~5% today) |

**How Ollama is used:** same-host diagnostic only (llama.cpp + CUDA). Never as the product runtime. Never as marketing success criteria until G4 is met with named P64 + `decode-proxy`.

**What “revolutionise” means here (not marketing fluff):**

1. **Close the arithmetic intensity gap** — stop shipping one-row-per-WG dequant-GEMV as the steady-state default when hardware can do fused multiproject + TC tiles.
2. **Collapse dispatch graph** — fewer than ~10 dispatches/layer; ideally **one fused layer kernel** (attn + FFN) or a CUDA sticky residual stream with sample-only D2H.
3. **Use the silicon** — tensor cores / WMMA / CUDA where certified; subgroups already help but are not enough.
4. **P64 as a real decode profile** — layout + residency schedule engineered for the hot path, not a GGUF dump with a nicer header.
5. **Keep Qualia-unique value** — Sentinel, VM gates, provenance — *without* paying 10× for them on the GEMV path.

---

## 2. Artifact map (do not conflate)

| Artifact | Magic / form | Role |
|----------|--------------|------|
| **P64** | `p64\0` (`.p64`, `.soa.p64`) | Weight + hparams + tokenizer + 10D manifold + CRC-32C |
| **Q42** | `Q42\0` (`.q42`) | Semantic SuperBlock / Quin graph — not weights |
| **D10** | 64 B `ManifoldCoordinate10D` rows **inside P64** | Layer/global manifold — not a separate LLM file |
| **GGUF / Safetensors** | import sources only | Convert once → P64; engine must not depend on them at decode |

---

## 3. Scoreboard (evidence, not hope)

**Host:** RTX A2000 12GB · Windows · wgpu (Vulkan passport / DX12 default app path)  
**Named model:** `C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64`  
**Command:** `qualia-cli llm decode-proxy <p64> --tokens 32` with `QUALIA_LLM_FFN_FUSION=1`

### 3.1 Decode tok/s (resident mega-pass = product default)

| Stage | tok/s | Delta vs start |
|-------|------:|----------------|
| Session start (2026-07-10 AM) | **~6.7** | — |
| Parallel RMSNorm + no CUDA double-buffer preload | **~8.9** | +33% |
| Full-vocab logits + dual_gemv subgroup | **~9.0–9.1** | +35% |
| Dual multi-row default (rejected) | **~8.75** | lose |
| Full-act LDS GEMV (rejected) | **~8.51** | lose |
| **Barrier-free global-act Q4_SOA (current)** | **~9.7–9.9** | **~+45%** |
| **Ollama diagnostic (same host, Q4_K_M)** | **~70** | **~7× ahead** |

### 3.2 Cost structure (still true after wins)

| Observation | Implication |
|-------------|-------------|
| GPU time ≈ **~95% `fused_block`** | Further wins must densify **layer kernels**, not logits shaving |
| ~**10 dispatches/layer** × 28 layers + logits | Launch/occupancy tax; llama.cpp fuses more aggressively |
| Effective weight BW ≈ **~5% of A2000 peak** | **Not** bandwidth-bound yet → latency / occupancy / under-utilized ALUs/TCs |
| CUDA_DECODE lab path | **~0.7 tok/s** — worse than resident; sticky full-token residual missing |
| Prefill | Dispatch density matched decode; **true M×K×N TC GEMM still open** |

### 3.3 Conclusion

P64 SoA + resident mega-pass + RMS + barrier-free Q4_SOA closed **~1.5×** of a **~10×** gap. Remaining gap is **structural execution**, not “one more multirow flag.” Beating Ollama requires a **different kernel and residency architecture**, not incremental opt-in packing of the current 256-thread single-row GEMV.

---

## 4. Progress — what is done

### 4.1 Container (P64)

| ID | Work | Status |
|----|------|--------|
| **C0** | Stop re-sorting layer-major plan by GGUF offset | ✅ |
| **C1** | Document / code version honesty (v4) | ✅ |
| **C2** | `dtype=112` Q4_K_SOA in standard | ✅ |
| **C3** | Header `reserved` read/write | ✅ |
| **C4** | `P64_FLAG_Q4K_SOA` + `P64_FLAG_LAYER_MAJOR` | ✅ |
| **C5** | Layer schedule table + layer-pack flags | ✅ partial (table + pack exist; product convert re-emit still needed for all models) |
| **C6–C8** | u64 offsets / certified decode profile bits | ⬜ open |

### 4.2 Product decode path (resident wgpu mega-pass)

| Work | Status | Notes |
|------|--------|-------|
| Resident single-submit forward (W1-class) | ✅ | Default product path |
| Fused FFN SwiGLU (`fused_ffn.wgsl`) | ✅ | Default when quant supported |
| Dual K+V GEMV | ✅ | Default SoA |
| Residual-fused O / down | ✅ | |
| Parallel RMSNorm (256-wide) | ✅ | Large win vs scalar |
| Subgroup reduce on dual / FFN / coop_gemv_sg | ✅ | Adapter-gated |
| Full-vocab logits (131072) + multi-row when rows>60k | ✅ | E3 largely closed for 3B |
| Barrier-free global-act Q4_SOA GEMV/FFN/dual | ✅ | Current peak ~9.8 |
| Prefill: one compute pass/chunk + fuse parity | ✅ partial | Structure only; no dense TC GEMM |
| No CUDA weight preload when CUDA_DECODE off | ✅ | Avoids ~1.8 GiB double-buffer |

### 4.3 Lab / CUDA path (not default)

| Work | Status | Notes |
|------|--------|-------|
| Q4_K_SOA CUDA dequant-GEMV + multi-weight preload | ✅ lab | Preload works |
| Device RoPE + KV write + GQA SDPA | ✅ lab | Hits first-hit path |
| `dispatch_async` / fence on readback | ✅ lab | |
| End-to-end CUDA sticky residual (sample-only D2H) | ⬜ | **Required** before CUDA can beat resident |
| CUDA_DECODE ≥ resident tok/s | ❌ | ~0.7 vs ~9.8 — **lab-only** |

### 4.4 Rejected as default (keep opt-in for future hardware)

| Experiment | Result | Env opt-in |
|------------|--------|------------|
| Multi-row residual / FFN multi-row | lose or tie | `QUALIA_LLM_MULTIROW`, `QUALIA_LLM_FFN_MR` |
| Dual multi-row (4 rows) | lose ~8.75 | `QUALIA_LLM_DUAL_MR` |
| Triple QKV | lose ~8.57 | `QUALIA_LLM_TRIPLE_QKV` |
| Full-act / 4k-chunk LDS act | lose ~8.51 | (reverted) |
| Warp GEMV as default | not proven win | `QUALIA_LLM_WARP_GEMV`, `QUALIA_LLM_FFN_WARP` |

**Rule:** no packing scheme becomes default without **two** consecutive A/B wins on named 3B SoA.

---

## 5. Gap analysis — why Ollama is still ~7× faster

Ollama/llama.cpp on this box is not “magic”; it is a different roofline utilization story:

| Factor | Ollama / llama.cpp (typical) | Qualia today |
|--------|------------------------------|--------------|
| Backend | CUDA native, mature kernels | wgpu portable (Vulkan/DX12) + optional CUDA lab |
| Quant GEMV | Heavily tuned CUDA Q4_K, often multi-row / mmq | 256-thread single-row dequant-GEMV, barrier-free act |
| Tensor cores | Used where profitable | Forge WMMA exists; **not** on hot decode path |
| Layer fusion | Aggressive (attn/FFN kernels) | ~10 separate dispatches/layer in one pass |
| Prefill | True batched GEMM | Per-token GEMV shape even when B>1 structure improved |
| Memory path | Single residency story | Resident wgpu good; CUDA double path still immature |
| Occupancy | Tuned for A100/consumer | Full-act LDS proved occupancy-sensitive on A2000 |

**Arithmetic sketch (order-of-magnitude):**  
3B Q4 ≈ ~1.5–2 GB weight touch/token if naive full-pass; A2000 has hundreds of GB/s. At ~10 tok/s we are nowhere near BW-limited. Beating 70 tok/s requires **~7× more useful work per second** from the same GPU — i.e. **kernel throughput**, not “read weights harder.”

---

## 6. Comprehensive worklist — resolve · improve · revolutionise

Work is grouped by **expected impact on G4 (beat Ollama)**. Within a band, order is preferred implementation sequence. Every item: **files / acceptance / risk**.

### Band R — Revolution (required to approach or pass ~70 tok/s)

These change the architecture. Incremental GEMV polish alone will not hit G4.

| ID | Work | Why it matters | Primary files | Acceptance |
|----|------|----------------|---------------|------------|
| **R1** | **Fused decode layer kernel (attn+FFN) or ≤3 dispatches/layer** | Cut launch tax; keep weights/acts hot in L2 | `resident_decode.rs`, new `fused_layer.wgsl` / CUDA | Dispatches/layer ≤ 3; tok/s ≥ G2 (25) |
| **R2** | **CUDA sticky full-token residual stream** — all 28 layers on device; **D2H only for sample/logits** | CUDA path currently loses to fences/host residual | `cuda_lane.rs`, attention/FFN CUDA, `resident_decode` plan fork | CUDA_DECODE ≥ resident; then ≥ G2 |
| **R3** | **Tensor-core / mmq-class Q4_SOA GEMM for decode M=1 and prefill M=B** | Use ALUs/TCs; close intensity gap | `wgsl_forge` WMMA, `emit/cuda_c`, `gemm`, shaders | Microbench GEMV µs ≤ 0.4× current coop; end-to-end ≥ G3 |
| **R4** | **Flash-style / fused SDPA decode** (GQA) with persistent KV layout matched to kernel | Attention is non-trivial share of fused_block | `fused_attention.wgsl`, CUDA `sdpa_decode_gqa` | Attn phase ≤ 25% of fused_block time on timeline |
| **R5** | **Decode-profile P64 vNext** — layer blobs + optional pre-repacked mmq tiles + certified profile bits | Layout for the kernel we actually run | `p64_weight.rs`, convert CLI, standard | Convert 3B; activate; G2 without hand flags |
| **R6** | **Persistent warm engine / daemon measure** (not cold CLI) | Steady-state = product reality | CLI daemon, passport, decode-proxy warm | E5: warm within 10% of peak |

**Revolution exit:** G3 met on named 3B SoA; G4 attempted with GB/s proof. If G4 fails, document **hardware ceiling** with instruments (honest E6).

---

### Band H — High impact (likely +2–4× if R-band unblocked)

| ID | Work | Why | Files | Acceptance |
|----|------|-----|-------|------------|
| **H1** | **Q-projection density** — ensure Q is not left on a slower path than K/V; consider fused QKV that *wins* A/B (triple currently loses) | Q path historically under-optimized | `resident_decode`, dual/triple, attention | Q GEMV ≤ K time; no tok/s regression |
| **H2** | **Fused residual SwiGLU+down** (vertical FFN) without recomputing expansion | One less global intermediate + dispatch | `fused_ffn.wgsl` + new down fuse | −1 dispatch/layer; ≥ +5% tok/s |
| **H3** | **Prefill true M×N×K GEMM** for B≥8 (and B≥2 if profitable) | Prefill still GEMV-shaped | `prefill_arena`, forge MatMul, CUDA TC | Prefill tok/s ≥ 5× current short-prompt path |
| **H4** | **Int8 / FP8 KV** with correct device SDPA | Lower KV BW for long context | KV layout, SDPA, CUDA_DECODE | Quality parity greedy; long-ctx speedup |
| **H5** | **Weight staging: single residency** (wgpu buffer *or* CUDA, never both unless decode path is CUDA) | Avoid memory pressure / thrash | `resident_decode`, cuda preload | Peak VRAM −1.8 GiB when not CUDA_DECODE |
| **H6** | **Pipeline cache + shader warm always-on** for product activate | First-token / cold start | `pipeline_cache`, init | Cold→warm gap documented; second run flat |
| **H7** | **Logits sampled path** — never materialise full 128k when top-k/sampler can early-exit | Long-tail vocab tax | `output`, topk, sample path | Logits < 10% of forward when sampling |

---

### Band M — Medium (quality of implementation; 10–40% class)

| ID | Work | Why | Notes |
|----|------|-----|-------|
| **M1** | Warp/FFN-warp **passport auto-select** if A/B wins on device | Already coded, never default | Measure on A2000 + other GPUs; store in passport |
| **M2** | Vectorized nibble loads / scale broadcast in Q4_SOA | ALU/memory micro-opts | Only after R3 baseline; easy to lose to occupancy |
| **M3** | Fuse RMS into residual write tail where legal | −1–2 dispatches/layer | Must preserve numerics |
| **M4** | Attention: RoPE fused into Q/K GEMV write | −dispatches | WGSL + CUDA both |
| **M5** | Dual-issue: overlap next-layer weight prefetch with current FFN | Hide latency | Needs schedule table (C5 complete) |
| **M6** | Multi-GPU / multi-stream — **out of scope** until single-GPU G3 | Distraction | Do not start |
| **M7** | Vulkan hang on large Q4 (historical) fully closed + CI gate | Reliability | DX12 default stays; Vulkan must not regress product |
| **M8** | Backend auto: DX12 vs Vulkan vs CUDA by passport rank | Pick fastest certified | `hardware_passport`, `gpu_context` |

---

### Band L — Lower / hygiene (do not pretend these beat Ollama)

| ID | Work | Why still do it |
|----|------|-----------------|
| **L1** | Re-convert all production models with layer-pack + schedule flags | Enables H5/M5 |
| **L2** | C6 u64 offsets / multi-volume for >4 GiB models | 7B+ readiness |
| **L3** | C8 certified decode-profile header bits | Fail-closed activate |
| **L4** | Metric honesty: never report provenance-length as tokens | Already partially fixed; audit all surfaces |
| **L5** | Timeline labels: per-op inside fused_block (attn vs ffn vs rms) | Directs R1/H2 |
| **L6** | Continuous A/B harness (`llm explore`) on named P64 matrix | Prevent silent regression |
| **L7** | Document rejected opts in passport “do-not-default” list | Stops re-litigation |
| **L8** | Quality: chat template + sampler defaults on all entry points | Speed without coherence is failure |

---

### Band X — Explicit non-goals (for this plan)

| Non-goal | Reason |
|----------|--------|
| Replacing product runtime with Ollama/llama.cpp HTTP | Violates architecture; principal directive |
| Claiming layout alone beats Ollama | Already disproven (~6.7→9.8 still 7×) |
| Default multirow/full-act without A/B | Measured losses |
| Fake tok/s / comparing to different model sizes | Audit-grade honesty |
| Spending the gap on governance theatre mid-GEMV | Sentinel stays; must not dominate forward |

---

## 7. Phased delivery (revised for beat-Ollama)

| Phase | Focus | Exit |
|-------|-------|------|
| **P0–P2** | Plan, container C0–C4, baseline lock | ✅ done (~6.7 baseline recorded) |
| **P3** | CUDA lab chain | ◑ kernels exist; **not** competitive — continue only as R2 |
| **P4** | Resident density + prefill structure + E3 | ◑ **~9.8 tok/s**; E4 GEMM open |
| **P5** | **R1 + H1–H2 + L5** — densest possible *current* mega-pass | G1 (13) then push G2 (25) **without** requiring CUDA win |
| **P6** | **R3 + H3** — TC/mmq GEMM decode + prefill | G2–G3; prefill competitive |
| **P7** | **R2 + R4** — CUDA sticky or fused device attention that **beats** resident | Default backend = winner of A/B |
| **P8** | **R5 + R6 + G4 attempt** — decode-profile P64 + warm product path + beat-Ollama proof or ceiling doc | G4 or honest E6 ceiling |

**Parallelism rule:** P5 (WGSL resident) and P6/P7 (CUDA/TC) may run in parallel **if** lanes claimed in `coordination/NOTICES.md`. Do not double-edit `resident_decode.rs` without claim.

---

## 8. Recommended next 5 actions (concrete)

1. **L5 — split fused_block timeline** into attn / ffn / rms / logits (timestamps). *Know* where the 95% is.  
2. **H2 — fused down into FFN residual** (one dispatch cut) + measure.  
3. **R1 spike — single-layer fused kernel** prototype for one Llama block (even if only FFN+RMS first).  
4. **R3 spike — wire existing forge WMMA/CUDA GEMM** to one projection (e.g. down or logits) on CUDA backend; A/B vs coop GEMV.  
5. **R2 design — sticky residual buffer protocol** (no per-layer D2H); implement only if spike shows ≥1.5× resident on one layer.

Do **not** spend another cycle on multirow packing or full-act LDS unless a new GPU passport shows different occupancy.

---

## 9. Measurement contract

Always name the file:

```text
C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64
C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64
```

```powershell
$env:QUALIA_INFERENCE_MODE='cuda'   # mode label; product path still resident unless CUDA_DECODE=1
$env:QUALIA_LLM_FFN_FUSION='1'
.\target\release\qualia-cli.exe llm decode-proxy <model.p64> --tokens 32
.\target\release\qualia-cli.exe llm lab timeline --model <model.p64> --tokens 16
```

**A/B rules:**

- Two runs minimum; report min–max.  
- Reject default change on any regression > 2% or quality fail.  
- Ollama numbers are diagnostic; record command + model tag when citing.

**Roofline honesty (G5):**

- Report effective GB/s = (bytes of weights+KV touched per token × tok/s) / 1e9.  
- Compare to adapter peak from passport / vendor.  
- If tok/s high but GB/s low → still compute-bound (good: R3 still open). If GB/s high and tok/s low → investigate stalls.

---

## 10. Implementation status (update in place)

| Phase | Status | Notes |
|-------|--------|-------|
| P0 | **done** | Living plan + progress log |
| P1 | **done** | C0–C4 + layer-pack + schedule (C5 partial) |
| P2 | **done** | Baseline ~6.7 → now re-lock at **~9.8** |
| P3 | **partial / lab** | CUDA ~0.7; sticky residual open (→ R2) |
| P4 | **partial** | Resident **~9.7–9.9**; E3 done; E4 structure done; dense GEMM open |
| P5 | **next** | Fused_block split + FFN vertical fuse + dispatch collapse |
| P6–P8 | **pending** | TC/mmq, CUDA sticky, beat-Ollama or ceiling |

**Current default product config (do not “fix” without A/B):**

- Resident mega-pass ON  
- FFN fusion ON  
- Dual K+V ON (SoA)  
- Single-row GEMV + barrier-free global act  
- Multirow / dual_mr / triple / warp / CUDA_DECODE **opt-in only**

---

## 11. Opportunity capture (append-only)

<!-- New findings go here. Do not delete old entries. -->

- **2026-07-10 — C0:** layer-major plan sorted by GGUF offset — fixed in P1.
- **2026-07-10 — C1:** version 3 vs code v4 — synced.
- **2026-07-10 — E-lab:** WGSL multirow default-on A/B lost (~6.36 vs ~6.68). Opt-in only.
- **2026-07-10 — P64 pack:** layer-pack + `P64LayerScheduleEntry` — bulk residency units.
- **2026-07-10 — CUDA preload:** ~168 SoA matrices; still not enough without sticky residual.
- **2026-07-10 — CUDA_DECODE:** ~0.74 tok/s; bottleneck host residual / non-sticky chain.
- **2026-07-10 — P4 E2 device attention:** RoPE/KV/SDPA CUDA present; path hits; still ~0.7 tok/s.
- **2026-07-10 — Default path:** parallel RMSNorm + no CUDA preload when off → ~6.7→~8.9.
- **2026-07-10 — E3:** full-vocab logits 131072; multi-row forced when rows>60k.
- **2026-07-10 — dual_gemv subgroupAdd:** peak ~9.0–9.1 with full logits.
- **2026-07-10 — Timeline:** fused_block still ~95%; logits not limiter.
- **2026-07-10 — E4 prefill:** one pass/chunk + fuse parity; dense M=B GEMM still open.
- **2026-07-10 — dual_mr:** lose ~8.75; opt-in only. BW ~5% peak → occupancy/latency wall.
- **2026-07-10 — full-act LDS:** lose ~8.51; occupancy kill on A2000.
- **2026-07-10 — barrier-free global-act Q4_SOA:** **~9.7–9.9 tok/s** current peak; multirow stays opt-in.
- **2026-07-10 — plan rewrite:** progress consolidated; Band R/H/M/L worklist aimed at **>Ollama (~70)** with honest G0–G5 bars; next actions L5→H2→R1/R3 spikes.

---

## 12. References

- Progress log: [`P64_DECODE_UPGRADE_PROGRESS_LOG.md`](../../P64_DECODE_UPGRADE_PROGRESS_LOG.md)
- Cost path: `experiments/inference-lab/cost-path-*/COST_PATH_REPORT.md`
- Opt pass honesty: `experiments/inference-lab/lockin-dramatic/OPT_PASS_HONEST.md`
- Prior remediation: `docs/plans/native-inference-p64-pipeline-remediation.md`
- Lab instruments: `docs/plans/inference-superiority-lab-and-toolset-plan.md`
- Fable analysis: `docs/reports/inference-performance-analysis-for-fable.md`
- Multi-mode plan: `docs/plans/inference-multi-mode-and-compression.md` (if present)
- AGENTS.md inference rules: no Ollama backend; Phase-8 Sentinel preserved

---

## 13. One-page summary for the principal

| | |
|--|--|
| **Now** | ~**9.8 tok/s** on named 3B SoA (was ~6.7 same day) |
| **Ollama diagnostic** | ~**70 tok/s** same host |
| **Gap** | ~**7×** — kernel/dispatch architecture, not container trivia |
| **Proven wins** | RMSNorm parallel, no CUDA double-buffer, subgroup dual, full logits, barrier-free Q4_SOA act |
| **Proven losses** | multirow, dual_mr, triple QKV, full-act LDS, CUDA_DECODE as-is |
| **To beat Ollama** | Fused layer kernels + TC/mmq GEMM + sticky device residual (or a resident path that matches that density) |
| **Do next** | Split fused_block timeline → fuse FFN vertical → spike fused layer + TC GEMM |

*Speed without coherence is failure. Coherence without speed is not competitive. Both are required for G4.*
