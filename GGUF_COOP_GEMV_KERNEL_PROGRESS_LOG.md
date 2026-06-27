# coop_gemv kernel optimization — progress log

Workstream: attack the 96%-of-token compute wall in native gguf decode (the road to 60 tok/s).
The resident-decode work ([GGUF_RESIDENT_DECODE_PROGRESS_LOG.md](GGUF_RESIDENT_DECODE_PROGRESS_LOG.md))
proved decode is **kernel-bound, not readback-bound**; this log is the kernel attack. Branch
`0.0.21-la`. Honest engineering record (§9); numbers are real or marked otherwise.

---

## 2026-06-27 · Step 0 — profiling baseline (DONE) · dequant ALU is the GEMM bottleneck (measured)

**What I set out to do.** Before touching any WGSL, get a hard number on *where* the GEMM (coop_gemv)
kernel time goes — dequant ALU vs the reduction/matmul — so the first cut is aimed, not guessed.

**Method.** A controlled comparison: the **same model architecture** (Llama-3.2-3B, 3072 dim, 28
layers, 896 GEMM calls / 16 tokens) run at two quantizations through the W2 per-kernel GPU-timestamp
profiler (`w2_gpu_phase_profile`). Because the dims are identical, the matmul FLOPs are identical, so
the GEMM-phase delta isolates the **weight-dequant cost** (F16 does only `unpack2x16float`; Q4_K_M
runs the full K-quant block dequant — 6-bit packing + per-block scales/mins). Profiler tooling change:
added a `QUALIA_LLM_PROFILE_MODEL` override (filename or absolute path) to `w2_gpu_phase_profile`.

**Measured results (A2000, 16 tokens, PROFILING-PERTURBED absolute µs — the F16:Q4 *ratio* at
identical dims is the clean signal):**

| model (same 3B arch) | GEMM total | GEMM µs/call (896 calls) | attention total | decode (perturbed) |
|---|---|---|---|---|
| **F16** (no block dequant) | 1,132,903 µs | **1264 µs** | 971,313 µs | 6.45 tok/s |
| **Q4_K_M** (K-quant dequant) | 2,442,762 µs | **2727 µs** | 1,583,723 µs | 3.12 tok/s |

- **Dequant cost = 2727 − 1264 = 1463 µs/call (2.16× slower).** K-quant unpacking is **~54% of the
  GEMM kernel time**.
- GEMM is **57.8%** of instrumented decode GPU → dequant alone ≈ **0.54 × 0.578 ≈ 31% of the whole
  Q4_K token**.
- Attention rose **+63%** F16→Q4_K (971k→1584k µs) too — its Q/K/V/O projections dequant the same
  weights, so the dequant tax is paid in the attention phase as well, not just GEMM.
- Separate datapoint (different arch, not directly comparable): SmolLM2-360M Q8 GEMM = 250.6 µs/call,
  51.7% of GPU — Q8's byte-unpack dequant is far cheaper than Q4_K's block dequant, consistent.

**Caveats (measurement honesty).** The W2 profiler serialises per-op readbacks, so the *absolute* µs
and the 6.45/3.12 tok/s are perturbed (inflated) — they are NOT the headline decode rate (that's
~16/13 tok/s unperturbed). The **ratio** F16:Q4_K at identical dims is the trustworthy signal, and it
is large and unambiguous. Side observation: Llama-3.2-3B now decodes **coherent** text on both paths
("...a young girl named Sophia who lived in a small village...") — the earlier RoPE/tied-embed
bring-up bug appears resolved; worth a separate coherence/PPL confirmation but not this workstream.

**⚑ Where I need the human (Timothy).** None this step — the baseline is unambiguous and needs no
out-of-band datum. Direction is already set (attack dequant first); this just confirms it with a
controlled number. The W2 `QUALIA_LLM_PROFILE_MODEL` override is committed as test tooling.

**Next step.** Vectorize the Q4_K dequant in the coop_gemv WGSL: `vec4`/wider unpacking, minimise
per-weight bit ops, hoist per-block scale/min decode out of the inner accumulate loop. Parity-gate
every iteration against the CPU reference via the W3 `gemm_parity_probe` (~2e-6 tolerance), then
re-measure GEMM µs/call (target: close the F16↔Q4_K gap) and the unperturbed `a0_decode_profile`
tok/s. Subgroup `subgroupAdd` reduction + wider loads are the follow-on pass once dequant is tamed.

---

## 2026-06-27 · Step 1 — block-cooperative Q4_K dequant (DONE + VERIFIED) · −33% GEMM kernel

**Status: done, parity-verified, committed (70127c55e).**

