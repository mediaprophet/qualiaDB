# WASM LLM Endgame — Open Questions Before Phase 2B TTFT Work

**Date:** 2026-06-19 · **Owner:** Qualia · **Author:** engineer (pre-implementation)
**Companion docs:** [`WASM_LLM_ENDGAME.md`](WASM_LLM_ENDGAME.md) ·
[`wasm_llm_planning.md`](wasm_llm_planning.md) · [`WASM_LLM_INFERENCE_DIAGNOSIS.md`](WASM_LLM_INFERENCE_DIAGNOSIS.md)

This document lists every open question I have **before** touching the MC8 kernel for the
Phase 2B TTFT push. Each question states my current hypothesis/recommendation so the
architect can confirm or redirect (matching the `Q | Decision` style of the planning log
§6/§8). Nothing in `gguf_bridge.rs` will change until these are answered.

---

## 0. State as I read it (for grounding, not a question)

- **Coherence:** locked — `Paris. The capital of France…` on `WASM_ASYNC=1`, naked prompt. ✅
- **TTFT:** ~5.6–6.1s vs **<4500ms** gate. ❌ Gap ≈ **1.1–1.5s**.
- **Submit floor:** 2/layer = 64 submits/prefill chunk — one KV-visibility flush + one
  layer-end flush. `mc8_flush` = `submit + new encoder`
  ([gguf_bridge.rs:441](crates/qualia-core-db/src/gguf_bridge.rs#L441)).
- **Empirical submit cost:** Part 3n→3o removed ~128 per-Q flushes and saved ~1.4–2.3s ⇒
  **~11–18 ms per submit**. So the remaining 64 submits are only **~0.7–1.1s** of the 5.6s.

---

## 1. STRATEGIC FORK — what do we optimize first?

**The core tension I want a ruling on.** At ~15 ms/submit, the entire submit budget is
~1s of a ~5.6s TTFT. The other **~4.5s is real GPU compute + readbacks** (per-chunk vocab
GEMM, CPU argmax, `pipeline_read_*` `map_async` round-trips) and has never been attributed
in the log. Two consequences:

- Even **zero** submit overhead lands at ~4.5s — *borderline at the gate*.
- Cross-layer merge (Part 3v) is **capped at ~0.4s** while the per-layer KV flush survives
  (it can only coalesce the layer-end flush, 64→~33), so `Mc8NormWeightArena` (next-step #1)
  unblocks a merge that probably **won't cross the gate alone**.

**Q1.1** — Should I **profile/attribute the ~5.6s first** (instrument prefill + first-decode
to split submit-overhead vs GPU-compute vs vocab-GEMM/argmax readback) before any more flush
surgery? *My recommendation: **yes** — it's cheap and decides whether submit reduction can
even reach 4500ms.*

**Q1.2** — If profiling shows non-submit cost dominates (≳4.5s), is the **vocab GEMM + argmax
readback path** in scope for this gate? Today `dispatch_output_argmax_chunked_async` →
`_mc8_fused` does **per-chunk** `dispatch_gemm_raw_into_async` (submit + `map_async` readback
each chunk; Part 3l). For SmolLM2 vocab ≈ 49152 that's several submit+readback round-trips on
the *first token's* critical path. *Hypothesis: this is a large, untracked slice of TTFT.*

**Q1.3** — Is the **<4500ms gate measured on the first token only** (prefill + 1 decode +
argmax), or does it include warm-up/first-paint? Confirming the denominator changes what
"profile" must measure.

---

## 2. THE KV-FLUSH BLOCKER — root cause is suspect

The batched-Q prefill path ([encode_attention_batched_q_prefill:1090](crates/qualia-core-db/src/gguf_bridge.rs#L1090))
already uses **dynamic-offset uniforms** (`mc8_dynamic_uniform_binding` + `attn_dyn_offset`)
and a **role-specific `AttnQ` weight buffer** — so the uniform/weight races (pt3c/pt3n) are
gone there. And **each K/V/Q encode opens its own compute pass** in the single encoder.

Per the WebGPU memory model, separate compute passes in one encoder get an **automatic
read-after-write barrier** on the KV storage buffer. So the K/V→Q `mc8_flush` *should* be
unnecessary — yet removing it still yields `is is…` (Part 3u). That contradiction tells me
the "KV storage write visibility" attribution is likely **masking a different hazard the
submit incidentally serializes.**

**Q2.1** — Before any fix, may I run a **targeted KV-staleness probe**: remove the K/V→Q
flush, then `pipeline_read_kv_head` for layer 0 vs layer 1 to confirm whether KV is *actually*
stale, or whether the corruption is downstream in the **FFN-tail buffers**
(`work_a`/`work_b`/`prefill_scratch`)? *This is read-only instrumentation, reverted after.*

**Q2.2** — Is the prime suspect the **sub-range KV binding** (`offset: layer_offset, size:
layer_bytes` at [:1120](crates/qualia-core-db/src/gguf_bridge.rs#L1120)) defeating Dawn's
whole-buffer hazard tracking? If so, would you accept testing a **full-buffer KV binding**
(bind whole `kv_cache_gpu`, index by `layer_offset` in-shader) to see if the auto-barrier
returns — eliminating the flush without a staging copy?

**Q2.3** — If it turns out to be **FFN-tail aliasing** (not KV), the documented "KV staging
copy" (next-step #2) is the wrong fix. Preferred direction in that case: **disjoint FFN-tail
scratch arena** (mirroring `Mc8WeightArenaBufs`) vs. inserting explicit pass boundaries?

---

## 3. SCOPE, RISK & SEQUENCING

**Q3.1** — Acceptable **risk budget** for this push: am I allowed to attempt a change that
*may* regress coherence (caught by the harness) as long as it's reverted on failure, or must
each step preserve `Paris.` at every intermediate commit (the pt3* discipline so far)?

**Q3.2** — If profiling proves the gate is **unreachable on SmolLM2-360M** via submit
reduction alone, what is the fallback ruling: (a) relax the 4500ms gate, (b) accept a smaller
model / shorter prefill for the gate, (c) pursue compute-side wins (batched vocab GEMM single
readback, $M{>}1$ already landed — extend to decode?), or (d) move the gate to a faster
target (`.q42` AOT, Phase 4)?

**Q3.3** — Is the **`Mc8NormWeightArena` (next-step #1)** still wanted even if Q1.1 profiling
deprioritizes cross-layer merge? It's the prerequisite for Part 3v but only pays off after
the KV flush is gone (Q2).

---

## 4. VERIFICATION & LOGISTICS

**Q4.1** — The verify loop (`agent-tools/wasm-mc2-test.mjs` + Chrome/WebGPU) needs a real
GPU. Confirm the division of labor: **I drive `wasm-pack` builds + code edits; you run the
harness and report TTFT/coherence.** Or do you have a headless Chrome/WebGPU runner I should
invoke?

**Q4.2** — Canonical build is Git Bash `wasm-pack` with the 8 MB stack `RUSTFLAGS`
(`WASM_LLM_INFERENCE_DIAGNOSIS.md` §5); the PS wrapper aborts on wasm-pack stderr. Should I
fix `scripts/package-qualia-wasm.ps1` to not treat `[INFO]` stderr as fatal, or keep using
the raw Git Bash command?

**Q4.3** — Standard harness config to compare against the log: `WASM_ASYNC=1`,
`WASM_NAKED_PROMPT=1`, `SmolLM2-360M-Instruct-Q4_K_M.gguf`, `MC8_FUSED_PREFILL_TAIL=true`.
Confirm that's the canonical gate config, and how many runs to average (TTFT varies
~5.6–6.1s run-to-run).

---

## 5. OUT-OF-SCOPE (confirm deferred, not forgotten)

**Q5.1** — **MC7 ChatML regression** (`WASM_NAKED_PROMPT=0` → garbled 22-token prompt):
confirmed deferred until *after* the Phase 2B TTFT gate? (Endgame §3 Part 3v / planning
"next #3".)

**Q5.2** — **Phase 3 OPFS caching** (>250MB GGUF; Cache Storage `put` fails): runs in
parallel and is independent — not part of this TTFT work. Confirm.

**Q5.3** — The endgame file is committed as `qualia_wasm_llm_endgae.md.md` (double extension +
typo). Want me to rename it to `WASM_LLM_ENDGAME.md` and fix the cross-references, or leave it?

---

## 6. MY RECOMMENDED PATH (one-line, pending your answers)

Profile-first (Q1.1) → if submit-dominated, attack the KV flush via the Q2 probe (true cause,
not the assumed staging copy) → then `Mc8NormWeightArena` + cross-layer merge → only then
ChatML/OPFS. I will not edit `gguf_bridge.rs` until §1 and §2 are answered.
