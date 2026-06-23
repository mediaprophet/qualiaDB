# STELLAR §A — Performance + Neuro-Symbolic Push: Progress Log

*Branch `0.0.19`. Companion to [`STELLAR_PHENOMENAL_PLAN.md`](STELLAR_PHENOMENAL_PLAN.md) (the plan)
and [`PROJECT_STATUS.md`](PROJECT_STATUS.md) (the snapshot). This file is the **running record of each
executed step** — what was built, the **real measured numbers**, and **⚑ where Timothy's input is
needed** (the things the engine cannot decide for itself). Per the project rule in `CLAUDE.md §9`,
an entry is appended at the end of every step before the next begins.*

*How to read it: each step is dated and honest (regressions and "not measured" included). The **⚑**
markers are the actionable asks — the curation-grade, out-of-band decisions reserved to the human
(machine proposes, human ratifies). Numbers are never extrapolated; a kernel figure is never reported
as an end-to-end figure.*

---

## ⚑ Standing "where I need you" (open across the whole §A push)

These gate *quality acceptance* and *governance content*, not the engineering. A0 + A1a can proceed
without them; A1b's quality sign-off, A5, and A6a need them. None of these can be derived from the
corpus — they come from your principled direction.

1. **Quality-gate eval set (D20)** — a small fixed perplexity corpus **+** a ~25–30-prompt
   governance/persona suite **+** the sentinel/refusal regression canaries. Even a rough seed you'll
   ratify unblocks the quality gate. Format agreed: JSONL `{id, category, prompt, expected_behavior,
   hard_gate}`; `hard_gate` rows must stay binary-pass after any change.
2. **Acceptable quality-loss bar** — confirm the framing (sentinel canaries = hard gate; perplexity =
   soft evidence) and set the ΔPPL number *after* A0/A1 give real baselines. Proposed default keep/reject
   rule: keep a change only if ≥5–10% end-to-end decode gain **or** a better residency class, with zero
   hard-gate regression.
3. **First attested ontology-shard content (A6a)** — the single narrow attested span / reserved control
   token for the first governance proof (e.g. a guard around "override human ratification"). Content is
   yours to attest; mechanism is mine to build.
4. **Performance target stance** — confirm "affordability-driven improvement from the measured
   baseline" (no fixed TPS number) vs a specific threshold you have in mind.

---

## A0 — Native LLM benchmark harness  ·  ✅ DONE  ·  2026-06-23

**What was built**
- [`crates/qualia-core-db/src/llm_bench.rs`](crates/qualia-core-db/src/llm_bench.rs) — the **shared**
  measurement surface. Drives the *real* inference path (`LocalLlmAgent::infer_local_model_streaming`)
  and reads per-phase timing recorded *inside that same path*, so the existing Q8 path and the future
  ternary/top-k paths are measured by **one** harness, not a forked loop (decision D22). Emits
  JSON/CSV + a console table. `set_decode_budget_override(n)` bounds decode to a fixed token count for
  stable, comparable tok/s.
- Phase-timing hooks in [`llm_agent.rs`](crates/qualia-core-db/src/llm_agent.rs) — once per phase
  (load / prefill / decode), **off** the per-token hot path, native-only. The production path now
  self-instruments (a shared improvement, not a benchmark-only fork).
- [`tests/llm_bench_a0.rs`](crates/qualia-core-db/tests/llm_bench_a0.rs) — skip-if-absent integration
  test. It is an **integration** test on purpose: the library then compiles *without* `cfg(test)`, so
  it runs full depth (all 32 layers). A unit test would silently run the 2-layer
  `TEST_TRANSFORMER_LAYER_CAP` and report a fantasy number.
- Run: `cargo test -p qualia-core-db --release --test llm_bench_a0 -- --nocapture`.
- Artifacts: `benchmarks/results/llm_a0_baseline.{json,csv}`.

**Measured results** (A2000 12 GB, DirectML, 64-token decode, 3 warm repeats)

| Model | Layers | Decode tok/s | Cold TTFT | Warm TTFT | Load/call |
|---|--:|--:|--:|--:|--:|
| SmolLM2-360M **Q8** | 32 | **1.52** | 2729 ms | 1100 ms | 408 ms |
| SmolLM2-360M Q4_K_M | 32 | 1.11 | 1460 ms | 1229 ms | 393 ms |

