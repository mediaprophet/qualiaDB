# ADR 0010: Prompt-lookup speculative decode is default-ON, with a runtime mode switch

## Status
Accepted (2026-07-05, 0.0.24) — directed by Timothy. Ships the W6a prompt-lookup speculative decoder
**on by default**, accepting rare benign near-tie divergence from single-token decode, with an explicit
mode switch so either mode can be selected at launch or at runtime (incl. from the desktop UI).

## Context
W6a added prompt-lookup (LLMA / n-gram) speculative decoding to the native decode loop
(`inference_agent.rs`), backed by a batched **verify forward** (`gguf_bridge/verify_arena.rs`). Per
step, when no sieve/sampler/sparse-route is active, it drafts the next few tokens from an earlier
recurrence in the context, verifies the whole draft in **one** batched forward, and emits the longest
greedily-agreeing prefix plus the model's own correction token. On this compute-bound class of GPU it
is the first **real** steady-state decode-tok/s win in the optimization programme (fence-latency wins
like W1/W3 are latent here): measured **~3–12× decode tok/s on repetitive / quoting / structured / code
text**; on novel prose the proposer drafts little and costs ≈nothing.

The open question was **transparency**: is speculative output identical to ordinary single-token
decode? Verify selects with a full-logit **CPU argmax** over its batched forward; the default decode
path (`resident_decode`) selects with a **GPU top-1** block reduction over its single-token forward. A
diagnostic (`a6` extended with a resident-path reference, `spec_verify_probe_blocking`) established:

- **verify == resident == legacy at every position across the B=1..6 sweep** — they agree in the
  overwhelming majority of positions.
- They diverge **only at genuine near-ties** — positions where the model is ~50/50 between two
  equally-valid tokens (e.g. "continue the list" vs "stop and summarise"), where a ULP-level difference
  between the batched and single-token forwards flips the pick. This is exactly the pre-existing **a1a**
  phenomenon (CPU-argmax vs GPU-top1 on a near-tie), already accepted as benign in the codebase.

Achieving *bit-perfect* transparency would require making verify's batched forward bit-identical to the
resident single-token forward at the ULP level — unifying two different orchestrations' float-reduction
order — a large, deep change for effectively no user-visible benefit (the only positions that would
change are ones where the model itself is indifferent).

The exact-output gate `a6a` confirms the wiring is correct: against a **consistent** selection method
(CPU argmax on both sides), speculative decode is **bit-identical** to greedy, with 48/48 draft tokens
accepted on a repetitive prompt.

## Decision
**Ship prompt-lookup speculative decode ON by default**, and expose an explicit **mode switch**.

1. `QUALIA_LLM_SPEC_DECODE` defaults **ON** (`inference_bench.rs`). Speculative decode runs whenever no
   sieve, sampler, or sparse-attention route is active and the full model runs. Ineligible steps fall
   back to ordinary single-token decode unchanged.
2. **We accept the rare, benign near-tie divergence** from single-token decode. It is confined to
   positions where the model is genuinely ambivalent (both tokens equally valid) and is the same class
   as the already-tolerated a1a near-tie. We do **not** pursue bit-perfect transparency (poor return).
3. **Mode switch — three equivalent controls**, so either mode is always selectable, including from the
   desktop UI / host:
   - **Launch**: env `QUALIA_LLM_SPEC_DECODE=0` (single-token) / `=1` (speculative). The env var, when
     set, overrides the runtime flag in both directions. Read by the native engine (desktop + daemon).
   - **Runtime**: `qualia_core_db::llm_bench::set_spec_decode(bool)` — the host/UI calls this to flip
     modes live between inferences.
   - **Read-back**: `spec_decode_enabled()` returns the effective mode (for reflecting state in a UI).
   The webizen-desktop UI wires a control to `set_spec_decode` / the env var (that wiring lives in the
   `webizen-browser` repo). Speculative decode is **native-only** — the decode-loop branch is
   `#[cfg(not(target_arch = "wasm32"))]`, so the wasm build always runs single-token decode and the
   switch is a no-op there.

## Consequences
- **Positive:** every user gets the large decode-tok/s win on repetitive/structured/code workloads by
  default, with ≈zero cost on novel text. Directly serves the original "improve tok/s" objective.
- **Positive:** an explicit, documented mode switch (env + runtime + read-back) means anyone who wants
  strictly identical-to-single-token output can select it per launch or per request.
- **Neutral / caveat:** speculative output can differ from single-token output **only at rare benign
  near-ties** — recorded here as an accepted, bounded behaviour, not a defect. Same class as a1a.
- **Negative / watch:** a decode step whose drafts are proposed but **rejected** pays one batched
  forward for a single token — a slight local loss on text with recurrence-but-no-follow-through. Net
  positive across realistic workloads; the mode switch is the escape hatch if a workload regresses.
- **Test impact:** with the default ON, path-isolating differential tests (`a1d` resident-vs-legacy,
  `a1c`, `a3a`, `w5a`) pin `set_spec_decode(false)` so they continue to measure their target path. The
  `lib` build is unaffected (test-only), so other lanes are not impacted.

## References
- W6a implementation: `gguf_bridge/verify_arena.rs` (batched verify forward), `inference/prompt_lookup.rs`
  (n-gram proposer), the decode-loop branch in `inference/inference_agent.rs`, toggle + counters in
  `inference/inference_bench.rs`.
- Gates: `a6_primitive` (verify == sequential per-token, B=1..6, + verify == resident diagnostic),
  `a6a` (spec == greedy bit-identical against a consistent selection).
- Design: `docs/plans/inference-W6a-speculative-verify.md`; running record:
  `INFERENCE_OPTIMIZATION_PROGRESS_LOG.md`.
- Related near-tie phenomenon: task #14 / a1a (CPU-argmax vs GPU-top1); this ADR accepts the same
  benign class for speculative decode.
- Commits: `96e78984`, `8b455546` (primitive), `74a6447e` (wiring), `08a915b6` (a6a), `7092deab`
  (verify-vs-resident diagnostic).