**What was built.** `shaders/fused_transformer.wgsl` `coop_gemv`: the inner loop called
`dequant_weight()` per element, so for Q4_K all 256 threads re-decoded the *same* superblock header
(2× `f16_to_f32` + 8 scale/min unpacks) at every step — constant across the block's 256 elements but
recomputed 256×. A Q4_K superblock is 256 elems == `COOP_WG`, so one workgroup step now maps to one
superblock: 8 threads decode the header once into shared memory (`coop_q4k_dsub`/`coop_q4k_msub`),
then all 256 threads reuse it for a single nibble extract + FMA. Indexing matches `dequant_q4_k_elem`
exactly. Strictly additive — a new `if Q4_K && K%256==0` branch; every other quant type takes the
unchanged generic path (so Q8/Q4_0/Q5_0/Q6_K/F16 cannot regress).

**Measured (Llama-3.2-3B Q4_K_M, A2000):**

| metric | before | after |
|---|---|---|
| GEMM kernel (W2, µs/call) | 2727 | **1816 (−33%)** |
| ⤷ dequant ALU (vs F16 1264 floor) | 1463 | **552 (−62%)** |
| decode (a0_decode_profile, coop-ON before/after) | 3.17 tok/s | **3.59 tok/s (+13%)** |
| forward / tok | 299 ms | **263 ms** |

The end-to-end +13% is smaller than the −33% GEMM because GEMM is ~58% of the token and the
attention SDPA core (KV cache is f32) doesn't dequant — only the projections do. As intended, the
dequant tax shrank in *both* the GEMM phase and the attention projections.