**The honest finding:** native decode is **~1.52 tok/s (~657 ms/token)** — *slower* than the browser
path's 5.9 tok/s. Hypothesis (to confirm in A1a/A2): ~33 serialized GPU round-trips/token (32 layer
dispatches + a full 49 k-vocab argmax readback) with per-token sync stalls. This is the number A1a
(kill the 196 KB readback) and A2 (tiling / fewer dispatches) must beat — we now have a real baseline
instead of an extrapolation.

**Caveats (what the numbers don't mean):** prefill t/s is **not** meaningful — the probe prompt is 5
tokens. Timings are **host wall-clock**, not GPU-timeline isolated. Both are fixed in **A0.2** (long
prompt + GPU `TIMESTAMP_QUERY`, not yet requested on the shared device).

**⚑ Where I need you:** none for A0 — pure instrumentation. (The standing items above remain open for
later steps.)

**Next step:** A1a — GPU top-k reduction (directly attacks the readback this baseline exposed).

---

## A1a — GPU top-k reduction  ·  ⏳ KERNEL DONE (verified on-device); splice + measure NEXT  ·  2026-06-23

**Why this is the right first lever (grounded in A0, not assumption):** A0 showed the output
projection is a major decode cost — `dispatch_output_argmax_chunked` does **6 separate
submit+map+wait stalls per token** (one per 8192-row vocab chunk), each **re-uploading the static
output weight** and **reading back all 49 152 logits** for a CPU argmax (the ~196 KB/token readback).
A1a replaces that with an on-GPU top-K reduction that reads back only K `(id, logit)` pairs.

**What was built (verified core)**
- [`src/shaders/topk_reduction.wgsl`](crates/qualia-core-db/src/shaders/topk_reduction.wgsl) — block
  top-K via K iterative parallel-argmax passes (decision D5: top-K, not a full-vocab sort). NaN→−∞;
  ties→lower id; K=1 == argmax. Each workgroup reduces a 1024-element block to its top-K; host merges.
- [`src/topk.rs`](crates/qualia-core-db/src/topk.rs) — CPU oracle (`topk_cpu`), host merge
  (`merge_block_candidates`, with an optional mask = "a masked/vetoed token is never returned" — the
  governance seam, currently host-side), the WGSL const, params. 3 unit tests.
- [`src/topk_gpu.rs`](crates/qualia-core-db/src/topk_gpu.rs) — native dispatch + on-device parity
  tests, mirroring `ternary_gpu.rs`.

**Measured results (on-device correctness, A2000)** — all 5 tests pass:
- `topk_gpu` == `topk_cpu` at **vocab scale (49 152, k∈{1,32,64})**, byte-exact (1e-5), incl.
  tie-break (two equal maxima → lower id) and NaN exclusion.
- ⚠ **No end-to-end decode tok/s delta yet** — the kernel is verified but not yet spliced into the
  decode loop, so there is nothing to compare against the A0 1.52 tok/s baseline. Per measurement
  honesty, no speedup is claimed until the splice is measured.

**Next sub-step (the measured win):** splice into the decode loop behind `QUALIA_LLM_GPU_TOPK`
(additive; existing argmax path untouched — D22). The real win needs the output-projection logits to
stay on-GPU: deposit each chunk's GEMM result into a **persistent vocab-logits GPU buffer**
(GPU-side copy, no host map), keep the **static output weight resident** (stop the per-token
re-upload A0 exposed), run the top-K kernel over that buffer, read back **K pairs**. Then re-run the
A0 harness `top-k off` vs `on` (attribution matrix) for the first real A1a number.

**⚑ Where I need you:** none yet for the kernel. The top-K **K value** (32 vs 64) and whether to
expose the full top-K to the sentinel/sampler by default are minor knobs I'll default and you can
adjust; flagging so it's visible.

---

## Plan addendum — Adaptive-hardware / memory-routing (AH track)  ·  📐 PLANNED  ·  2026-06-23

**What changed (planning, not yet code):** added decisions **D23–D28** and the **AH track (H0–H4)** to
`STELLAR_PHENOMENAL_PLAN.md` — sense-then-route boot, bounded floors, three residency protocols
(Resident / Streaming / Heterogeneous-overflow), the boot micro-probe + cached **human-key-signed**
hardware passport, and the unified-vs-discrete split. The load-bearing call is **D27**: the human's
sovereign key is the root of trust; a TPM/Secure-Enclave is at most *one optional witness*, never the
root (TPM-as-root re-imports the corporate locus-of-control + a capture surface — the 2001-DRM
topology we reject).

**Honest grounding (real vs aspirational today):** the budget/pressure substrate **exists**
(`VramLedger`, `OperationalMode` Full/Eco/Reserve, `UniverseOrchestrator`, 448 MB KV cap, the Windows
adapter-memory probe). What does **not** exist: multi-adapter enumeration (today `shared_gpu()` grabs
**only** the A2000 via `HighPerformance` and ignores the iGPU + 64 GB), unified/discrete detection,
the micro-probe, the cached passport, the residency *decision*, heterogeneous dispatch, and streaming.

**Topology-shaped expectation (not a claim — to be measured):** discrete desktop ← most gain
(heterogeneous overflow + resident); unified Mac/phone ← no-duplicate mapping + host floors; small
single-GPU ← resident-vs-stream decision + fast-boot. **No numbers yet** — H0–H2 produce the first.

**Next buildable step:** **H0** (host topology sensor) — small, lands on the dev box now, and finally
makes the invisible iGPU + 64 GB visible to the planner. Feeds A4 (streaming) + A7 (fast-boot TTFT).

**⚑ Where I need you:** sequencing call — **H0 now, or finish the A1a decode splice first?** (Both
are unblocked; A1a gives the top-k decode number, H0 starts the hardware-routing foundation.)
*(Resolved: built H0 — see below.)*

---

## H0 — Host topology + capability sensor  ·  ✅ DONE (verified on dev box)  ·  2026-06-23

**What was built**
- [`src/host_topology.rs`](crates/qualia-core-db/src/host_topology.rs) — enumerates **all** wgpu
  adapters across backends (deduped), classifies `device_type` (discrete/integrated/cpu), reads host
  RAM + cores (`sysinfo`/`num_cpus`), pulls discrete VRAM (Windows DXGI via `directml_bridge`), and
  computes the **bounded OS floor** (D24). Emits `HostTopology` (+ `has_heterogeneous_overflow()`).
  Makes **no** routing decision and changes **no** existing behaviour — it's the sensor the H2 planner
  and H3/H5 dispatch will consume. Native-only.

**Measured result (dev box, the one that matters):**
```
HostTopology: Discrete | host RAM 68.5 GB (34.6 GB free) | 8 cores | OS floor 1.6 GB | model budget 10.3 GB | heterogeneous=true
  - Discrete   [Dx12] NVIDIA RTX A2000 12GB        VRAM 11.9 GB
  - Integrated [Dx12] Intel(R) HD Graphics 530
  - Cpu        [Dx12] Microsoft Basic Render Driver
```
**The headline:** the engine now *sees the Intel iGPU + 64 GB that `shared_gpu()` silently ignored* —
the precondition for ever using it (H2/H3). `heterogeneous=true` is correctly detected.

**Honest limitations (documented in-code):** the GL backend reports device-id 0, so a card can appear
twice across backends — fixed with a same-vendor phantom-drop. The deeper one: wgpu exposes no stable
per-card UUID, so **two *identical-model* GPUs collapse to one** (relevant to your old 2×A4000 + 2×A4500
rig and to H5 clusters) — precise identical-card counting needs a per-OS PCI-bus/LUID probe, deferred
to H3/H5.

**⚑ Where I need you:** **H1 is partly gated by the human-key-root issue you flagged.** H1 splits
cleanly: (a) the micro-probe + cached passport (TTFT fast-boot) is buildable **now** and needs nothing
from you; (b) the **human-key signing / trust** layer must wait until the separate human-key-root
problem is resolved (D27 caveat). I'll build (a) and stop before (b) unless you say otherwise.

**Next step:** either **A1a decode splice** (the top-k decode number) or **H1(a)** (probe + cache),
your call — see the report. *(Resolved: built H1(a) benchmark + matrix — see below.)*

---

## H1(a) — Cross-circuit capability benchmark  ·  ✅ BENCHMARK+MATRIX DONE (verified); caching remains  ·  2026-06-23

**What was built**
- [`src/device_benchmark.rs`](crates/qualia-core-db/src/device_benchmark.rs) +
  [`shaders/gemv_bench.wgsl`](crates/qualia-core-db/src/shaders/gemv_bench.wgsl) — runs an identical
  representative GEMV on **every compute circuit** (each wgpu GPU/iGPU, deduped across backends, +
  a native `rayon` CPU path) and returns a `CapabilityMatrix` **ranked by measured throughput**
  (D30). The planner's priority order, from data — not a static "GPU > CPU".

**Measured result (dev box):**
```
CapabilityMatrix (GEMV 2048x2048, ranked by measured throughput; NPU probed=false):
  1. NVIDIA RTX A2000 12GB      [DiscreteGpu/Dx12]   0.407 ms   20.6 GFLOP/s  score 1.000
  2. Intel(R) HD Graphics 530   [IntegratedGpu/Dx12] 7.306 ms    1.1 GFLOP/s  score 0.056
  3. CPU native (rayon, 8 cores)[Cpu/native]        23.879 ms    0.4 GFLOP/s  score 0.017
```
**Reads:** A2000 ≈ **18× iGPU**, ≈ **59× CPU**; iGPU ≈ **3.3× CPU**. So the iGPU is rightly an
*overflow home* (not co-equal), but it **is** the correct overflow target over the CPU — exactly the
data-driven call H2 will make. **Caveat:** naive untiled GEMV → the absolute GFLOP/s is low; these are
**relative ranking** numbers, not peak capability.

**Transfer axis added (D31, Timothy R3):** the matrix now also measures host→device upload:
A2000 **3.3 GB/s** (PCIe), iGPU **1.8 GB/s**, CPU **in-pool** (no transfer). **The crossover, quantified:**
a ~50 MB FFN layer *streamed to the A2000 every token* ≈ 50 MB ÷ 3.3 GB/s ≈ **15 ms/token** (+0.43 ms
compute) vs **6.5 ms in-place on the iGPU with zero transfer** → for overflow weights the **slow iGPU
beats streaming-to-the-fast-GPU**. This validates heterogeneous-overflow (D25/D31) with numbers.
**Caveat:** transfer goes through wgpu's staging path → *relative signals, not raw bandwidth*; the
iGPU's 1.8 GB/s badly **understates** its true near-zero in-pool access (wgpu can't expose unified
zero-copy), so the real heterogeneous win is *larger* than measured. Both axes now in `device_benchmark.rs`.

