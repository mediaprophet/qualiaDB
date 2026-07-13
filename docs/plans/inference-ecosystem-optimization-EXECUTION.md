# Inference Ecosystem Optimization — EXECUTION PLAN

Mechanical, step-by-step version of [`inference-ecosystem-optimization.md`](inference-ecosystem-optimization.md).
Written so any implementing agent can execute WITHOUT re-deriving architecture. Follow steps in
order. Every step ends with a Verify command and a failure branch. **Do not skip Verify. Do not
mark a step done if Verify failed** (CLAUDE.md §12).

## G. Global preamble — read once, obey always

**G1. Environment facts**
- Repo: `C:\Projects\qualia-27062026` (canonical; never a worktree/clone — CLAUDE.md §0).
- GPU: RTX A2000 12 GB. Working backend: **Vulkan**. DX12 decode HANGS (that's W4's subject) —
  never run a decode bench with `QUALIA_WGPU_BACKEND=dx12` without the timeout guard in W4.1.
- Bench model: `C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf` (Q8_0; also
  `SmolLM2-360M-Instruct-Q4_K_M.gguf` may resolve under `docs/models/`).
- Shell is PowerShell 5.1: no `&&`; env vars via `$env:NAME='value'`.

**G2. Commands (the only ones you need)**
```powershell
# fast type-check (~1.5 min warm)
cargo check -p qualia-core-db --lib
# unit tests for one module
cargo test -p qualia-core-db --lib <filter> -- --nocapture
# release bench binary (~4 min warm, ~15 min cold; harness may time out at 10 min — RERUN, it resumes)
cargo test -p qualia-core-db --release --test llm_bench_a0 --no-run
# THE decode profile (tok/s + waits/token + fence estimate)
$env:QUALIA_LLM_PROFILE_MODEL='C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf'
cargo test -p qualia-core-db --release --test llm_bench_a0 a0_decode_profile -- --nocapture
# token-identity + coherence regression (must stay green after EVERY workstream)
cargo test -p qualia-core-db --release --test llm_bench_a0 a1a_gpu_topk_matches_argmax_text -- --nocapture
cargo test -p qualia-core-db --release --test llm_bench_a0 a1c_q8_gemm_decode_coherent -- --nocapture
```

**G3. Baseline to beat (2026-07-05, this machine):** 19.89 tok/s · 50.3 ms/tok · 107 waits/tok ·
~12 ms/tok fence. If your measured "after" is worse, that is a FINDING to log, not to hide.

**G4. After every landed step (non-negotiable):**
1. Append a dated entry to `INFERENCE_OPTIMIZATION_PROGRESS_LOG.md`: step + status
   (done/partial/blocked), files touched + mechanism (1–2 sentences), REAL numbers or "not
   measured", ⚑ human asks (or "none"), next step.
2. One line in `coordination/NOTICES.md` (PROGRESS or RELEASE).
3. `git add` the touched files + commit on branch `0.0.24` with a conventional message. Do NOT push
   unless Timothy said to.