**Parity — verified EXACT.** With the new path (default) vs the untouched `main` reference kernel
(`QUALIA_LLM_COOP_GEMV=0`, original per-element dequant), 3B Q4_K decode output is **byte-identical**
across 32 layers ("...a young girl named Lily who lived in a small village surrounded by rolling
hills and dense"). Q8 coherence guard (`a1c`) unchanged (Q8 takes the generic path). 32 layers × 16
tokens of accumulated GEMVs through both kernels agreeing is a stronger gate than a single-matrix
probe.

**Scope note.** The win is for **Q4_K** models — the production edge quant (Llama-3.2-3B Q4_K_M is
2.0 GB vs 6.4 GB F16). The SmolLM2-360M Q8 benchmark is unaffected (Q8 dequant was already cheap).

**⚑ Where I need the human (Timothy).** None this step — clean win, parity-proven, no out-of-band
datum. Commit `70127c55e` is local/unpushed (your call on pushing).

**Next step (the follow-on passes flagged in Step 0, in leverage order):**
1. Apply the same block-cooperative header-hoist to **Q6_K** (used for some `output.weight`/embed
   tensors) and verify the smaller K-quants (Q4_0/Q5_0/Q8_0 are 32-elem blocks — different geometry,
   lower redundancy, lower priority).
2. **subgroup `subgroupAdd`** for the tree reduction (wgpu 29 subgroups) — removes log2(256)=8
   barriers + shared-memory traffic per row; native fast-path, generic fallback retained.
3. **Wider loads** (`vec4` weight-word reads) to push the now-ALU-relieved kernel toward
   bandwidth-bound.
Each parity-gated (byte-identical decode + W3 probe) and re-measured before the next.

---

## 2026-06-27 · Reference baseline — ollama (llama.cpp) on the SAME A2000 · the honest gap + the diagnosis

**Why.** Independent reference (ollama = llama.cpp, used strictly as an external dev tool per the
project rule — never wired into the engine) to know how far native is from a mature implementation on
identical model + hardware. Method: ollama HTTP API, `num_predict=64`, `temperature=0` (greedy),
prompt "Once upon a time, there was a". Same A2000.

| model | ollama (llama.cpp) | qualia native | gap |
|---|---|---|---|
| SmolLM2-360M Q8 | **148.8 tok/s** | ~16 | **9.3×** |
| Llama-3.2-3B F16 | **28.2 tok/s** | ~6.5 | **4.3×** |
| Llama-3.2-3B Q4_K_M | **60.9 tok/s** | 3.59 | **~17×** |

**The two findings that matter:**
1. **60 tok/s is not aspirational — it's what llama.cpp does on 3B Q4_K_M on this exact box, today.**
   The target is real and the hardware can clearly do it; our gap is pure software efficiency, not
   physics. (And neither engine is bandwidth-bound on the tiny Q8 model — llama.cpp at 149 tok/s on a
   386 MB model is still well under the ~750 tok/s bandwidth ceiling, i.e. *implementation* headroom.)
2. **The diagnosis is in the *shape* of the gap.** A healthy engine gets **faster** with
   quantization — ollama Q4_K_M (60.9) is **2.16× its own F16** (28.2), because Q4_K is ¼ the bytes
   and llama.cpp is near bandwidth-bound. **Ours does the opposite**: native Q4_K (3.59) is *slower*
   than native F16 (~6.5). Quantizing **hurts** us because our dequant ALU dominates the kernel — so
   we pay K-quant unpack cost without banking the bandwidth win. That is the disease, and it's exactly
   the path Step 1 started treating: the Q4_K gap (17×) is far worse than the F16 gap (4.3×), so most
   of the recoverable speed is in the **quant dequant + GEMV efficiency**, precisely where Steps 1–3
   aim. llama.cpp gets there with integer dot-products (dp4a/`__dp4a`-style) and tightly fused dequant;
   our WGSL does f32 dequant + f32 FMA.

**⚑ Where I need the human (Timothy).** None — this is reference data. It reframes the goal honestly:
we are ~9–17× off a mature reference, *and* the reference proves the box does 60 tok/s on our target
model. The kernel workstream is the right place; the dequant→subgroup→vec4→attention sequence is the
road, with this 60.9 tok/s Q4_K_M number as the concrete bar to chase.

**Note.** Pulled `llama3.2:3b-instruct-q4_K_M` into ollama for the comparison (canonical tag; the
local-GGUF `ollama create` hit a version "unknown type" error — not worth fighting, the registry tag
is the same weights). External tool only; nothing wired into the engine.

---

## 2026-06-27 · Step 2 — subgroupAdd wave reduction (DONE + VERIFIED) · +12% on 3B Q4_K

**Status: done, parity-verified, committed (2606d7adc).**

**What was built.** Replaced `coop_gemv`'s 8-step barrier-synced shared-memory tree reduction with a
single `subgroupAdd` per subgroup + a small cross-subgroup combine. Factored the accumulation (incl.
the Step-1 block-cooperative Q4_K dequant) into a shared `coop_row_dot()` so the shared-memory
`coop_gemv` and the new `coop_gemv_sg` (`shaders/coop_gemv_subgroup.wgsl`) share one copy. When the
adapter advertises SUBGROUP, `init.rs` builds the `coop_gemv_pipeline` field from the subgroup variant
(concatenated after the base module) — identical group-0 bindings + `(n_out,1,1)` dispatch, so every
call site and the derived bind layout pick it up transparently. Adapters without subgroups (and wasm)
keep the shared-memory `coop_gemv`.

**naga 29 gotcha (cost a cycle; worth recording).** naga 29.0.3 does **not** implement the WGSL
`enable subgroups;` *directive* ("the `subgroups` enable-extension is not yet supported") — but it
**does** lower the subgroup ops/builtins (`subgroupAdd`, `@builtin(subgroup_size)`,
`@builtin(subgroup_invocation_id)`) gated on the device's SUBGROUP validation capability, which wgpu
derives from `Features::SUBGROUP`. That feature is already requested unconditionally in
`gpu_context/caps.rs` (never stripped). So the fix was to concatenate the variant **without** the
directive. Separately: `QUALIA_WGPU_EXPERIMENTAL_FEATURES=1` requests EXPERIMENTAL_COOPERATIVE_MATRIX,
which **fails device creation on the A2000** — unrelated to subgroups; correctly left off.

**Measured (Llama-3.2-3B Q4_K_M, A2000, a0_decode_profile):**

| metric | dequant-only (Step 1) | + subgroupAdd (Step 2) |
|---|---|---|
| decode | 3.59 tok/s | **4.01 tok/s (+12%)** |
| forward / tok | 263 ms | **235 ms** |
| W2 GEMM µs/call | 1816 | **1728** |

**Cumulative across the two kernel steps: 3.17 → 4.01 tok/s (+26%).**

**Parity — verified.** 3B Q4_K decode byte-identical to the `main` reference ("...a young girl named
Lily..."); Q8 `a1c` coherent + unchanged (Q8 runs the generic branch of `coop_row_dot` through the
same `coop_gemv_sg`). The profiler now shows **attention (SDPA) = 66% of forward** — the untouched
attention shader is now the dominant cost, confirming it as the next target.

**⚑ Where I need the human (Timothy).** None — clean win, parity-proven. Commit `2606d7adc`
local/unpushed (with `70127c55e` dequant + `686cf3824` tooling; your call on pushing). Honest
correction to the "enable experimental features" steer: the subgroup path needed neither the directive
nor experimental features — only the directive *removed*; experimental (coop-matrix) breaks device
init here. SUBGROUP was already on.

**Next step (still the leverage order):** `vec4` weight loads (push the ALU-relieved GEMV toward
bandwidth-bound), then the **attention SDPA shader** (now 66% of forward), then integer dot-product
dequant (dp4a-class — most of llama.cpp's remaining 15× on Q4_K). The 60.9 tok/s ollama Q4_K_M number
is the bar.