**Honest scope:** **NPU not probed** (platform-API DirectML/NNAPI/CoreML, not wgpu — recorded as
`npu_probed=false`). The **caching half of H1(a)** (serialize topology+matrix to a passport keyed by the
adapter set + fast-boot skip for TTFT) is **not yet built** — small follow-on. **H1(b) signing/trust
stays blocked** on the identity remediation Phase 2/3.

**⚑ Where I need you:** none for the benchmark. (The standing items + the human-key-root/identity
remediation gating H1(b)/H5 remain.)

**Next step:** the small H1(a) cache, then **H2 (residency + device-priority planner)** which consumes
this matrix — or pivot back to the **A1a decode splice** for the top-k decode number. Your call.
*(Resolved: built H2 — see below.)*

---

## H2 — Residency + device-priority planner  ·  ✅ PLANNER DONE (tested); decode-wiring pending  ·  2026-06-23

**What was built**
- [`src/residency_planner.rs`](crates/qualia-core-db/src/residency_planner.rs) — the **discovery-derived
  employment planner** (D31). A **pure** function `plan_employment(topology, capability_matrix,
  model_bytes, kv_reserve)` → `EmploymentPlan { protocol, device_priority, placements, rationale }`,
  plus `plan_for_model()` that probes real hardware. Chooses **Resident / HeterogeneousOverflow /
  Streaming** (D25) by fit, places overflow in-place on the best large-pool secondary (iGPU before CPU,
  matrix order), and orders devices by *measured* score (D30). Pure → unit-testable with synthetic
  profiles, no GPU.

**Measured result (unit tests, deterministic):** 4/4 pass —
- small model + discrete+iGPU → **Resident** on the discrete GPU;
- 20 GB model, 12 GB VRAM, iGPU present → **HeterogeneousOverflow** (resident on GPU, overflow in-place
  on iGPU);
- 20 GB model, no iGPU → **Streaming** (won't silently dump a 20 GB transformer overflow onto the CPU);
- unified host (no discrete) → **Resident** on the iGPU reading the large host pool (D28).

**Honest scope:** this is the **decision logic only** — the planner is built + tested, but **not yet
wired into the decode loop** (the `QUALIA_LLM_ROUTE` activation). Wiring needs a *target* to route to —
i.e. **H3** (heterogeneous dispatch) and **A4** (streaming) must exist before the plan changes runtime
behaviour. v1 uses the H1(a) measured crossover (iGPU-in-place beats streaming for overflow) as the
rule; the full per-segment `argmin(measured compute + transfer)` is the documented refinement.

**⚑ Where I need you:** none for the planner logic.

**Next step:** H3 (make the plan executable — heterogeneous dispatch) or the A1a decode splice. Plus
the small H1(a) cache. Committing the session checkpoint now (per your instruction).