**G5. Do-not-touch (other instruments' lanes):** `specialized_libs/computational_geometry/`,
`container_10d/`, `render/spectral_oracle.rs`, `laplacian_3d.rs`, `tda.rs`, `wellfair/`,
`webizen-desktop/`, `webizen-studio/`. If one of these breaks the build: flag in NOTICES, do not fix.

**G6. Numeric-safety rules:** every fast path is a toggle + auto-fallback; legacy paths are never
deleted; anything lossy passes ΔPPL ≤ 5% (`w1_perplexity_*` pattern); anything exact passes
token-identity vs legacy. The Webizen gates (`validate_intent`/`validate_output`) and the Phase-8
Sentinel ring buffers stay exactly where they are in `inference_agent.rs`.

**G7. Key file map** (all under `crates/qualia-core-db/src/` unless noted):
| Concern | File |
|---|---|
| Decode loop (per-token) | `inference/inference_agent.rs` (~line 1062–1350) |
| Resident single-fence path (W1) | `gguf_bridge/resident_decode.rs` |
| Toggles + phase metrics | `inference/inference_bench.rs` (crate alias `crate::llm_bench`) |
| Legacy per-layer dispatch | `gguf_bridge/{forward,attention,ffn,output}.rs`, `attention/{preproject,fused_tail}.rs` |
| GEMV/GEMM + resident weights | `gguf_bridge/gemm.rs` |
| Buffers/pipelines init | `gguf_bridge/init.rs` (`ensure_gemm_buffers`, elem pipelines) |
| KV layout + load | `gguf_bridge/load.rs` (also resident logits upload, `poll_wait`) |
| Shaders | `shaders/{fused_attention,fused_tensor_contraction,wasm_elementwise,topk_reduction(in inference/topk.rs)}.wgsl` |
| Bench harness | `tests/llm_bench_a0.rs` + `inference/llm_bench` module surface |
| Forge | `wgsl_forge/` |
| AWQ capture (W10 donor) | `inference/llm_awq.rs` (via `crate::llm_awq`) |
| PPL oracle | `llm_bench::perplexity_eval_blocking`, `inference/inference_eval.rs` |

---

## W1 — Resident-token decode (single fence/token) — *code exists, finish it*

State: `resident_decode.rs` written + wired (toggle `QUALIA_LLM_RESIDENT_DECODE`, default ON,
called from the decode loop before the legacy forward). Remaining = compile-fix → differential
test → profile.

**W1.1 Compile loop.** Run `cargo check -p qualia-core-db --lib`. Fix errors ONLY in
`resident_decode.rs` / its 4 integration edits (`gguf_bridge/mod.rs`, `gguf_bridge/init.rs`,
`inference/inference_bench.rs`, `inference/inference_agent.rs`). Likely error classes + fixes:
- *Closure lifetime errors* on `norm_bind`/`mk_elem_bg`/`mk_gemm_bg`/`mk_attn_bg` (they return/borrow
  `BindingResource` tied to captured buffers): inline the closure body at call sites, or make them
  take `&wgpu::Buffer` params instead of capturing.
- *`GgufTensorInfo` not `Copy`*: replace `t.attn_q?` moves with `t.attn_q.as_ref()?` + clone fields,
  and `*index.logits_projection_info()?` with `.clone()`.
- *`LayerTensors` lifetime* (borrows `index`): keep all uses of `t` inside the per-layer loop scope.
- *wgpu resource `.clone()`*: `Device/Queue/Buffer/ComputePipeline/BindGroupLayout` are all `Clone`
  in wgpu 29; if a clone fails the name is wrong, not the concept.
- *Second struct-literal constructor* missing the new `resident_decode` field: search
  `mc8_logits_row_bytes: 0` in `init.rs` — add the field to every literal the compiler names.
Verify: check exits 0. Then `cargo test -p qualia-core-db --lib gguf_bridge -- --nocapture` — unit
tests unchanged (unit tests run the 2-layer cap ⇒ resident path is OFF there by design).

**W1.2 Differential test (a1d).** Append to `tests/llm_bench_a0.rs`:
```rust
/// a1d — resident single-fence decode must emit IDENTICAL text to the legacy per-layer path.
/// Toggles are process-global: this test must run alone (--test-threads=1) or rely on the
/// harness serializing integration tests.
#[test]
fn a1d_resident_decode_matches_legacy_text() {
    use qualia_core_db::llm_bench::{decode_with_metrics_blocking, set_resident_decode};
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[a1d] model absent — skipping");
        return;
    };
    let model = path.to_string_lossy().to_string();
    let prompt = "Once upon a time, there was a";
    set_resident_decode(false);
    let (legacy, legacy_tok) = decode_with_metrics_blocking(&model, prompt, 24).expect("legacy");
    set_resident_decode(true);
    let (resident, resident_tok) = decode_with_metrics_blocking(&model, prompt, 24).expect("resident");
    println!("[a1d] legacy   {legacy_tok:.2} tok/s: {legacy:?}");
    println!("[a1d] resident {resident_tok:.2} tok/s: {resident:?}");
    assert_eq!(legacy, resident, "resident decode diverged from legacy");
}
```
(If `set_resident_decode` isn't re-exported on `llm_bench`, add `pub use` where the other toggles
are re-exported — find `set_ternary_ffn` and mirror it.)
Run: `cargo test -p qualia-core-db --release --test llm_bench_a0 a1d -- --nocapture --test-threads=1`.
**If text differs:** set `$env:QUALIA_LLM_DEBUG_DECODE='1'`, rerun, find the first divergent token.
Two known-plausible causes, in check order: (1) Q-pass mask fields (`mask_active`/`mask_word_count`
patch in `run_resident_token`) — compare with the values `dispatch_attention_pass` computes for the
same token; (2) `write_node`… KV write position: K/V-write protos must have `proj_row_stride =
kv_dim` and `token_idx` patched. If unresolved after those: set the toggle default to OFF
(`RESIDENT_DECODE: AtomicBool::new(false)`), commit as partial with the failure documented in the
log, and continue to W2 — do NOT ship default-ON without a green a1d.

**W1.3 Profile.** Run the a0 profile (G2). Record: tok/s, waits/token (expect ~1–3, was 107),
fence est. Run a1a + a1c (must be green). A/B: rerun with `$env:QUALIA_LLM_RESIDENT_DECODE='0'`
to confirm the legacy number still reproduces (~19.9). Remove the env var after.

**W1.4 Land.** Log entry (before/after table), NOTICES line, commit
(`feat(inference): W1 resident-token single-fence decode`).

---

## W2 — Exact sampler (temperature/penalties/top-k/top-p, seeded)

**W2.1 Sampler core (pure, testable, no GPU).** New file `inference/sampler.rs` (+ `mod sampler;`
+ re-export in `inference/mod.rs`):
```rust
pub struct SamplerConfig { pub temperature: f32, pub top_k: u32, pub top_p: f32,
    pub repeat_penalty: f32, pub freq_penalty: f32, pub presence_penalty: f32,
    pub penalty_window: u32, pub seed: u64 }   // temperature <= 0.0 ⇒ greedy
pub struct SamplerState { rng: u64 /* SplitMix64 state */, pub cfg: SamplerConfig }
impl SamplerState {
    pub fn new(cfg: SamplerConfig) -> Self { /* seed rng */ }
    /// logits: full vocab; ctx: generated+prompt token ids (penalty window applies from the end).
    pub fn sample(&mut self, logits: &mut [f32], ctx: &[u32]) -> u32 { /* chain below */ }
}
```
Chain order (llama.cpp-compatible): repetition/freq/presence penalties over the last
`penalty_window` ctx tokens → temperature scale → top-k filter → top-p (nucleus) filter → softmax
→ CDF draw with SplitMix64 (implement SplitMix64 inline, ~6 lines — no new deps, wasm-safe).
`temperature <= 0` short-circuits to argmax BEFORE any penalty (greedy contract = today's behaviour).
Unit tests in-file: (t1) greedy returns argmax; (t2) same seed ⇒ same 100-draw sequence; (t3)
different seed diverges; (t4) top_k=1 equals argmax for any temperature; (t5) repeat_penalty>1
lowers a repeated token's probability (construct logits by hand); (t6) top_p=0.01 picks only the
head. Verify: `cargo test -p qualia-core-db --lib sampler`.

**W2.2 Full-logits readback seam.** The sampler needs all logits for one token.
- Legacy path: `dispatch_output_argmax_chunked` already reads chunk logits into caller scratch —
  add `pub fn dispatch_output_logits_into(&self, index, hidden, emb_dim, out: &mut [f32]) -> bool`
  in `gguf_bridge/output.rs` that reuses its chunk loop but copies each chunk into `out` instead of
  argmaxing (mirror its buffer/params handling exactly; keep the per-chunk submit).
- Resident path (W1): in `run_resident_token`, when a `want_full_logits` flag is set, additionally
  `copy_buffer_to_buffer(logits_chunk → full_logits_staging chunk offset)` per chunk inside the
  same encoder, map both stagings after the one fence. Add the `full_logits_staging` buffer
  (vocab×4 B, MAP_READ|COPY_DST) to the plan.
**W2.3 Wire into the decode loop.** In `inference_agent.rs`: sampling is active when the agent
config carries a sampler config (add `Option<SamplerConfig>` to the agent params — find where
`decode_tokens`/prompt config flows into `infer_local_model_inner`; thread it the same way). When
active: skip GPU top-1; get full logits (resident flag or legacy fn); `let next =
sampler.sample(&mut logits, &ctx);`. The Sentinel push and sieve logic consume `next` exactly as
they consume the argmax today. Greedy (`None` config) is byte-identical to today.
**W2.4 Surface.** Add sampler fields to the MCP inference tool schema in `mcp_server.rs` (find the
existing inference tool; add optional temperature/top_k/top_p/seed/penalties params, default =
greedy) and to `llm_bench` as `decode_sampled_blocking(model, prompt, n, cfg)` for tests.
**W2.5 Tests.** (a) a1a still green (greedy untouched). (b) New `a2a_sampler_deterministic`:
same seed twice ⇒ identical text; different seed ⇒ (report, don't assert). (c) De-loop demo:
decode 64 tok greedy vs sampled (T=0.8, repeat_penalty=1.15) on a repetition-prone prompt; print
unique-word ratios (report only). Land per G4.

---

## W3 — Prefill param-arena (one submit per chunk)

**W3.1** Read `dispatch_prefill_chunk` + `dispatch_prefill_layer_batch` (`forward.rs` ~line 300–372)
to confirm current per-layer submit/fence structure on native.
**W3.2** New `gguf_bridge/prefill_arena.rs` mirroring `resident_decode.rs`: a `PrefillPlan` with
batched-GEMM bind groups per layer (n_batch = chunk tokens), K/V batch writes (the attention shader
already takes `num_tokens_in_batch`/`batch_start_token_idx` — the W1 dynamic-arena slots gain a
batch variant), elem RMSNorm with `batch = n_tokens` (the shader supports `batch` via wg_id.y),
whole chunk in ONE encoder; readback = final chunk's last-token hidden only (decode needs it).
**W3.3** Toggle `QUALIA_LLM_RESIDENT_PREFILL` default ON + fallback, same pattern as W1. Prefix
cache: after arena prefill, the existing CPU-mirror copy path must still run (find `prov_hash`
handling near `inference_agent.rs:994`; it reads the GPU cache back — unchanged).
**W3.4** Verify: `a1a` green; NEW `a3a_prefill_arena_first_token_identity` (mirror a1d: toggle
off/on, decode 8 tokens from a ≥32-token prompt, assert equal). Profile: `a0_native_llm_baseline`
prints prefill tok/s + cold/warm TTFT — record before/after. Land per G4.

---

## W4 — DX12 decode deadlock

**W4.1 Guarded re-test (NEVER run unguarded).**
```powershell
$env:QUALIA_WGPU_BACKEND='dx12'
$env:QUALIA_LLM_PROFILE_MODEL='C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf'
$j = Start-Job { cargo test -p qualia-core-db --release --test llm_bench_a0 a0_decode_profile -- --nocapture }
if (-not (Wait-Job $j -Timeout 300)) { Stop-Job $j; 'DX12 STILL HANGS (>5min)' } else { Receive-Job $j }
Remove-Job $j -Force; Remove-Item Env:QUALIA_WGPU_BACKEND
```
If it completes: record the DX12 tok/s, W4 is DONE (the W1 fence collapse fixed it) — log + close.
**W4.2 Bisect (only if still hung).** Same guarded pattern on, in order: (a)
`gpu_context::tests::report_inference_backend -- --ignored` (device init only); (b) `w3_gemm_parity`
(single GEMM submit+map); (c) a new `#[ignore]` test calling `bench_empty_submit_roundtrip(64)`
(pure submit/poll — add it next to w3 in the bench file); (d) `a1a` (full decode). First one that
hangs = the primitive.
**W4.3 Known fix candidates** (apply the one matching the hanging primitive, A/B under the guard):
map_async registered BEFORE the submit that produces the data (reorder: submit → map_async → poll);
`PollType::wait_indefinitely()` → bounded `wait_for_submission_index` / poll-loop with 100 ms
`PollType::Poll` + sleep on dx12 only (branch on `crate::gpu_context` backend report); staging
buffer reused while still mapped (ensure `unmap()` on every early-return path — audit
`output.rs`/`fused_tail.rs` returns).
**W4.4** If none fixes it: write `docs/dx12-decode-deadlock-repro.md` with the minimal repro +
wgpu version + adapter info, NOTICES line, log entry marked **blocked-upstream**. That is a valid
W4 completion (root-caused + documented) — a silent skip is not.

---

## W5a — int8 KV cache (ΔPPL-gated)

**W5a.1 Layout.** In `load.rs` `ensure_kv_cache`/`KvCacheLayout`: add `quantized: bool`. Quantized
arena = i8 K/V slots + f16 scale per (layer, slot, kv_head) in a SECOND buffer `kv_scale_gpu`
(simpler binding than interleave). Keep the f32 arena allocation path untouched when off.
**W5a.2 Shader.** `fused_attention.wgsl`: where K/V are written (proj_kind 1/2) quantize
(amax/127 per head-slot → scale write + i8 store via `pack4x8snorm`-free manual packing into u32
lanes); where Q-SDPA reads K/V, dequant with the scale. Gate every change behind a uniform flag
(`params.kv_quant != 0`) so ONE shader serves both modes (the params struct has `_pad` space —
claim one field, update `AttentionGpuParams` in `gpu_params.rs` on both CPU/GPU sides).
**W5a.3 CPU parity.** `cpu_attention_pass` + `kv_cache_cpu` mirror get the same quant/dequant so
the CPU reference stays a valid oracle. Unit test: quantize→dequantize round-trip error bound;
GPU-vs-CPU attention parity test with kv_quant on (mirror `w3_gemm_parity` structure).
**W5a.4 Toggle** `QUALIA_LLM_KV_INT8` — **default OFF until the gate passes.**
**W5a.5 Gate + measure.** `w1_perplexity_smollm2_q8_vs_q4` pattern: PPL with kv_quant off vs on
(same model!) — ΔPPL must be ≤ 5% (expect ≪1%); a1a-style coherence with the toggle on; decode
tok/s A/B; KV bytes logged (expect ~80→~44 MiB @ctx1024). Pass ⇒ flip default ON. Land per G4,
W10 note: also land the KV-capture hook here (W10.2).

---

## W6a — Prompt-lookup speculative decode (exact-output)

**W6a.1 Proposer.** New `inference/prompt_lookup.rs`: `fn propose(ctx: &[u32], max_k: usize) ->
([u32; 8], usize)` — find the longest suffix of `ctx` (n-gram, try n=3 then 2) that recurs earlier
in `ctx`; return up to `max_k` (≤8) tokens that followed that earlier occurrence. Pure, unit-tested
(repetition case proposes; random case proposes 0).
**W6a.2 Verify step.** Speculation only when greedy + resident path active. Loop change in
`inference_agent.rs`: after sampling token `t0` normally, call `propose`; if k ≥ 2, run the drafted
tokens through the BATCHED forward (the W3 arena path with n_tokens=k, batch_start = current
token_idx) capturing each position's argmax via the batched output projection (`n_batch=k` on the
logits GEMM chunks — shader already supports `n_batch`; verify against `output.rs` params); accept
the longest prefix where `argmax[i] == draft[i]`; append accepted tokens (each goes through the
SAME Sentinel push + sieve/EOS checks — no governance bypass); the KV slots for rejected positions
are simply overwritten by the next real token (monotonic token_idx — confirm no stale-read window
by asserting accepted_len ≤ drafted before ctx advance).
**W6a.3 Exact-output test** `a6a_prompt_lookup_identical`: toggle `QUALIA_LLM_PROMPT_LOOKUP`
(default OFF until this passes) off/on, 64-token decode on a quote-heavy prompt, assert identical
text; report tok/s both ways (expect win on repetitive text, ~neutral elsewhere). Pass ⇒ default ON
for greedy. Land per G4.

---

## W7 — Thermal/power telemetry + governor

**W7.1 Dep (feature-gated).** `qualia-core-db/Cargo.toml`: `nvml-wrapper = { version = "0.11", optional = true }`,
feature `thermal-telemetry = ["dep:nvml-wrapper"]` (native-only; NOT in default features until
verified). Check `DEPENDENCY_MODERNIZATION.md` conventions first.
**W7.2 Module.** New `inference/thermal_telemetry.rs`: `struct GpuTelemetry { temp_c, power_w,
power_limit_w, sm_clock_mhz, throttle_reasons: u64, ts }`; `fn sample() -> Option<GpuTelemetry>`
(NVML init once, cached); background sampler thread at 1 Hz started by the orchestrator, writing a
process-global `ArcSwap`-style latest (use `Mutex<Option<GpuTelemetry>>` — no new deps). No NVML ⇒
`None` ⇒ everything degrades silently.
**W7.3 Governor wiring.** Find `ThermalGovernor` in `orchestrator.rs`: replace its synthetic input
with the telemetry latest (Nominal <70 °C, Warm 70–83, Hot >83 — read actual thresholds from the
governor if it has them). Pacing: in the decode loop, on Hot the governor may inject an inter-token
`std::thread::yield_now()` + reduced speculation (W6a k→0); NEVER silently changes clocks/TDP —
log + expose via the existing diagnostics/MCP surface (`mcp_server.rs` telemetry tool if present,
else a `llm_bench::thermal_snapshot()` accessor).
**W7.4 Verify.** Manual: print `sample()` vs `nvidia-smi -q -d TEMPERATURE,POWER,CLOCK` — values
within sensor tolerance. Sustained: `#[ignore]` test decoding 256 tokens × 20 rounds printing
telemetry every round — governor states + pacing visible in output. A2000 on mains likely never
leaves Nominal: REPORT that honestly (the mechanism is verified, the transition needs a constrained
device). ⚑ Timothy: opt-in admin TDP-cap helper — build only if he says yes. Land per G4.

---

## W8 — Coopmat selection seam (gated)

**W8.1** Add feature `coopmat-decode` (default OFF) + a `DecodeGemvBackend` enum
(`CoopGemv | Naive | CoopMat`) resolved once at plan build in `resident_decode.rs` (today returns
CoopGemv/Naive exactly as now; CoopMat arm returns Ineligible unless the feature + runtime support
probe pass). **W8.2** Do NOT vendor/soft-fork wgpu on any shipping path; the soft-fork stays
validation-only per `docs/WGPU_UPSTREAM_TRACKING.md`. **W8.3** When a wgpu release ships #9741:
bump wgpu per §13, implement the CoopMat arm against the certified kernels, gate = a1d-style
token-identity + tok/s A/B. Until then W8 closes as "seam landed, integration gated" (log says
exactly that).

---

## W9 — Harness path-visibility + smoke gate (continuous)

**W9.1 Path counters.** In `inference_bench.rs`, mirror `TOPK_HITS`: add `RESIDENT_HITS`,
`RESIDENT_FALLBACKS`, `SAMPLER_TOKENS`, `SPEC_ACCEPTED`, `SPEC_REJECTED` + `record_*`/accessors;
increment at the obvious sites (W1/W2/W6a code). `a0_decode_profile` prints them all + which path
dominated. **W9.2 Toggle table.** Add a "Runtime toggles" section to
`docs/manuals/wgsl-forge.md`-adjacent (or a new `docs/manuals/inference-tuning.md`): every
`QUALIA_LLM_*` env var, default, what it A/Bs. **W9.3 Smoke gate.** Document (in the same manual)
the pre-push set: `cargo test -p qualia-core-db --lib sampler prompt_lookup gguf_bridge` +
release `a1a a1d` — and keep it green.

---

## W10 — Forge upgrade: calibration pipeline

*(Decision: upgrade the EXISTING forge — no new forge. Stages 4–5 already exist as forge muscles.)*

**W10.1 Module skeleton.** `wgsl_forge/calibration/mod.rs` + submodules `corpus.rs`, `capture.rs`,
`learn.rs`, `certify.rs`, `package.rs` (§11: split-as-you-go). Entry point:
`pub fn run_calibration(job: CalibrationJob) -> Result<CalibrationReport, CalibrationError>` where
`CalibrationJob { model_path, artifact: ArtifactKind (AwqScales | KvInt8Scales | KvDictionary),
corpus: CorpusSpec, gate: GateSpec }`. This sits beside certify-kernel/transcode as the forge's
third entry point.
**W10.2 Capture hooks (lands WITH W5a).** Mirror `llm_awq`: `inference/llm_kv_capture.rs` with
`enable(n_layer, kv_dim)`, `record_kv(layer, k: &[f32], v: &[f32])` called from the K/V write site
(CPU-side capture: the preproject path has the projected K/V on GPU only — capture via the CPU
attention reference pass over the corpus instead; that is honest and sufficient for calibration),
`snapshot()`. Document the CPU-reference-capture caveat in the module header.
**W10.3 Corpus (Ollama, forge-side ONLY).** `corpus.rs`: `CorpusSpec::{Files(Vec<PathBuf>),
OllamaSynth { model: String, prompts: Vec<String>, n_per_prompt: u32 }}`. Ollama via
`std::process::Command` → `ollama run <model> <prompt>` (NO new HTTP dep; NO qualia-client-core
reqwest — that's Gemini's lane). If `ollama` is not on PATH ⇒ clear error, Files-only still works.
Cache synth output under `benchmarks/calibration_corpus/` (gitignored) with a manifest hash.
**W10.4 Learn.** `learn.rs`: AwqScales = call existing `llm_awq` fold; KvInt8Scales = percentile
(99.9) amax fit per (layer, head) from W10.2 snapshots; KvDictionary = k-SVD + OMP (W5b — implement
ONLY when W5b opens; until then the enum arm returns `Err(NotYetImplemented)` — visible, not fake).
**W10.5 Certify.** `certify.rs`: run `perplexity_eval_blocking` ref-vs-candidate → ΔPPL vs
`GateSpec` (default 5%); optional cross-check: `ollama run` the same passages and compare PPL-rank
sanity (report only). **W10.6 Package.** `package.rs`: artifact bytes + provenance (corpus manifest
hash, engine version from `CARGO_PKG_VERSION`, gate numbers, date) into a p64 sidecar section via
the existing `p64_weight` section-writing surface; engine loads only artifacts whose provenance
parses. **W10.7 Tests:** corpus Files round-trip; capture snapshot non-empty on a 3-passage run;
int8-scale learn produces finite scales; certify gate rejects a deliberately corrupted artifact;
package round-trips. Land per G4.

---

## Order of execution (do not reorder)

`W1.1→W1.4` → `W2.1→W2.5` → `W3.1→W3.4` → `W4.1→(W4.2–W4.4 as needed)` → `W5a.1→W5a.5` (+`W10.2`)
→ `W6a.1→W6a.3` → `W7.1→W7.4` → `W10.1,W10.3–W10.7` → `W8.1–W8.2` → keep `W9` current throughout.
W5b / W6b / W8.3 open only when their gates open (corpus curation / draft-model decision / wgpu release).

## Standing ⚑ asks for Timothy (unchanged, none blocking)
1. W5b eval-corpus curation (when W5b starts). 2. W6b: ship a 135M draft model? 3. W7: opt-in
admin TDP-cap helper — yes/no. 4. W8: no-soft-fork stance holds until wgpu releases #9741 — confirm.
